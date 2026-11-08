//! Hayro glyph device → [`TextSpan`] (PdfCraft-style, simplified).

use crate::types::{Rect, TextSpan};
use hayro::hayro_interpret::font::Glyph;
use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_interpret::{
    BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, InterpreterCache, InterpreterSettings,
    Paint, PathDrawMode, SoftMask, interpret_page,
};
use hayro::hayro_syntax::page::Page;
use hayro::vello_cpu::kurbo::{Affine, BezPath, Point, Rect as KurboRect, Shape};

struct SpanDevice {
    page_h: f32,
    spans: Vec<TextSpan>,
}

impl Device<'_> for SpanDevice {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'_>>) {}
    fn set_blend_mode(&mut self, _: BlendMode) {}
    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'_>, _: &PathDrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'_>>, _: BlendMode) {}
    fn pop_clip_path(&mut self) {}
    fn pop_transparency_group(&mut self) {}

    fn draw_glyph(
        &mut self,
        glyph: &Glyph<'_>,
        transform: Affine,
        glyph_transform: Affine,
        _: &Paint<'_>,
        mode: &GlyphDrawMode,
    ) {
        if matches!(mode, GlyphDrawMode::Invisible) {
            return;
        }
        let Some(unicode) = glyph.as_unicode() else {
            return;
        };
        let text: String = match unicode {
            hayro::hayro_interpret::hayro_cmap::BfString::Char(c) => c.to_string(),
            hayro::hayro_interpret::hayro_cmap::BfString::String(s) => s,
        };
        if text.chars().all(char::is_control) {
            return;
        }
        let t = transform * glyph_transform;
        let advance = match glyph {
            Glyph::Outline(g) => g
                .advance_width()
                .filter(|a| *a > 0.0)
                .map(f64::from)
                .unwrap_or_else(|| g.outline().bounding_box().width().max(500.0)),
            Glyph::Type3(_) => 600.0,
        };
        let em = KurboRect::new(0.0, -200.0, advance, 800.0);
        let corners = [(em.x0, em.y0), (em.x1, em.y0), (em.x1, em.y1), (em.x0, em.y1)]
            .map(|(x, y)| t * Point::new(x, y));
        let view = corners.iter().fold(
            KurboRect::new(f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
            |b, p| KurboRect::new(b.x0.min(p.x), b.y0.min(p.y), b.x1.max(p.x), b.y1.max(p.y)),
        );
        if !view.x0.is_finite() {
            return;
        }
        let w = (view.x1 - view.x0) as f32;
        let h = (view.y1 - view.y0) as f32;
        let x = view.x0 as f32;
        let y_view_top = view.y0 as f32;
        // View space: origin top-left, y down → PDF user space: origin bottom-left.
        let pdf = view_rect_to_pdf(x, y_view_top, w, h, self.page_h);
        let font_size = h.max(0.5);
        self.spans.push(TextSpan {
            text,
            x: pdf.x,
            y: pdf.y,
            width: pdf.width.max(0.5),
            height: pdf.height.max(0.5),
            y_top: y_view_top,
            font_size,
        });
    }

    fn draw_image(&mut self, _: Image<'_, '_>, _: Affine) {}
}

fn view_rect_to_pdf(x: f32, y_top: f32, w: f32, h: f32, page_h: f32) -> Rect {
    Rect {
        x,
        y: page_h - y_top - h,
        width: w,
        height: h,
    }
}

pub fn extract_page_spans(page: &Page, settings: &InterpreterSettings) -> Vec<TextSpan> {
    let (w, h) = page.render_dimensions();
    let cache = InterpreterCache::new();
    let initial = page.initial_transform(true).to_kurbo();
    let bbox = KurboRect::new(0.0, 0.0, f64::from(w), f64::from(h));
    let mut ctx = Context::new(initial, bbox, &cache, page.xref(), settings.clone());
    let mut dev = SpanDevice {
        page_h: h,
        spans: Vec::new(),
    };
    interpret_page(page, &mut ctx, &mut dev);
    dev.spans
}
