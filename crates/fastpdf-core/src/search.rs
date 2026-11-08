use crate::text_layout::find_unicode_ranges;
use crate::types::{SearchMatch, TextSpan};

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
    let chars: Vec<char> = text.chars().collect();
    let mut matches = Vec::new();
    for range in find_unicode_ranges(text, query) {
        let snippet_start = range.start.saturating_sub(20);
        let snippet_end = (range.end + 20).min(chars.len());
        let snippet: String = chars[snippet_start..snippet_end].iter().collect();
        matches.push(SearchMatch {
            page_index,
            start: range.start,
            end: range.end,
            snippet,
            rects: Vec::new(),
        });
    }
    matches
}

/// Highlight rects for hayro spans (uses layout-aware plain text).
pub fn rects_for_query(spans: &[TextSpan], query: &str) -> Vec<crate::types::Rect> {
    crate::text_layout::rects_for_query(spans, query)
}
