//! Reading-order layout, word gaps, and plain text for search/copy (PdfCraft-inspired heuristics).

use crate::types::{Rect, TextSpan};

const WORD_GAP: f32 = 0.15;
const LINE_TOL: f32 = 0.45;

pub fn is_rtl_char(c: char) -> bool {
    matches!(
        c as u32,
        0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF
    )
}

fn line_rtl(spans: &[&TextSpan]) -> bool {
    let (mut rtl, mut total) = (0usize, 0usize);
    for s in spans {
        for c in s.text.chars().filter(|c| !c.is_whitespace()) {
            total += 1;
            if is_rtl_char(c) {
                rtl += 1;
            }
        }
    }
    total > 0 && rtl * 2 >= total
}

/// Infer lines, word breaks, RTL, and line breaks on hayro glyph spans.
pub fn annotate_layout(spans: &mut [TextSpan]) {
    if spans.is_empty() {
        return;
    }
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by(|&a, &b| {
        let (sa, sb) = (&spans[a], &spans[b]);
        let lh = sa.font_size.max(sa.height).max(1.0);
        let dy = sa.y_top - sb.y_top;
        if dy.abs() > lh * LINE_TOL {
            sa.y_top.partial_cmp(&sb.y_top).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            sa.x.partial_cmp(&sb.x).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    let mut line_ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut start = 0usize;
    for i in 1..=order.len() {
        let break_line = i == order.len()
            || {
                let a = &spans[order[i - 1]];
                let b = &spans[order[i]];
                let lh = a.font_size.max(a.height).max(1.0);
                (a.y_top - b.y_top).abs() > lh * LINE_TOL
            };
        if break_line {
            line_ranges.push(start..i);
            start = i;
        }
    }

    for (line_no, range) in line_ranges.iter().enumerate() {
        let mut line_idxs: Vec<usize> = order[range.clone()].to_vec();
        let refs: Vec<&TextSpan> = line_idxs.iter().map(|&i| &spans[i]).collect();
        let rtl = line_rtl(&refs);
        line_idxs.sort_by(|&a, &b| {
            if rtl {
                spans[b].x.partial_cmp(&spans[a].x).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                spans[a].x.partial_cmp(&spans[b].x).unwrap_or(std::cmp::Ordering::Equal)
            }
        });
        for (i, &idx) in line_idxs.iter().enumerate() {
            spans[idx].rtl = rtl;
            spans[idx].line_break_before = i == 0 && line_no > 0;
            if i == 0 {
                spans[idx].space_before = false;
                continue;
            }
            let prev = &spans[line_idxs[i - 1]];
            let cur = &spans[idx];
            let lh = prev.font_size.max(prev.height).max(1.0);
            let prev_right = prev.x + prev.width;
            let gap = if rtl {
                prev.x - (cur.x + cur.width)
            } else {
                cur.x - prev_right
            };
            spans[idx].space_before = gap > lh * WORD_GAP;
        }
    }
}

pub struct PlainText {
    pub text: String,
    /// One entry per Unicode scalar in `text`.
    pub char_span: Vec<Option<usize>>,
}

pub fn plain_text_with_map(spans: &[TextSpan]) -> PlainText {
    let mut text = String::new();
    let mut char_span = Vec::new();
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by(|&a, &b| {
        let (sa, sb) = (&spans[a], &spans[b]);
        let lh = sa.font_size.max(sa.height).max(1.0);
        let dy = sa.y_top - sb.y_top;
        if dy.abs() > lh * LINE_TOL {
            sa.y_top.partial_cmp(&sb.y_top).unwrap_or(std::cmp::Ordering::Equal)
        } else if sa.rtl {
            sb.x.partial_cmp(&sa.x).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            sa.x.partial_cmp(&sb.x).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    for idx in order {
        let s = &spans[idx];
        if s.line_break_before && !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
            char_span.push(None);
        }
        if s.space_before && !text.is_empty() && !text.ends_with(' ') && !text.ends_with('\n') {
            text.push(' ');
            char_span.push(None);
        }
        for c in s.text.chars() {
            text.push(c);
            char_span.push(Some(idx));
        }
    }
    PlainText { text, char_span }
}

pub fn plain_text(spans: &[TextSpan]) -> String {
    plain_text_with_map(spans).text
}

pub fn fold_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'I' | 'i' | '\u{130}' | '\u{131}' => out.push('i'),
            'Σ' | 'σ' | 'ς' => out.push('σ'),
            _ => out.extend(c.to_lowercase()),
        }
    }
    out
}

/// Character-index ranges in `plain` matching `query` (case-insensitive).
pub fn find_unicode_ranges(plain: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    let needle: Vec<char> = fold_case(query).chars().collect();
    if needle.is_empty() {
        return vec![];
    }
    let hay: Vec<char> = fold_case(plain).chars().collect();
    let mut out = Vec::new();
    let mut start = 0usize;
    while start + needle.len() <= hay.len() {
        if hay[start..start + needle.len()] == needle[..] {
            out.push(start..start + needle.len());
            start += 1;
        } else {
            start += 1;
        }
    }
    out
}

fn union_rect(a: &Rect, b: &Rect) -> Rect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.width).max(b.x + b.width);
    let y1 = (a.y + a.height).max(b.y + b.height);
    Rect {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
    }
}

pub fn rects_for_query(spans: &[TextSpan], query: &str) -> Vec<Rect> {
    let mapped = plain_text_with_map(spans);
    let ranges = find_unicode_ranges(&mapped.text, query);
    let mut rects = Vec::new();
    for range in ranges {
        let mut span_ids = std::collections::BTreeSet::new();
        for (ci, slot) in mapped.char_span.iter().enumerate() {
            if ci >= range.start && ci < range.end {
                if let Some(s) = slot {
                    span_ids.insert(*s);
                }
            }
        }
        if span_ids.is_empty() {
            continue;
        }
        let mut union = None::<Rect>;
        for id in span_ids {
            let s = &spans[id];
            let r = Rect {
                x: s.x,
                y: s.y,
                width: s.width,
                height: s.height,
            };
            union = Some(match union {
                None => r,
                Some(u) => union_rect(&u, &r),
            });
        }
        if let Some(r) = union {
            rects.push(r);
        }
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_spaces() {
        let mut spans = vec![
            TextSpan {
                text: "Hello".into(),
                x: 10.0,
                y: 0.0,
                width: 30.0,
                height: 12.0,
                y_top: 100.0,
                font_size: 12.0,
                ..Default::default()
            },
            TextSpan {
                text: "world".into(),
                x: 50.0,
                y: 0.0,
                width: 30.0,
                height: 12.0,
                y_top: 100.0,
                font_size: 12.0,
                ..Default::default()
            },
        ];
        annotate_layout(&mut spans);
        assert!(spans[1].space_before);
        assert_eq!(plain_text(&spans), "Hello world");
    }

    #[test]
    fn unicode_search_arabic() {
        let plain = "مرحبا بالعالم";
        let hits = find_unicode_ranges(plain, "بالع");
        assert_eq!(hits.len(), 1);
    }
}
