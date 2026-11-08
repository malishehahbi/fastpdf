use crate::types::SearchMatch;

pub fn search_pages(pages_text: &[(u32, String)], query: &str) -> Vec<SearchMatch> {
    if query.is_empty() {
        return vec![];
    }
    let q = query.to_lowercase();
    let mut matches = Vec::new();
    for (page_index, text) in pages_text {
        let lower = text.to_lowercase();
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
                page_index: *page_index,
                start: abs,
                end,
                snippet,
            });
            start = abs + 1;
        }
    }
    matches
}
