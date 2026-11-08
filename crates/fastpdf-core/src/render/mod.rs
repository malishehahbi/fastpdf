//! Page rasterization via [hayro](https://docs.rs/hayro) (PdfCraft-style).

mod text_extract;

use crate::error::{FastPdfError, Result};
use crate::types::{RenderOutput, TextSpan};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::vello_cpu::Pixmap;
use hayro::{RenderCache, RenderSettings, render};

/// Parsed hayro document; reused across `render_page` calls (PdfCraft `PageRenderer` pattern).
pub struct HayroSession {
    pdf: Pdf,
    interp: InterpreterSettings,
}

impl HayroSession {
    pub fn open(source: &[u8], password: Option<&str>) -> Result<Self> {
        let pdf = open_pdf(source, password)?;
        Ok(Self {
            pdf,
            interp: InterpreterSettings::default(),
        })
    }

    pub fn reload(&mut self, source: &[u8], password: Option<&str>) -> Result<()> {
        self.pdf = open_pdf(source, password)?;
        Ok(())
    }

    pub fn render_page(&self, page_index: usize, scale: f32) -> Result<RenderOutput> {
        let pages = self.pdf.pages();
        let page = pages
            .get(page_index)
            .ok_or(FastPdfError::PageOutOfRange(page_index as u32))?;
        let scale = effective_scale(scale);
        let settings = RenderSettings {
            x_scale: scale,
            y_scale: scale,
            bg_color: WHITE,
            ..Default::default()
        };
        let cache = RenderCache::new();
        let pixmap = render(&page, &cache, &self.interp, &settings);
        pixmap_to_output(&pixmap)
    }

    pub fn render_thumbnail(&self, page_index: usize, max_side: f32) -> Result<RenderOutput> {
        let pages = self.pdf.pages();
        let page = pages
            .get(page_index)
            .ok_or(FastPdfError::PageOutOfRange(page_index as u32))?;
        let (w, h) = page.render_dimensions();
        let scale = effective_scale(max_side / w.max(h).max(1.0));
        self.render_page(page_index, scale)
    }

    pub fn render_tile(
        &self,
        page_index: usize,
        scale: f32,
        tile_x: f32,
        tile_y: f32,
        tile_w: f32,
        tile_h: f32,
    ) -> Result<RenderOutput> {
        let full = self.render_page(page_index, scale)?;
        crop_pixels(
            &full.pixels,
            full.width,
            full.height,
            tile_x,
            tile_y,
            tile_w,
            tile_h,
            scale,
        )
    }

    pub fn text_spans(&self, page_index: usize) -> Result<Vec<TextSpan>> {
        let pages = self.pdf.pages();
        let page = pages
            .get(page_index)
            .ok_or(FastPdfError::PageOutOfRange(page_index as u32))?;
        Ok(text_extract::extract_page_spans(&page, &self.interp))
    }

    pub fn view_dimensions(&self, page_index: usize) -> Result<(f32, f32)> {
        let pages = self.pdf.pages();
        let page = pages
            .get(page_index)
            .ok_or(FastPdfError::PageOutOfRange(page_index as u32))?;
        Ok(page.render_dimensions())
    }
}

pub fn render_bytes(source: &[u8], page_index: usize, scale: f32, password: Option<&str>) -> Result<RenderOutput> {
    let session = HayroSession::open(source, password)?;
    session.render_page(page_index, scale)
}

fn open_pdf(bytes: &[u8], password: Option<&str>) -> Result<Pdf> {
    if let Some(pwd) = password.filter(|p| !p.is_empty()) {
        return Pdf::new_with_password(bytes.to_vec(), pwd)
            .map_err(|_| FastPdfError::InvalidPassword);
    }
    match Pdf::new(bytes.to_vec()) {
        Ok(p) => Ok(p),
        Err(_) => Pdf::new_with_password(bytes.to_vec(), "")
            .map_err(|_| FastPdfError::InvalidPassword),
    }
}

fn effective_scale(scale: f32) -> f32 {
    if scale.is_finite() && scale > 0.05 {
        scale
    } else {
        1.0
    }
}

fn pixmap_to_output(pixmap: &Pixmap) -> Result<RenderOutput> {
    let width = pixmap.width() as u32;
    let height = pixmap.height() as u32;
    let mut pixels = pixmap.data_as_u8_slice().to_vec();
    unpremultiply_rgba(&mut pixels);
    Ok(RenderOutput {
        width,
        height,
        pixels,
    })
}

fn unpremultiply_rgba(pixels: &mut [u8]) {
    for px in pixels.chunks_exact_mut(4) {
        let a = px[3];
        if a == 0 || a == 255 {
            continue;
        }
        let af = a as u16;
        px[0] = ((px[0] as u16 * 255) / af).min(255) as u8;
        px[1] = ((px[1] as u16 * 255) / af).min(255) as u8;
        px[2] = ((px[2] as u16 * 255) / af).min(255) as u8;
    }
}

/// Crop a region in PDF points (bottom-left origin) from a full-page bitmap (top-left pixels).
fn crop_pixels(
    full: &[u8],
    full_w: u32,
    full_h: u32,
    tile_x: f32,
    tile_y: f32,
    tile_w: f32,
    tile_h: f32,
    scale: f32,
) -> Result<RenderOutput> {
    let x0 = (tile_x * scale).floor().max(0.0) as u32;
    let y0_pdf = tile_y;
    let w = (tile_w * scale).ceil().max(1.0) as u32;
    let h = (tile_h * scale).ceil().max(1.0) as u32;
    let page_h_pt = full_h as f32 / scale;
    let y0 = ((page_h_pt - y0_pdf - tile_h) * scale).floor().max(0.0) as u32;

    if x0 >= full_w || y0 >= full_h {
        return Ok(RenderOutput {
            width: w,
            height: h,
            pixels: vec![255; (w * h * 4) as usize],
        });
    }

    let cw = w.min(full_w.saturating_sub(x0));
    let ch = h.min(full_h.saturating_sub(y0));
    let mut out = vec![255u8; (w * h * 4) as usize];
    for row in 0..ch {
        let src_y = y0 + row;
        let src_start = (src_y * full_w + x0) as usize * 4;
        let dst_start = row as usize * w as usize * 4;
        let len = cw as usize * 4;
        if src_start + len <= full.len() && dst_start + len <= out.len() {
            out[dst_start..dst_start + len].copy_from_slice(&full[src_start..src_start + len]);
        }
    }
    Ok(RenderOutput {
        width: w,
        height: h,
        pixels: out,
    })
}
