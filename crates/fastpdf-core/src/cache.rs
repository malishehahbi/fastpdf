use crate::types::RenderOutput;
use std::collections::HashMap;

#[derive(Debug)]
pub struct TileCache {
    max_entries: usize,
    order: Vec<String>,
    map: HashMap<String, RenderOutput>,
}

impl TileCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            order: Vec::new(),
            map: HashMap::new(),
        }
    }

    pub fn set_max_entries(&mut self, max_entries: usize) {
        self.max_entries = max_entries.max(1);
        while self.order.len() > self.max_entries {
            if let Some(key) = self.order.first().cloned() {
                self.order.remove(0);
                self.map.remove(&key);
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&RenderOutput> {
        self.map.get(key)
    }

    pub fn insert(&mut self, key: String, value: RenderOutput) {
        if self.map.contains_key(&key) {
            self.order.retain(|k| k != &key);
        }
        self.order.push(key.clone());
        self.map.insert(key, value);
        while self.order.len() > self.max_entries {
            if let Some(old) = self.order.first().cloned() {
                self.order.remove(0);
                self.map.remove(&old);
            }
        }
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.map.clear();
    }
}
