use crate::annotation;
use crate::cache::{page_cache_key, thumb_cache_key, tile_cache_key, TileCache};
use crate::render::{crop_render_output, effective_scale};
use crate::error::{FastPdfError, Result};
use crate::form;
use crate::info;
use crate::outline;
use crate::page;
use crate::render;
use crate::search;
use crate::security;
use crate::text;
use crate::types::*;
use lopdf::Document;
use lopdf::ObjectId;
use std::collections::BTreeMap;

pub struct FastPdfDocument {
    pub(crate) doc: Document,
    /// Original file bytes. Page pixels are produced by hayro from this buffer.
    source: Vec<u8>,
    /// User password for hayro when the file is still encrypted on disk.
    password: Option<String>,
    hayro: Option<render::HayroSession>,
    pub(crate) pages: BTreeMap<u32, ObjectId>,
    labels: BTreeMap<u32, String>,
    cache: TileCache,
    span_cache: BTreeMap<u32, Vec<TextSpan>>,
    plain_cache: BTreeMap<u32, PagePlain>,
    line_cache: BTreeMap<u32, Vec<TextLine>>,
}

impl FastPdfDocument {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let doc = Document::load_mem(bytes)?;
        Self::from_document(doc, bytes.to_vec())
    }

    pub fn from_bytes_with_password(bytes: &[u8], password: &str) -> Result<Self> {
        let mut doc = Document::load_mem(bytes)?;
        doc.decrypt(password.as_bytes())
            .map_err(|_| FastPdfError::InvalidPassword)?;
        let mut s = Self::from_document(doc, bytes.to_vec())?;
        s.password = Some(password.to_string());
        Ok(s)
    }

    fn ensure_hayro(&mut self) -> Result<()> {
        if self.hayro.is_none() {
            self.hayro = Some(render::HayroSession::open(
                &self.source,
                self.password.as_deref(),
            )?);
        }
        Ok(())
    }

    fn invalidate_render(&mut self) {
        self.hayro = None;
        self.cache.clear();
        self.span_cache.clear();
        self.plain_cache.clear();
        self.line_cache.clear();
    }

    pub fn page_view_size(&mut self, index: u32) -> Result<(f32, f32)> {
        self.ensure_hayro()?;
        self.hayro
            .as_ref()
            .unwrap()
            .view_dimensions(index as usize)
    }

    fn normalize_pages(pages: BTreeMap<u32, ObjectId>) -> BTreeMap<u32, ObjectId> {
        pages
            .into_iter()
            .map(|(k, v)| (k.saturating_sub(1), v))
            .collect()
    }

    fn from_document(doc: Document, source: Vec<u8>) -> Result<Self> {
        let pages = Self::normalize_pages(doc.get_pages());
        if pages.is_empty() {
            return Err(FastPdfError::CorruptPdf("no pages".into()));
        }
        let labels = page::resolve_page_labels(&doc, &pages);
        Ok(Self {
            doc,
            source,
            password: None,
            hayro: None,
            pages,
            labels,
            cache: TileCache::new(32),
            span_cache: BTreeMap::new(),
            plain_cache: BTreeMap::new(),
            line_cache: BTreeMap::new(),
        })
    }

    pub fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    pub fn page_count(&self) -> u32 {
        page::page_count(&self.pages)
    }

    pub fn info(&self) -> DocumentInfo {
        info::extract_info(&self.doc)
    }

    pub fn outline(&self) -> Result<Vec<OutlineItem>> {
        outline::build_outline(&self.doc, &self.pages)
    }

    pub fn permissions(&self) -> PermissionsDto {
        security::extract_permissions(&self.doc)
    }

    pub fn is_encrypted(&self) -> bool {
        security::is_encrypted(&self.doc)
    }

    pub fn page_info(&self, index: u32) -> Result<PageInfo> {
        let mut p = page::page_info(&self.doc, &self.pages, index)?;
        p.label = self.labels.get(&index).cloned();
        Ok(p)
    }

    pub fn render_page(&mut self, index: u32, scale: f32) -> Result<RenderOutput> {
        let scale = effective_scale(scale);
        let key = page_cache_key(index, scale);
        if let Some(hit) = self.cache.get(&key) {
            return Ok(hit);
        }
        self.ensure_hayro()?;
        let out = self
            .hayro
            .as_mut()
            .ok_or_else(|| FastPdfError::CorruptPdf("hayro session missing".into()))?
            .render_page(index as usize, scale)?;
        self.cache.try_insert(key, out.clone());
        Ok(out)
    }

    pub fn render_thumbnail(&mut self, index: u32, max_side: f32) -> Result<RenderOutput> {
        let key = thumb_cache_key(index, max_side);
        if let Some(hit) = self.cache.get(&key) {
            return Ok(hit);
        }
        self.ensure_hayro()?;
        let out = self
            .hayro
            .as_mut()
            .ok_or_else(|| FastPdfError::CorruptPdf("hayro session missing".into()))?
            .render_thumbnail(index as usize, max_side)?;
        self.cache.try_insert(key, out.clone());
        Ok(out)
    }

    pub fn render_tile(
        &mut self,
        index: u32,
        scale: f32,
        tile_x: f32,
        tile_y: f32,
        tile_w: f32,
        tile_h: f32,
    ) -> Result<RenderOutput> {
        let scale = effective_scale(scale);
        let tile_key = tile_cache_key(index, scale, tile_x, tile_y, tile_w, tile_h);
        if let Some(hit) = self.cache.get(&tile_key) {
            return Ok(hit);
        }

        let page_key = page_cache_key(index, scale);
        let full = if let Some(cached) = self.cache.peek(&page_key) {
            cached
        } else {
            self.render_page(index, scale)?
        };

        let out = crop_render_output(&full, tile_x, tile_y, tile_w, tile_h, scale)?;
        self.cache.try_insert(tile_key, out.clone());
        Ok(out)
    }

    pub fn set_max_cache_entries(&mut self, n: usize) {
        self.cache.set_max_entries(n);
    }

    pub fn set_max_cache_bytes(&mut self, nbytes: usize) {
        self.cache.set_max_bytes(nbytes);
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn cache_byte_usage(&self) -> (usize, usize) {
        (self.cache.current_bytes(), self.cache.max_bytes())
    }

    fn ensure_page_layout(&mut self, index: u32) -> Result<(&[TextSpan], &PagePlain)> {
        if !self.span_cache.contains_key(&index) {
            self.ensure_hayro()?;
            let mut spans = if let Some(session) = &self.hayro {
                let spans = session.text_spans(index as usize)?;
                if !spans.is_empty() {
                    spans
                } else {
                    let id = page::page_id(&self.pages, index)?;
                    text::extract_text_spans(&self.doc, id)?
                }
            } else {
                let id = page::page_id(&self.pages, index)?;
                text::extract_text_spans(&self.doc, id)?
            };
            if spans.is_empty() {
                let id = page::page_id(&self.pages, index)?;
                spans = text::extract_text_spans(&self.doc, id)?;
            }
            if spans.is_empty() {
                let id = page::page_id(&self.pages, index)?;
                let fallback = text::extract_page_text(&self.doc, id)?;
                self.plain_cache.insert(
                    index,
                    PagePlain {
                        text: fallback,
                        char_map: vec![],
                    },
                );
                self.span_cache.insert(index, spans);
            } else {
                let layout = crate::text_layout::build_page_layout(&mut spans);
                self.plain_cache.insert(index, layout.plain);
                self.line_cache.insert(index, layout.lines);
                self.span_cache.insert(index, spans);
            }
        }
        let spans = self
            .span_cache
            .get(&index)
            .ok_or(FastPdfError::PageOutOfRange(index))?;
        let plain = self
            .plain_cache
            .get(&index)
            .ok_or(FastPdfError::PageOutOfRange(index))?;
        Ok((spans.as_slice(), plain))
    }

    pub fn text_spans(&mut self, index: u32) -> Result<Vec<TextSpan>> {
        let (spans, _) = self.ensure_page_layout(index)?;
        Ok(spans.to_vec())
    }

    pub fn page_text(&mut self, index: u32) -> Result<String> {
        let (_, plain) = self.ensure_page_layout(index)?;
        Ok(plain.text.clone())
    }

    pub fn text_lines(&mut self, index: u32) -> Result<Vec<TextLine>> {
        self.ensure_page_layout(index)?;
        self.line_cache
            .get(&index)
            .cloned()
            .ok_or(FastPdfError::PageOutOfRange(index))
    }

    pub fn search_page(&mut self, page_index: u32, query: &str) -> Result<Vec<SearchMatch>> {
        if query.is_empty() {
            return Ok(vec![]);
        }
        let (spans, plain) = self.ensure_page_layout(page_index)?;
        Ok(search::search_page_plain(
            page_index,
            plain,
            spans,
            query,
        ))
    }

    pub fn search(&mut self, query: &str) -> Result<Vec<SearchMatch>> {
        if query.is_empty() {
            return Ok(vec![]);
        }
        let indices: Vec<u32> = self.pages.keys().copied().collect();
        let mut hits = Vec::new();
        for idx in indices {
            hits.extend(self.search_page(idx, query)?);
        }
        Ok(hits)
    }

    pub fn annotations(&self, index: u32) -> Result<Vec<AnnotationDto>> {
        annotation::list_annotations(&self.doc, &self.pages, index)
    }

    pub fn hit_test_annotation(&self, index: u32, x: f32, y: f32) -> Result<Vec<AnnotationDto>> {
        annotation::hit_test(&self.doc, &self.pages, index, x, y)
    }

    pub fn links_on_page(&self, page_index: u32) -> Result<Vec<LinkDto>> {
        let annots = annotation::list_annotations(&self.doc, &self.pages, page_index)?;
        Ok(annots
            .into_iter()
            .filter(|a| a.subtype == "Link")
            .map(|a| LinkDto {
                rect: a.rect,
                uri: a.url.clone(),
                page_index: a.page_index,
            })
            .collect())
    }

    pub fn form_fields(&self) -> Vec<FormFieldDto> {
        form::list_form_fields(&self.doc)
    }

    pub fn set_form_field(&mut self, name: &str, value: &str) -> bool {
        form::set_field_value(&mut self.doc, name, value)
    }

    pub fn add_highlight(
        &mut self,
        page_index: u32,
        quad: Rect,
        contents: Option<&str>,
    ) -> Result<u32> {
        annotation::add_highlight(&mut self.doc, &self.pages, page_index, quad, contents)?;
        self.plain_cache.remove(&page_index);
        self.span_cache.remove(&page_index);
        self.line_cache.remove(&page_index);
        self.invalidate_render();
        Ok(0)
    }

    pub fn save_to_bytes(&mut self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.doc
            .save_to(&mut buf)
            .map_err(|e| FastPdfError::Lopdf(e.to_string()))?;
        self.source = buf.clone();
        self.invalidate_render();
        Ok(buf)
    }

    pub fn append_bytes(&mut self, chunk: &[u8]) -> Result<()> {
        if chunk.is_empty() {
            return Ok(());
        }
        let mut merged = self.save_to_bytes()?; // mut self
        merged.extend_from_slice(chunk);
        let doc = Document::load_mem(&merged)?;
        self.source = merged;
        self.doc = doc;
        self.pages = Self::normalize_pages(self.doc.get_pages());
        self.labels = page::resolve_page_labels(&self.doc, &self.pages);
        self.invalidate_render();
        Ok(())
    }
}
