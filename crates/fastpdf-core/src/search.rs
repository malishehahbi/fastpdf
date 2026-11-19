use crate::text_layout::{find_unicode_ranges, rects_for_char_range};
use crate::types::{PagePlain, SearchMatch, TextSpan};

pub fn search_page_plain(
    page_index: u32,
    plain: &PagePlain,
    spans: &[TextSpan],
    query: &str,
) -> Vec<SearchMatch> {
    if query.is_empty() {
        return vec![];
    }
    let chars: Vec<char> = plain.text.chars().collect();
    let mut matches = Vec::new();
    for range in find_unicode_ranges(&plain.text, query) {
        let snippet_start = range.start.saturating_sub(20);
        let snippet_end = (range.end + 20).min(chars.len());
        let snippet: String = chars[snippet_start..snippet_end].iter().collect();
        let rects = rects_for_char_range(spans, &plain.char_map, range.start, range.end);
        matches.push(SearchMatch {
            page_index,
            start: range.start,
            end: range.end,
            snippet,
            rects,
        });
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_layout::build_page_layout;
    use crate::types::{CharMapEntry, TextSpan};

    #[test]
    fn each_match_has_own_rects() {
        let mut spans = vec![TextSpan {
            text: "one two one".into(),
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 12.0,
            y_top: 10.0,
            font_size: 12.0,
            ..Default::default()
        }];
        let plain = build_page_layout(&mut spans).plain;
        let hits = search_page_plain(0, &plain, &spans, "one");
        assert_eq!(hits.len(), 2);
        assert_ne!(hits[0].rects[0].x, hits[1].rects[0].x);
    }

    #[test]
    fn empty_query_returns_nothing() {
        let plain = PagePlain {
            text: "x".into(),
            char_map: vec![CharMapEntry {
                span_index: 0,
                offset_in_span: 0,
            }],
        };
        assert!(search_page_plain(0, &plain, &[], " ").is_empty());
    }
}