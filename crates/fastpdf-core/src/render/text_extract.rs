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

/// Merge gap must stay strictly below `text_layout::WORD_GAP` (0.15) so word
/// boundaries always flush into separate spans and synthetic spaces are preserved.
const MERGE_MAX_GAP_EM: f32 = 0.12;
const MERGE_MIN_OVERLAP_EM: f32 = -0.50;
const MERGE_BASELINE_DELTA_EM: f32 = 0.20;
const MERGE_FONT_SIZE_LO: f32 = 0.85;
const MERGE_FONT_SIZE_HI: f32 = 1.15;

#[derive(Clone)]
struct PendingGlyph {
    text: String,
    view: KurboRect,
    advance: f32,
    font_key: Option<u128>,
    horizontal: bool,
    font_size: f32,
}

#[derive(Clone)]
struct PendingRun {
    glyphs: Vec<PendingGlyph>,
    horizontal: bool,
    direction_sign: i8,
    min_primary: f32,
    max_primary: f32,
    anchor: f32,
    font_size: f32,
}

struct SpanDevice {
    page_h: f32,
    spans: Vec<TextSpan>,
    pending: Option<PendingRun>,
}

impl SpanDevice {
    fn flush_pending(&mut self) {
        let Some(run) = self.pending.take() else {
            return;
        };
        if run.glyphs.is_empty() {
            return;
        }

        let mut x0 = run.glyphs[0].view.x0;
        let mut y0 = run.glyphs[0].view.y0;
        let mut x1 = run.glyphs[0].view.x1;
        let mut y1 = run.glyphs[0].view.y1;
        for g in &run.glyphs[1..] {
            x0 = x0.min(g.view.x0);
            y0 = y0.min(g.view.y0);
            x1 = x1.max(g.view.x1);
            y1 = y1.max(g.view.y1);
        }
        let view = KurboRect::new(x0, y0, x1, y1);

        let w = (view.x1 - view.x0) as f32;
        let h = (view.y1 - view.y0) as f32;
        let x = view.x0 as f32;
        let y_view_top = view.y0 as f32;
        let pdf = view_rect_to_pdf(x, y_view_top, w, h, self.page_h);

        let mut text = String::new();
        for g in &run.glyphs {
            text.push_str(&g.text);
        }

        let mut char_advances = Vec::new();
        for g in &run.glyphs {
            let n = g.text.chars().count().max(1);
            let per = g.advance / n as f32;
            for _ in 0..n {
                char_advances.push(per);
            }
        }

        let font_size = run
            .glyphs
            .iter()
            .map(|g| g.font_size)
            .fold(0.0_f32, f32::max)
            .max(0.5);

        self.spans.push(TextSpan {
            text,
            x: pdf.x,
            y: pdf.y,
            width: pdf.width.max(0.5),
            height: pdf.height.max(0.5),
            y_top: y_view_top,
            font_size,
            char_advances,
            ..Default::default()
        });
    }

    fn is_duplicate(&self, g: &PendingGlyph) -> bool {
        let Some(run) = &self.pending else {
            return false;
        };
        let Some(last) = run.glyphs.last() else {
            return false;
        };
        if run.glyphs.len() != 1 {
            return false;
        }
        last.text == g.text
            && last.font_key == g.font_key
            && last.horizontal == g.horizontal
            && last.advance == g.advance
            && (last.view.x0 - g.view.x0).abs() < f64::EPSILON
            && (last.view.y0 - g.view.y0).abs() < f64::EPSILON
    }

    /// Accumulate one glyph into the current run, flushing first when needed.
    fn accumulate_glyph(
        &mut self,
        text: String,
        view: KurboRect,
        advance: f32,
        font_key: Option<u128>,
        horizontal: bool,
        font_size: f32,
    ) {
        let g = PendingGlyph {
            text,
            view,
            advance,
            font_key,
            horizontal,
            font_size,
        };

        if self.is_duplicate(&g) {
            return;
        }

        let mut need_flush = false;
        if let Some(run) = &self.pending {
            let last = run.glyphs.last().expect("non-empty run");
            if last.font_key != g.font_key || last.horizontal != g.horizontal {
                need_flush = true;
            } else {
                let fs = last.font_size.max(0.01);
                let ratio = g.font_size / fs;
                if !(MERGE_FONT_SIZE_LO..=MERGE_FONT_SIZE_HI).contains(&ratio) {
                    need_flush = true;
                } else {
                    // For vertical runs use horizontal anchor instead.
                    let baseline_delta = if run.horizontal {
                        (g.view.y0 as f32 - run.anchor).abs()
                    } else {
                        (g.view.x0 as f32 - run.anchor).abs()
                    };
                    let threshold = MERGE_BASELINE_DELTA_EM * last.font_size.max(g.font_size);
                    if baseline_delta > threshold {
                        need_flush = true;
                    } else {
                        // Direction / gap check along the primary axis.
                        let primary_new0 = if run.horizontal {
                            g.view.x0 as f32
                        } else {
                            g.view.y0 as f32
                        };
                        let primary_new1 = if run.horizontal {
                            g.view.x1 as f32
                        } else {
                            g.view.y1 as f32
                        };

                        let last_primary0 = if run.horizontal {
                            last.view.x0 as f32
                        } else {
                            last.view.y0 as f32
                        };
                        let tentative_dir: i8 = if run.direction_sign != 0 {
                            run.direction_sign
                        } else if primary_new0 > last_primary0 + 0.01 {
                            1
                        } else if primary_new0 < last_primary0 - 0.01 {
                            -1
                        } else {
                            0
                        };

                        let new_sign: i8 = if primary_new0 > run.min_primary + 0.01 {
                            1
                        } else if primary_new0 < run.min_primary - 0.01 {
                            -1
                        } else {
                            tentative_dir
                        };

                        if run.direction_sign != 0
                            && new_sign != 0
                            && new_sign != run.direction_sign
                        {
                            need_flush = true;
                        } else {
                            let (gap, min_overlap, max_gap) = if tentative_dir >= 0 {
                                let gap = primary_new0 - run.max_primary;
                                (
                                    gap,
                                    MERGE_MIN_OVERLAP_EM * last.font_size.max(g.font_size),
                                    MERGE_MAX_GAP_EM * last.font_size.max(g.font_size),
                                )
                            } else {
                                let gap = run.min_primary - primary_new1;
                                (
                                    gap,
                                    MERGE_MIN_OVERLAP_EM * last.font_size.max(g.font_size),
                                    MERGE_MAX_GAP_EM * last.font_size.max(g.font_size),
                                )
                            };
                            if gap < min_overlap || gap > max_gap {
                                need_flush = true;
                            }
                        }
                    }
                }
            }
        }

        if need_flush {
            self.flush_pending();
        }

        if let Some(run) = &mut self.pending {
            run.glyphs.push(g.clone());
            // Update direction sign if still unknown.
            if run.direction_sign == 0 && run.glyphs.len() >= 2 {
                let a = &run.glyphs[run.glyphs.len() - 2];
                let b = &run.glyphs[run.glyphs.len() - 1];
                let pa = if run.horizontal {
                    a.view.x0 as f32
                } else {
                    a.view.y0 as f32
                };
                let pb = if run.horizontal {
                    b.view.x0 as f32
                } else {
                    b.view.y0 as f32
                };
                if pb > pa + 0.01 {
                    run.direction_sign = 1;
                } else if pb < pa - 0.01 {
                    run.direction_sign = -1;
                }
            }
            // Update primary extents.
            let (n0, n1) = if run.horizontal {
                (g.view.x0 as f32, g.view.x1 as f32)
            } else {
                (g.view.y0 as f32, g.view.y1 as f32)
            };
            run.min_primary = run.min_primary.min(n0);
            run.max_primary = run.max_primary.max(n1);
            run.font_size = run.font_size.max(g.font_size);
        } else {
            let (n0, n1) = if horizontal {
                (g.view.x0 as f32, g.view.x1 as f32)
            } else {
                (g.view.y0 as f32, g.view.y1 as f32)
            };
            let anchor = if horizontal {
                g.view.y0 as f32
            } else {
                g.view.x0 as f32
            };
            self.pending = Some(PendingRun {
                glyphs: vec![g],
                horizontal,
                direction_sign: 0,
                min_primary: n0,
                max_primary: n1,
                anchor,
                font_size,
            });
        }
    }
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
            self.flush_pending();
            return;
        }
        let Some(unicode) = glyph.as_unicode() else {
            self.flush_pending();
            return;
        };
        let text: String = match unicode {
            hayro::hayro_interpret::hayro_cmap::BfString::Char(c) => c.to_string(),
            hayro::hayro_interpret::hayro_cmap::BfString::String(s) => s,
        };
        if text.chars().all(char::is_control) {
            self.flush_pending();
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
        let font_key = match glyph {
            Glyph::Outline(g) => Some(g.font_cache_key()),
            Glyph::Type3(_) => None,
        };

        // Accurate ink bounds (outline bbox) plus em-square height.
        let em = KurboRect::new(0.0, -200.0, advance, 800.0);
        let mut points: Vec<Point> = Vec::new();
        for (x, y) in [(em.x0, em.y0), (em.x1, em.y0), (em.x1, em.y1), (em.x0, em.y1)] {
            points.push(t * Point::new(x, y));
        }
        if let Glyph::Outline(g) = glyph {
            let bb = g.outline().bounding_box();
            if bb.width() > 0.0 && bb.height() > 0.0 {
                for (x, y) in [
                    (bb.x0, bb.y0),
                    (bb.x1, bb.y0),
                    (bb.x1, bb.y1),
                    (bb.x0, bb.y1),
                ] {
                    points.push(t * Point::new(x, y));
                }
            }
        }

        let view = points.iter().fold(
            KurboRect::new(f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
            |b, p| KurboRect::new(b.x0.min(p.x), b.y0.min(p.y), b.x1.max(p.x), b.y1.max(p.y)),
        );
        if !view.x0.is_finite() {
            return;
        }

        let font_size = ((view.y1 - view.y0) as f32).max(0.5);

        let horizontal = {
            let p0 = t * Point::new(0.0, 0.0);
            let p1 = t * Point::new(advance, 0.0);
            (p1.x - p0.x).abs() >= (p1.y - p0.y).abs()
        };

        self.accumulate_glyph(text, view, advance as f32, font_key, horizontal, font_size);
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
        pending: None,
    };
    interpret_page(page, &mut ctx, &mut dev);
    dev.flush_pending();
    dev.spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_adjacent_glyphs() {
        let mut dev = SpanDevice {
            page_h: 100.0,
            spans: Vec::new(),
            pending: None,
        };
        dev.accumulate_glyph("H".into(), KurboRect::new(0.0, 10.0, 10.0, 22.0), 10.0, Some(1), true, 12.0);
        dev.accumulate_glyph("i".into(), KurboRect::new(10.0, 10.0, 14.0, 22.0), 4.0, Some(1), true, 12.0);
        dev.flush_pending();
        assert_eq!(dev.spans.len(), 1);
        assert_eq!(dev.spans[0].text, "Hi");
        assert_eq!(dev.spans[0].char_advances.len(), 2);
        assert!(dev.spans[0].width > 10.0);
    }

    #[test]
    fn flush_on_word_gap() {
        let mut dev = SpanDevice {
            page_h: 100.0,
            spans: Vec::new(),
            pending: None,
        };
        dev.accumulate_glyph("Hello".into(), KurboRect::new(0.0, 10.0, 30.0, 22.0), 30.0, Some(1), true, 12.0);
        dev.accumulate_glyph("world".into(), KurboRect::new(50.0, 10.0, 80.0, 22.0), 30.0, Some(1), true, 12.0);
        dev.flush_pending();
        assert_eq!(dev.spans.len(), 2);
    }

    #[test]
    fn flush_on_baseline_or_size_shift() {
        let mut dev = SpanDevice {
            page_h: 100.0,
            spans: Vec::new(),
            pending: None,
        };
        dev.accumulate_glyph("x".into(), KurboRect::new(0.0, 10.0, 10.0, 22.0), 10.0, Some(1), true, 12.0);
        dev.accumulate_glyph("2".into(), KurboRect::new(10.0, 0.0, 20.0, 12.0), 8.0, Some(1), true, 8.0);
        dev.flush_pending();
        assert_eq!(dev.spans.len(), 2);
    }

    #[test]
    fn combining_diacritic_and_kerning_merged() {
        let mut dev = SpanDevice {
            page_h: 100.0,
            spans: Vec::new(),
            pending: None,
        };
        dev.accumulate_glyph("a".into(), KurboRect::new(0.0, 10.0, 10.0, 22.0), 10.0, Some(1), true, 12.0);
        dev.accumulate_glyph("\u{0301}".into(), KurboRect::new(8.0, 8.0, 12.0, 14.0), 0.0, Some(1), true, 12.0);
        dev.flush_pending();
        assert_eq!(dev.spans.len(), 1);
        assert_eq!(dev.spans[0].text.chars().count(), 2);
    }

    #[test]
    fn ligature_char_advances() {
        let mut dev = SpanDevice {
            page_h: 100.0,
            spans: Vec::new(),
            pending: None,
        };
        dev.accumulate_glyph("fi".into(), KurboRect::new(0.0, 10.0, 12.0, 22.0), 12.0, Some(1), true, 12.0);
        dev.flush_pending();
        assert_eq!(dev.spans.len(), 1);
        assert_eq!(dev.spans[0].char_advances.len(), 2);
    }

    #[test]
    fn rtl_run_merged() {
        let mut dev = SpanDevice {
            page_h: 100.0,
            spans: Vec::new(),
            pending: None,
        };
        dev.accumulate_glyph("ب".into(), KurboRect::new(20.0, 10.0, 30.0, 22.0), 10.0, Some(1), true, 12.0);
        dev.accumulate_glyph("ا".into(), KurboRect::new(10.0, 10.0, 20.0, 22.0), 10.0, Some(1), true, 12.0);
        dev.flush_pending();
        assert_eq!(dev.spans.len(), 1);
        assert_eq!(dev.spans[0].text, "با");
    }

    #[test]
    fn dedup_fill_stroke() {
        let mut dev = SpanDevice {
            page_h: 100.0,
            spans: Vec::new(),
            pending: None,
        };
        dev.accumulate_glyph("A".into(), KurboRect::new(0.0, 10.0, 10.0, 22.0), 10.0, Some(1), true, 12.0);
        dev.accumulate_glyph("A".into(), KurboRect::new(0.0, 10.0, 10.0, 22.0), 10.0, Some(1), true, 12.0);
        dev.flush_pending();
        assert_eq!(dev.spans.len(), 1);
        assert_eq!(dev.spans[0].text, "A");
    }
}