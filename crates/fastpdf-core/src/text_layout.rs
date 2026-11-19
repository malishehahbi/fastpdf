//! Reading-order layout, plain-text projection, and search geometry (single canonical pass).

use crate::types::{CharMapEntry, PagePlain, Rect, TextLine, TextSpan, SYNTHETIC_CHAR_MAP};
use std::cmp::Ordering;

const WORD_GAP: f32 = 0.15;
const LINE_TOL: f32 = 0.45;
const COLUMN_GAP: f32 = 2.0;

#[derive(Debug, Clone)]
pub struct PageLayout {
    pub lines: Vec<TextLine>,
    pub plain: PagePlain,
}

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

fn line_height(span: &TextSpan) -> f32 {
    span.font_size.max(span.height).max(1.0)
}

fn cmp_vertical(a: &TextSpan, b: &TextSpan) -> Ordering {
    let lh = line_height(a).max(line_height(b));
    let dy = a.y_top - b.y_top;
    if dy.abs() > lh * LINE_TOL {
        a.y_top
            .partial_cmp(&b.y_top)
            .unwrap_or(Ordering::Equal)
    } else {
        a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal)
    }
}

/// Single layout pass: annotates spans, returns reading-order lines and searchable plain text.
pub fn build_page_layout(spans: &mut Vec<TextSpan>) -> PageLayout {
    if spans.is_empty() {
        return PageLayout {
            lines: vec![],
            plain: PagePlain {
                text: String::new(),
                char_map: vec![],
            },
        };
    }

    for s in spans.iter_mut() {
        s.space_before = false;
        s.line_break_before = false;
        s.rtl = false;
    }

    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by(|&a, &b| cmp_vertical(&spans[a], &spans[b]));

    let y_bands = split_y_bands(&order, spans);

    let mut text_lines: Vec<TextLine> = Vec::new();
    let mut reading_order: Vec<usize> = Vec::new();

    for band in y_bands {
        let columns = split_columns(&band, spans);
        for (col_no, col_idxs) in columns.into_iter().enumerate() {
            if col_idxs.is_empty() {
                continue;
            }
            let refs: Vec<&TextSpan> = col_idxs.iter().map(|&i| &spans[i]).collect();
            let rtl = line_rtl(&refs);
            let mut sorted = col_idxs;
            sorted.sort_by(|&a, &b| {
                if rtl {
                    spans[b].x.partial_cmp(&spans[a].x).unwrap_or(Ordering::Equal)
                } else {
                    spans[a].x.partial_cmp(&spans[b].x).unwrap_or(Ordering::Equal)
                }
            });

            let y_top = sorted
                .iter()
                .map(|&i| spans[i].y_top)
                .fold(f32::INFINITY, f32::min);

            for (i, &idx) in sorted.iter().enumerate() {
                spans[idx].rtl = rtl;
                spans[idx].line_break_before = !reading_order.is_empty() && i == 0;

                if i > 0 {
                    let prev_idx = sorted[i - 1];
                    let prev = &spans[prev_idx];
                    let cur = &spans[idx];
                    let lh = line_height(prev);
                    let prev_right = prev.x + prev.width;
                    let gap = if rtl {
                        prev.x - (cur.x + cur.width)
                    } else {
                        cur.x - prev_right
                    };
                    spans[idx].space_before = gap > lh * WORD_GAP;
                } else {
                    spans[idx].space_before = false;
                }

                reading_order.push(idx);
            }

            text_lines.push(TextLine {
                span_indices: sorted,
                rtl,
                column: col_no as u32,
                y_top,
            });
        }
    }

    let mut reordered: Vec<TextSpan> = Vec::with_capacity(reading_order.len());
    for &idx in &reading_order {
        reordered.push(spans[idx].clone());
    }
    *spans = reordered;

    let plain = build_plain_from_spans(spans);

    PageLayout {
        lines: text_lines,
        plain,
    }
}

fn split_y_bands(order: &[usize], spans: &[TextSpan]) -> Vec<Vec<usize>> {
    if order.is_empty() {
        return vec![];
    }
    let mut bands: Vec<Vec<usize>> = Vec::new();
    let mut current = vec![order[0]];
    for &idx in &order[1..] {
        let prev = &spans[*current.last().expect("non-empty band")];
        let cur = &spans[idx];
        let lh = line_height(prev);
        if (prev.y_top - cur.y_top).abs() > lh * LINE_TOL {
            bands.push(current);
            current = vec![idx];
        } else {
            current.push(idx);
        }
    }
    bands.push(current);
    bands
}

fn split_columns(line_idxs: &[usize], spans: &[TextSpan]) -> Vec<Vec<usize>> {
    if line_idxs.is_empty() {
        return vec![];
    }
    let mut sorted = line_idxs.to_vec();
    sorted.sort_by(|&a, &b| {
        spans[a]
            .x
            .partial_cmp(&spans[b].x)
            .unwrap_or(Ordering::Equal)
    });

    let lh = sorted
        .iter()
        .map(|&i| line_height(&spans[i]))
        .fold(1.0_f32, f32::max);

    let mut columns: Vec<Vec<usize>> = vec![vec![sorted[0]]];
    for &idx in &sorted[1..] {
        let prev_idx = *columns
            .last()
            .and_then(|c| c.last())
            .expect("non-empty column");
        let prev = &spans[prev_idx];
        let cur = &spans[idx];
        let prev_right = prev.x + prev.width;
        if cur.x - prev_right > lh * COLUMN_GAP {
            columns.push(vec![idx]);
        } else {
            columns.last_mut().expect("column").push(idx);
        }
    }
    columns
}

fn build_plain_from_spans(spans: &[TextSpan]) -> PagePlain {
    let mut text = String::new();
    let mut char_map = Vec::new();

    for (span_index, s) in spans.iter().enumerate() {
        if s.line_break_before && !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
            char_map.push(CharMapEntry::synthetic());
        }
        if s.space_before && !text.is_empty() && !text.ends_with(' ') && !text.ends_with('\n') {
            text.push(' ');
            char_map.push(CharMapEntry::synthetic());
        }
        for (offset, c) in s.text.chars().enumerate() {
            text.push(c);
            char_map.push(CharMapEntry {
                span_index,
                offset_in_span: offset as u32,
            });
        }
    }

    PagePlain { text, char_map }
}

/// One folded character per original Unicode scalar (length-preserving).
pub fn fold_case_char(c: char) -> char {
    match c {
        'I' | 'i' | '\u{130}' | '\u{131}' => 'i',
        'Σ' | 'σ' | 'ς' => 'σ',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

/// Character-index ranges in `plain` matching `query` (case-insensitive, index-aligned with `plain`).
pub fn find_unicode_ranges(plain: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    let hay: Vec<char> = plain.chars().map(fold_case_char).collect();
    let needle: Vec<char> = query.chars().map(fold_case_char).collect();
    if needle.is_empty() || hay.len() < needle.len() {
        return vec![];
    }
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

fn span_char_count(span: &TextSpan) -> usize {
    span.text.chars().count()
}

fn slice_span_rect(span: &TextSpan, char_start: usize, char_len: usize) -> Option<Rect> {
    let total = span_char_count(span).max(1);
    if char_len == 0 {
        return None;
    }
    let start = char_start.min(total);
    let end = (char_start + char_len).min(total);
    if start >= end {
        return None;
    }
    let frac0 = start as f32 / total as f32;
    let frac1 = end as f32 / total as f32;
    let x0 = span.x + span.width * frac0;
    let w = span.width * (frac1 - frac0).max(0.5 / total as f32);
    Some(Rect {
        x: x0,
        y: span.y,
        width: w.max(0.5),
        height: span.height.max(0.5),
    })
}

/// Bounding rects for `plain.text[start..end]` using proportional sub-span geometry.
pub fn rects_for_char_range(
    spans: &[TextSpan],
    char_map: &[CharMapEntry],
    start: usize,
    end: usize,
) -> Vec<Rect> {
    if start >= end || char_map.len() < end {
        return vec![];
    }

    #[derive(Clone, Copy)]
    struct Run {
        span_index: usize,
        char_start: usize,
        char_len: usize,
    }

    let mut runs: Vec<Run> = Vec::new();
    for entry in char_map.get(start..end).unwrap_or(&[]) {
        if entry.span_index == SYNTHETIC_CHAR_MAP {
            continue;
        }
        let offset = entry.offset_in_span as usize;
        let extend = runs.last_mut().is_some_and(|last| {
            last.span_index == entry.span_index && last.char_start + last.char_len == offset
        });
        if extend {
            runs.last_mut().expect("checked").char_len += 1;
        } else {
            runs.push(Run {
                span_index: entry.span_index,
                char_start: offset,
                char_len: 1,
            });
        }
    }

    let mut rects = Vec::new();
    for run in runs {
        if run.span_index >= spans.len() {
            continue;
        }
        let span = &spans[run.span_index];
        if let Some(r) = slice_span_rect(span, run.char_start, run.char_len) {
            let merge = rects
                .last()
                .is_some_and(|prev: &Rect| prev.y == r.y && (prev.x + prev.width - r.x).abs() < 1.0);
            if merge {
                let prev = rects.pop().expect("checked");
                rects.push(union_rect(&prev, &r));
            } else {
                rects.push(r);
            }
        }
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PagePlain;

    fn span_at(text: &str, x: f32, y_top: f32, width: f32) -> TextSpan {
        TextSpan {
            text: text.to_string(),
            x,
            y: 0.0,
            width,
            height: 12.0,
            y_top,
            font_size: 12.0,
            ..Default::default()
        }
    }

    #[test]
    fn inserts_spaces_and_plain() {
        let mut spans = vec![
            span_at("Hello", 10.0, 100.0, 30.0),
            span_at("world", 50.0, 100.0, 30.0),
        ];
        let layout = build_page_layout(&mut spans);
        assert!(spans[1].space_before);
        assert_eq!(layout.plain.text, "Hello world");
    }

    #[test]
    fn two_column_reading_order() {
        let mut spans = vec![
            span_at("LeftA", 10.0, 100.0, 40.0),
            span_at("RightA", 320.0, 100.0, 40.0),
            span_at("LeftB", 10.0, 120.0, 40.0),
            span_at("RightB", 320.0, 120.0, 40.0),
        ];
        let layout = build_page_layout(&mut spans);
        assert!(layout.plain.text.contains("LeftA"));
        assert!(layout.plain.text.contains("RightA"));
        assert_eq!(layout.lines.len(), 4);
    }

    #[test]
    fn rtl_line_ordering() {
        let mut spans = vec![
            span_at("ب", 200.0, 50.0, 15.0),
            span_at("ا", 180.0, 50.0, 15.0),
            span_at("ت", 160.0, 50.0, 15.0),
        ];
        build_page_layout(&mut spans);
        assert!(spans.iter().all(|s| s.rtl));
        assert!(spans[0].x >= spans[1].x);
    }

    #[test]
    fn unicode_search_arabic_index_aligned() {
        let plain = "مرحبا بالعالم";
        let hits = find_unicode_ranges(plain, "بالع");
        assert_eq!(hits.len(), 1);
        let idx = hits[0].start;
        assert_eq!(plain.chars().nth(idx).unwrap(), 'ب');
    }

    #[test]
    fn turkish_case_fold_preserves_indices() {
        let plain = "Işık";
        let folded: String = plain.chars().map(fold_case_char).collect();
        assert_eq!(folded.chars().count(), plain.chars().count());
        let hits = find_unicode_ranges(plain, "ışık");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0], 0..plain.chars().count());
    }

    #[test]
    fn per_hit_rects_subspan() {
        let spans = vec![span_at("HelloFastPDF", 0.0, 10.0, 120.0)];
        let plain = build_plain_from_spans(&spans);
        let rects = rects_for_char_range(&spans, &plain.char_map, 5, 8);
        assert_eq!(rects.len(), 1);
        assert!(rects[0].width < spans[0].width);

        let mut spans2 = vec![
            span_at("foo", 0.0, 10.0, 30.0),
            span_at("bar", 40.0, 10.0, 30.0),
        ];
        let page = build_page_layout(&mut spans2);
        let r_foo = rects_for_char_range(&spans2, &page.plain.char_map, 0, 3);
        let r_bar = rects_for_char_range(&spans2, &page.plain.char_map, 4, 7);
        assert!(r_foo[0].x < r_bar[0].x);
    }

    #[test]
    fn multiple_matches_isolated_rects() {
        let spans = vec![span_at("aa bb aa", 0.0, 10.0, 200.0)];
        let plain = build_plain_from_spans(&spans);
        let hits = find_unicode_ranges(&plain.text, "aa");
        assert_eq!(hits.len(), 2);
        let r0 = rects_for_char_range(&spans, &plain.char_map, hits[0].start, hits[0].end);
        let r1 = rects_for_char_range(&spans, &plain.char_map, hits[1].start, hits[1].end);
        assert!(r0[0].x < r1[0].x);
    }

    #[test]
    fn search_snippet_matches_range() {
        let plain = PagePlain {
            text: "İstanbul".to_string(),
            char_map: (0..8)
                .map(|i| CharMapEntry {
                    span_index: 0,
                    offset_in_span: i,
                })
                .collect(),
        };
        let spans = vec![span_at("İstanbul", 0.0, 0.0, 80.0)];
        let hits = find_unicode_ranges(&plain.text, "istanbul");
        assert!(!hits.is_empty());
        let r = hits[0].clone();
        let snippet: String = plain.text.chars().skip(r.start).take(r.end - r.start).collect();
        assert_eq!(fold_case_char(snippet.chars().next().unwrap()), 'i');
        let rects = rects_for_char_range(&spans, &plain.char_map, r.start, r.end);
        assert!(!rects.is_empty());
    }
}