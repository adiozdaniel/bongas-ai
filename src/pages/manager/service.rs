//! Logic for managing and serving UI page layouts with SDUI support.

use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use tracing::{info, debug};

use crate::pages::types::{PageLayout, PageSlug, SavePageLayoutRequest, PageCompositionItem};
use crate::db::repositories::page_layout_repository::PageLayoutRepository;
use crate::error::AppResult;

/// The central manager for UI orchestration and layout resolution.
pub struct PagesManager {
    repo: Arc<PageLayoutRepository>,
    /// Cache for frequently accessed page layouts to avoid DB roundtrips.
    /// Note: Cache is keyed by (Slug, Device, Maturity) for accurate resolution.
    cache: Arc<RwLock<LruCache<(PageSlug, Option<String>, Option<String>), PageLayout>>>,
}

impl PagesManager {
    /// Create a new PagesManager with a default cache size.
    pub fn new(repo: Arc<PageLayoutRepository>) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap()))),
        }
    }

    /// HOT-RELOAD: Load all active pages from database into cache.
    pub async fn load_all_active(&self) -> AppResult<usize> {
        info!("Hydrating enriched page layout cache from database...");
        let layouts = self.repo.find_all_active().await?;
        let count = layouts.len();

        let mut cache = self.cache.write().await;
        for db_layout in layouts {
            let page_slug = PageSlug(db_layout.page_slug.clone());
            let composition: Vec<PageCompositionItem> = serde_json::from_value(db_layout.composition)
                .unwrap_or_default();
            
            let layout = PageLayout {
                page_slug: page_slug.clone(),
                device_type: db_layout.device_type.clone(),
                maturity_rating: db_layout.maturity_rating.clone(),
                priority: db_layout.priority,
                composition,
                is_active: db_layout.is_active,
                updated_at: db_layout.updated_at,
            };
            cache.put((page_slug, db_layout.device_type, db_layout.maturity_rating), layout);
        }

        info!(count = count, "SDUI page layout cache hydrated");
        Ok(count)
    }

    /// List all currently active page layouts.
    pub async fn list_active_pages(&self) -> AppResult<Vec<PageLayout>> {
        let db_layouts = self.repo.find_all_active().await?;
        
        let layouts = db_layouts.into_iter().map(|layout_row| {
            let composition: Vec<PageCompositionItem> = serde_json::from_value(layout_row.composition)
                .unwrap_or_default();
            
            PageLayout {
                page_slug: PageSlug(layout_row.page_slug),
                device_type: layout_row.device_type,
                maturity_rating: layout_row.maturity_rating,
                priority: layout_row.priority,
                composition,
                is_active: layout_row.is_active,
                updated_at: layout_row.updated_at,
            }
        }).collect();

        Ok(layouts)
    }

    /// Retrieve the best matching layout for a specific page and context.
    pub async fn get_layout_contextual(
        &self, 
        slug: &str, 
        device_type: Option<&str>, 
        maturity_rating: Option<&str>
    ) -> AppResult<Option<PageLayout>> {
        let page_slug = PageSlug(slug.to_string());
        let device = device_type.map(|s| s.to_string());
        let maturity = maturity_rating.map(|s| s.to_string());

        // 1. Try cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(layout) = cache.get(&(page_slug.clone(), device.clone(), maturity.clone())) {
                debug!(page = %slug, "Cache hit for contextual page layout");
                return Ok(Some(layout.clone()));
            }
        }

        // 2. Fetch from repository using Targeting Resolver
        info!(page = %slug, device = ?device_type, maturity = ?maturity_rating, "Cache miss, resolving best layout match from DB");
        let db_layout = self.repo.find_best_match(slug, device_type, maturity_rating).await?;

        if let Some(layout_row) = db_layout {
            let composition: Vec<PageCompositionItem> = serde_json::from_value(layout_row.composition)
                .unwrap_or_default();
            
            let layout = PageLayout {
                page_slug: page_slug.clone(),
                device_type: layout_row.device_type.clone(),
                maturity_rating: layout_row.maturity_rating.clone(),
                priority: layout_row.priority,
                composition,
                is_active: layout_row.is_active,
                updated_at: layout_row.updated_at,
            };

            // 3. Hydrate cache with exact match
            let mut cache = self.cache.write().await;
            cache.put((page_slug, device, maturity), layout.clone());
            
            return Ok(Some(layout));
        }

        Ok(None)
    }

    /// Invalidate cache for a specific slug.
    pub async fn invalidate_cache(&self, slug: &str) {
        let mut cache = self.cache.write().await;
        // Invalidate all variants of this slug
        let keys_to_remove: Vec<_> = cache.iter()
            .filter(|((s, _, _), _)| s.0 == slug)
            .map(|(k, _)| k.clone())
            .collect();
        
        for key in keys_to_remove {
            cache.pop(&key);
        }
    }

    /// Soft-delete a page layout and invalidate cache.
    pub async fn delete_layout(&self, slug: &str) -> AppResult<bool> {
        let success = self.repo.soft_delete(slug).await?;
        if success {
            self.invalidate_cache(slug).await;
        }
        Ok(success)
    }

    /// Create or update an enriched page layout.
    pub async fn save_layout(&self, req: SavePageLayoutRequest) -> AppResult<PageLayout> {
        let is_active = req.is_active.unwrap_or(true);
        let priority = req.priority.unwrap_or(0);
        let composition_json = serde_json::to_value(&req.composition)
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to serialize composition: {}", e)))?;

        let db_row = self.repo.upsert(
            &req.page_slug, 
            composition_json, 
            req.device_type.clone(), 
            req.maturity_rating.clone(), 
            priority, 
            is_active
        ).await?;
        
        let page_slug = PageSlug(req.page_slug.clone());
        let layout = PageLayout {
            page_slug: page_slug.clone(),
            device_type: db_row.device_type,
            maturity_rating: db_row.maturity_rating,
            priority: db_row.priority,
            composition: req.composition,
            is_active: db_row.is_active,
            updated_at: db_row.updated_at,
        };

        self.invalidate_cache(&req.page_slug).await;

        Ok(layout)
    }
}
