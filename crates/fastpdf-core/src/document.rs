use crate::annotation;
use crate::cache::TileCache;
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
    pages_text: Option<Vec<(u32, String)>>,
    span_cache: BTreeMap<u32, Vec<TextSpan>>,
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
        self.pages_text = None;
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
            pages_text: None,
            span_cache: BTreeMap::new(),
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
        let key = format!("page-{index}-{scale}");
        if let Some(hit) = self.cache.get(&key) {
            return Ok(hit.clone());
        }
        self.ensure_hayro()?;
        let out = self
            .hayro
            .as_ref()
            .unwrap()
            .render_page(index as usize, scale)?;
        self.cache.insert(key, out.clone());
        Ok(out)
    }

    pub fn render_thumbnail(&mut self, index: u32, max_side: f32) -> Result<RenderOutput> {
        let key = format!("thumb-{index}-{max_side}");
        if let Some(hit) = self.cache.get(&key) {
            return Ok(hit.clone());
        }
        self.ensure_hayro()?;
        let out = self
            .hayro
            .as_ref()
            .unwrap()
            .render_thumbnail(index as usize, max_side)?;
        self.cache.insert(key, out.clone());
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
        let key = format!("tile-{index}-{scale}-{tile_x}-{tile_y}-{tile_w}-{tile_h}");
        if let Some(hit) = self.cache.get(&key) {
            return Ok(hit.clone());
        }
        self.ensure_hayro()?;
        let out = self.hayro.as_ref().unwrap().render_tile(
            index as usize,
            scale,
            tile_x,
            tile_y,
            tile_w,
            tile_h,
        )?;
        self.cache.insert(key, out.clone());
        Ok(out)
    }

    pub fn set_max_cache_entries(&mut self, n: usize) {
        self.cache.set_max_entries(n);
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    pub fn text_spans(&mut self, index: u32) -> Result<Vec<TextSpan>> {
        if let Some(cached) = self.span_cache.get(&index) {
            return Ok(cached.clone());
        }
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
        crate::text_layout::annotate_layout(&mut spans);
        self.span_cache.insert(index, spans.clone());
        Ok(spans)
    }

    fn lopdf_page_text(&self, index: u32) -> Result<String> {
        let id = page::page_id(&self.pages, index)?;
        text::extract_page_text(&self.doc, id)
    }

    pub fn page_text(&mut self, index: u32) -> Result<String> {
        let spans = self.text_spans(index)?;
        if spans.is_empty() {
            return self.lopdf_page_text(index);
        }
        Ok(crate::text_layout::plain_text(&spans))
    }

    fn ensure_pages_text(&mut self) -> Result<&[(u32, String)]> {
        if self.pages_text.is_none() {
            let mut out = Vec::with_capacity(self.pages.len());
            let indices: Vec<u32> = self.pages.keys().copied().collect();
            for idx in indices {
                out.push((idx, self.page_text(idx)?));
            }
            self.pages_text = Some(out);
        }
        Ok(self.pages_text.as_ref().unwrap())
    }

    pub fn search(&mut self, query: &str) -> Result<Vec<SearchMatch>> {
        let pages_text: Vec<(u32, String)> = self.ensure_pages_text()?.to_vec();
        let mut hits = search::search_pages(&pages_text, query);
        if query.is_empty() {
            return Ok(hits);
        }
        let pages: std::collections::BTreeSet<u32> = hits.iter().map(|h| h.page_index).collect();
        let mut span_by_page = BTreeMap::new();
        for p in pages {
            if let Ok(spans) = self.text_spans(p) {
                span_by_page.insert(p, spans);
            }
        }
        for hit in &mut hits {
            if let Some(spans) = span_by_page.get(&hit.page_index) {
                hit.rects = search::rects_for_query(spans, query);
            }
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
        self.pages_text = None;
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
        self.pages_text = None;
        self.invalidate_render();
        Ok(())
    }
}
