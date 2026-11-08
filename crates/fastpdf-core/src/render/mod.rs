//! Page rasterization via [hayro](https://docs.rs/hayro) — the same pure-Rust engine PdfCraft uses
//! while its own renderer is unfinished.

use crate::error::{FastPdfError, Result};
use crate::types::RenderOutput;
use hayro::hayro_interpret::InterpreterSettings;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{RenderCache, RenderSettings, render};

pub fn render_bytes(pdf_bytes: &[u8], page_index: usize, scale: f32) -> Result<RenderOutput> {
    let data = pdf_bytes.to_vec();
    let pdf = hayro::hayro_syntax::Pdf::new(data)
        .map_err(|e| FastPdfError::CorruptPdf(format!("hayro parse: {e:?}")))?;
    let pages = pdf.pages();
    let page = pages
        .get(page_index)
        .ok_or(FastPdfError::PageOutOfRange(page_index as u32))?;
    let cache = RenderCache::new();
    let interp = InterpreterSettings::default();
    let scale = if scale.is_finite() && scale > 0.05 { scale } else { 1.0 };
    let settings = RenderSettings {
        x_scale: scale,
        y_scale: scale,
        bg_color: WHITE,
        ..Default::default()
    };
    let mut pixmap = render(&page, &cache, &interp, &settings);
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

/// hayro / vello writes premultiplied RGBA. Canvas `ImageData` wants straight alpha.
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
