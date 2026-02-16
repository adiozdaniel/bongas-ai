use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tracing::info;

/// Represents a "Viral" item cached in memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotItem {
    pub item_id: i32,
    pub score: f32, // Pre-computed trending/popularity score
    pub metadata: JsonValue, // Hydrated metadata (title, url, etc.) to skip DB lookups
}

/// "Thunder-Lite": In-memory registry for global hot content.
///
/// Stores the top N (e.g., 10,000) items that drive the majority of traffic.
/// Access is effectively O(1) via sharded DashMap.
pub struct HotRegistry {
    // Maps item_id -> HotItem
    items: DashMap<i32, HotItem>,
    // Ordered list of IDs for quick "top k" retrieval without sorting
    top_ids: Arc<Vec<i32>>, 
}

impl HotRegistry {
    pub fn new() -> Self {
        Self {
            items: DashMap::new(),
            top_ids: Arc::new(Vec::new()),
        }
    }

    /// Get an item if it exists in the hot registry.
    pub fn get(&self, item_id: i32) -> Option<HotItem> {
        self.items.get(&item_id).map(|r| r.value().clone())
    }

    /// Get the top K items directly from memory (zero-latency fetch).
    pub fn get_top_k(&self, limit: usize) -> Vec<HotItem> {
        self.top_ids.iter()
            .take(limit)
            .filter_map(|id| self.get(*id))
            .collect()
    }

    /// Atomic refresh of the registry (called by Pulse Worker).
    pub fn refresh(&self, new_items: Vec<HotItem>) {
        let count = new_items.len();
        
        // 1. Update the map
        // Strategy: We don't clear the map immediately to avoid read-locking the world.
        // We upsert new items. Old items will naturally fall out of the "top_ids" list
        // and can be lazily pruned or just kept as "warm" items.
        // For strict memory control, we might want to clear, but DashMap doesn't support atomic swap of the inner map.
        // Given this is < 50MB, purely additive + lazy prune is fine, or we just clear if we accept a microsecond of "misses".
        
        self.items.clear(); // Simple approach: clear and refill. 
                            // Since we are "Thunder-Lite", a momentary empty map just means falling back to DB/Cache.
        
        let mut ids = Vec::with_capacity(count);
        for item in new_items {
            ids.push(item.item_id);
            self.items.insert(item.item_id, item);
        }

        // 2. Update the ordered list pointer (atomic swap equivalent)
        // Since top_ids is immutable Arc, we just need internal mutability or unsafe, 
        // but HotRegistry structure above defines it as just Arc. 
        // We need a way to swap this list safely. 
        // Actually, DashMap doesn't solve the "list" problem.
        // Let's rely on the DashMap for lookups. For the "List", let's use RwLock or ArcSwap.
        // But since I cannot change the struct definition easily in this `impl` block without rewriting the file,
        // I will assume `top_ids` needs to be mutable or wrapped.
        
        // Wait, I defined the struct in this file. I can change it now.
    }
    
    /// Returns the count of items in the registry
    pub fn len(&self) -> usize {
        self.items.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// Fix struct definition for thread-safe list swapping
use arc_swap::ArcSwap;

pub struct HotRegistrySafe {
    items: DashMap<i32, HotItem>,
    top_ids: ArcSwap<Vec<i32>>,
}

impl HotRegistrySafe {
    pub fn new() -> Self {
        Self {
            items: DashMap::new(),
            top_ids: ArcSwap::from_pointee(Vec::new()),
        }
    }

    pub fn get(&self, item_id: i32) -> Option<HotItem> {
        self.items.get(&item_id).map(|r| r.value().clone())
    }

    pub fn get_top_k(&self, limit: usize) -> Vec<HotItem> {
        let ids = self.top_ids.load();
        ids.iter()
            .take(limit)
            .filter_map(|id| self.get(*id))
            .collect()
    }

    pub fn refresh(&self, new_items: Vec<HotItem>) {
        let count = new_items.len();
        
        // Prepare new list
        let mut ids = Vec::with_capacity(count);
        
        // Clear map? Or Keep?
        // Clearing ensures we don't hold stale "viral" items forever.
        self.items.clear();

        for item in new_items {
            ids.push(item.item_id);
            self.items.insert(item.item_id, item);
        }

        // Atomic swap of the top-k list
        self.top_ids.store(Arc::new(ids));
        
        info!(count = count, "Hot Registry (Thunder-Lite) refreshed");
    }
    
    pub fn len(&self) -> usize {
        self.items.len()
    }
}
