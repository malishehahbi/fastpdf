use crate::types::{Rect, SearchMatch, TextSpan};

pub fn search_pages(pages_text: &[(u32, String)], query: &str) -> Vec<SearchMatch> {
    if query.is_empty() {
        return vec![];
    }
    let mut matches = Vec::new();
    for (page_index, text) in pages_text {
        matches.extend(search_page_text(*page_index, text, query));
    }
    matches
}

pub fn search_page_text(page_index: u32, text: &str, query: &str) -> Vec<SearchMatch> {
    if query.is_empty() {
        return vec![];
    }
    let q = query.to_lowercase();
    let lower = text.to_lowercase();
    let mut matches = Vec::new();
    let mut start = 0;
    while let Some(pos) = lower[start..].find(&q) {
        let abs = start + pos;
        let end = abs + q.len();
        let snippet = text
            .chars()
            .skip(abs.saturating_sub(20))
            .take(q.len() + 40)
            .collect();
        matches.push(SearchMatch {
            page_index,
            start: abs,
            end,
            snippet,
            rects: Vec::new(),
        });
        start = abs + 1;
    }
    matches
}

/// Highlight rects for spans whose text contains `query` (hayro glyph boxes).
pub fn rects_for_query(spans: &[TextSpan], query: &str) -> Vec<Rect> {
    if query.is_empty() {
        return vec![];
    }
    let q = query.to_lowercase();
    spans
        .iter()
        .filter(|s| s.text.to_lowercase().contains(&q))
        .map(|s| Rect {
            x: s.x,
            y: s.y,
            width: s.width,
            height: s.height,
        })
        .collect()
}
