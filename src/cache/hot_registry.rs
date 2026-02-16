//! "Thunder-Lite" in-memory hot registry for global viral content.
//!
//! # Architecture
//! ```text
//! Pulse Worker (Background) ───> Refresh (every 60s) ───┐
//!                                                       │
//! API Request (Hot Path) ──────> get_top_k ─────────> [HotRegistry] ──> Sub-millisecond return
//!                                                       │
//!                                                       └─> DashMap (Sharded RAM)
//! ```
//!
//! Provides zero-latency access to the top N (e.g., 10,000) items that drive 
//! the majority of platform traffic, effectively skipping Redis and PostgreSQL
//! for the hottest data.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use arc_swap::ArcSwap;
use tracing::info;

/// Represents a "Viral" item cached in memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotItem {
    pub item_id: i32,
    pub score: f32, // Pre-computed trending/popularity score
    pub metadata: JsonValue, // Hydrated metadata (title, url, etc.) to skip DB lookups
}

/// Thread-safe in-memory registry for global hot content.
///
/// Uses a sharded DashMap for O(1) metadata lookups and an ArcSwap-protected
/// vector for atomic top-K list updates.
pub struct HotRegistry {
    /// Full item data indexed by item_id.
    items: DashMap<i32, HotItem>,
    /// Ordered list of item IDs for quick "top k" retrieval without sorting.
    top_ids: ArcSwap<Vec<i32>>,
}

impl HotRegistry {
    /// Create a new empty hot registry.
    pub fn new() -> Self {
        Self {
            items: DashMap::new(),
            top_ids: ArcSwap::from_pointee(Vec::new()),
        }
    }

    /// Get an item if it exists in the hot registry.
    pub fn get(&self, item_id: i32) -> Option<HotItem> {
        self.items.get(&item_id).map(|r| r.value().clone())
    }

    /// Get the top K items directly from memory (zero-latency fetch).
    pub fn get_top_k(&self, limit: usize) -> Vec<HotItem> {
        let ids = self.top_ids.load();
        ids.iter()
            .take(limit)
            .filter_map(|id| self.get(*id))
            .collect()
    }

    /// Atomic refresh of the registry.
    ///
    /// Clears existing items and populates with new viral content.
    /// Swaps the top_ids pointer atomically to ensure zero downtime for readers.
    pub fn refresh(&self, new_items: Vec<HotItem>) {
        let count = new_items.len();
        let mut ids = Vec::with_capacity(count);
        
        // Clear and refill map
        self.items.clear();
        for item in new_items {
            ids.push(item.item_id);
            self.items.insert(item.item_id, item);
        }

        // Atomic swap of the top-k list
        self.top_ids.store(Arc::new(ids));
        
        info!(
            count = count,
            "Thunder-Lite Hot Registry refreshed successfully"
        );
    }
    
    /// Returns the current number of hot items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for HotRegistry {
    fn default() -> Self {
        Self::new()
    }
}
