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

use std::collections::HashMap;
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

/// Internal data for the hot registry.
struct RegistryData {
    /// Full item data indexed by item_id.
    items: HashMap<i32, HotItem>,
    /// Ordered list of item IDs for quick "top k" retrieval without sorting.
    top_ids: Vec<i32>,
}

/// Thread-safe in-memory registry for global hot content.
///
/// Uses an ArcSwap-protected structure for atomic updates, ensuring 
/// zero availability gaps for readers during refresh.
pub struct HotRegistry {
    data: ArcSwap<RegistryData>,
}

impl HotRegistry {
    /// Create a new empty hot registry.
    pub fn new() -> Self {
        Self {
            data: ArcSwap::from_pointee(RegistryData {
                items: HashMap::new(),
                top_ids: Vec::new(),
            }),
        }
    }

    /// Get an item if it exists in the hot registry.
    pub fn get(&self, item_id: i32) -> Option<HotItem> {
        self.data.load().items.get(&item_id).cloned()
    }

    /// Get the top K items directly from memory (zero-latency fetch).
    pub fn get_top_k(&self, limit: usize) -> Vec<HotItem> {
        let data = self.data.load();
        data.top_ids.iter()
            .take(limit)
            .filter_map(|id| data.items.get(id).cloned())
            .collect()
    }

    /// Atomic refresh of the registry.
    ///
    /// Populates with new viral content and performs a single atomic swap.
    pub fn refresh(&self, new_items: Vec<HotItem>) {
        let count = new_items.len();
        let mut items = HashMap::with_capacity(count);
        let mut top_ids = Vec::with_capacity(count);
        
        for item in new_items {
            top_ids.push(item.item_id);
            items.insert(item.item_id, item);
        }

        // Atomic swap of the entire data structure
        self.data.store(Arc::new(RegistryData {
            items,
            top_ids,
        }));
        
        info!(
            count = count,
            "Thunder-Lite Hot Registry refreshed successfully (Atomic Swap)"
        );
    }
    
    /// Returns the current number of hot items.
    pub fn len(&self) -> usize {
        self.data.load().items.len()
    }

    /// Returns true if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.data.load().items.is_empty()
    }
}

impl Default for HotRegistry {
    fn default() -> Self {
        Self::new()
    }
}
