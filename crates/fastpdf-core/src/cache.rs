use crate::types::RenderOutput;
use std::collections::HashMap;

/// Default pixel cache budget (64 MiB).
pub const DEFAULT_CACHE_BYTES: usize = 64 * 1024 * 1024;

/// Thousandths-of-a-unit quantization for cache keys.
pub fn quantize_f32(value: f32) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1000.0).round() as i64
}

pub fn page_cache_key(page_index: u32, scale: f32) -> String {
    format!("page-{page_index}-{}", quantize_f32(scale))
}

pub fn thumb_cache_key(page_index: u32, max_side: f32) -> String {
    format!("thumb-{page_index}-{}", quantize_f32(max_side))
}

pub fn tile_cache_key(
    page_index: u32,
    scale: f32,
    tile_x: f32,
    tile_y: f32,
    tile_w: f32,
    tile_h: f32,
) -> String {
    format!(
        "tile-{page_index}-{}-{}-{}-{}-{}",
        quantize_f32(scale),
        quantize_f32(tile_x),
        quantize_f32(tile_y),
        quantize_f32(tile_w),
        quantize_f32(tile_h)
    )
}

#[derive(Debug)]
pub struct TileCache {
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
    order: Vec<String>,
    map: HashMap<String, RenderOutput>,
}

impl TileCache {
    pub fn new(max_entries: usize) -> Self {
        Self::with_limits(max_entries, DEFAULT_CACHE_BYTES)
    }

    pub fn with_limits(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            max_bytes: max_bytes.max(1),
            current_bytes: 0,
            order: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    pub fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    pub fn set_max_entries(&mut self, max_entries: usize) {
        self.max_entries = max_entries.max(1);
        self.enforce_limits();
    }

    pub fn set_max_bytes(&mut self, max_bytes: usize) {
        self.max_bytes = max_bytes.max(1);
        self.enforce_limits();
    }

    /// Returns a clone of the cached output (`Arc` pixel buffer is shared).
    pub fn get(&mut self, key: &str) -> Option<RenderOutput> {
        let value = self.map.get(key)?.clone();
        self.touch(key);
        Some(value)
    }

  /// Peek without LRU touch (used when cropping a parent page that stays hot elsewhere).
    pub fn peek(&self, key: &str) -> Option<RenderOutput> {
        self.map.get(key).cloned()
    }

    /// Store `value` when it fits the byte budget. Returns whether the entry was cached.
    pub fn try_insert(&mut self, key: String, value: RenderOutput) -> bool {
        let bytes = value.byte_len();
        if bytes > self.max_bytes {
            return false;
        }

        if let Some(old) = self.map.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(old.byte_len());
            self.order.retain(|k| k != &key);
        }

        while (!self.order.is_empty())
            && (self.current_bytes + bytes > self.max_bytes || self.order.len() >= self.max_entries)
        {
            self.evict_oldest();
        }

        self.current_bytes += bytes;
        self.order.push(key.clone());
        self.map.insert(key, value);
        true
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.map.clear();
        self.current_bytes = 0;
    }

    fn touch(&mut self, key: &str) {
        self.order.retain(|k| k != key);
        self.order.push(key.to_string());
    }

    fn evict_oldest(&mut self) {
        let Some(old_key) = self.order.first().cloned() else {
            return;
        };
        self.order.remove(0);
        if let Some(old) = self.map.remove(&old_key) {
            self.current_bytes = self.current_bytes.saturating_sub(old.byte_len());
        }
    }

    fn enforce_limits(&mut self) {
        while (!self.order.is_empty())
            && (self.current_bytes > self.max_bytes || self.order.len() > self.max_entries)
        {
            self.evict_oldest();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn sample_output(byte_fill: u8, pixel_count: usize) -> RenderOutput {
        let pixels: Arc<[u8]> = vec![byte_fill; pixel_count * 4].into();
        RenderOutput {
            width: pixel_count as u32,
            height: 1,
            pixels,
        }
    }

    #[test]
    fn quantize_scale_equivalence() {
        assert_eq!(quantize_f32(1.5), quantize_f32(1.500_000_1));
        assert_eq!(
            page_cache_key(0, 1.5),
            page_cache_key(0, 1.500_000_1)
        );
    }

    #[test]
    fn arc_shared_on_cache_hit() {
        let mut cache = TileCache::with_limits(4, DEFAULT_CACHE_BYTES);
        let out = sample_output(42, 16);
        cache.try_insert("a".into(), out);
        let hit = cache.get("a").expect("hit");
        let hit2 = cache.get("a").expect("hit2");
        assert!(Arc::ptr_eq(&hit.pixels, &hit2.pixels));
    }

    #[test]
    fn byte_budget_never_exceeded() {
        let mut cache = TileCache::with_limits(100, 150);
        assert!(cache.try_insert("a".into(), sample_output(1, 30)));
        assert_eq!(cache.current_bytes(), 120);
        assert!(cache.try_insert("b".into(), sample_output(2, 20)));
        assert!(cache.current_bytes() <= 150);
        assert!(cache.peek("a").is_none());
        assert!(cache.peek("b").is_some());
        assert!(!cache.try_insert("c".into(), sample_output(3, 50)));
        assert!(cache.current_bytes() <= 150);
        assert!(cache.peek("c").is_none());
    }

    #[test]
    fn oversized_entry_does_not_loop() {
        let mut cache = TileCache::with_limits(10, 64);
        assert!(!cache.try_insert("huge".into(), sample_output(9, 32)));
        assert_eq!(cache.current_bytes(), 0);
        assert!(cache.try_insert("ok".into(), sample_output(1, 8)));
        assert_eq!(cache.current_bytes(), 32);
    }
}