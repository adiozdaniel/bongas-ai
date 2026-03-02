//! Logic for managing and serving UI page layouts.

use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use tracing::info;

use crate::pages::types::{PageLayout, PageSlug, SavePageLayoutRequest};
use crate::db::repositories::page_layout_repository::PageLayoutRepository;
use crate::error::AppResult;

/// The central manager for UI orchestration.
pub struct PagesManager {
    repo: Arc<PageLayoutRepository>,
    /// Cache for frequently accessed page layouts to avoid DB roundtrips during streaming.
    cache: Arc<RwLock<LruCache<PageSlug, PageLayout>>>,
}

impl PagesManager {
    /// Create a new PagesManager with a default cache size.
    pub fn new(repo: Arc<PageLayoutRepository>) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(100).unwrap()))),
        }
    }

    /// HOT-RELOAD: Load all active pages from database into cache.
    pub async fn load_all_active(&self) -> AppResult<usize> {
        info!("Hydrating page layout cache from database...");
        let layouts = self.repo.find_all_active().await?;
        let count = layouts.len();

        let mut cache = self.cache.write().await;
        for db_layout in layouts {
            let page_slug = PageSlug(db_layout.page_slug.clone());
            let scenario_slugs: Vec<String> = serde_json::from_value(db_layout.scenario_slugs)
                .unwrap_or_default();
            
            let layout = PageLayout {
                page_slug: page_slug.clone(),
                scenario_slugs,
                is_active: db_layout.is_active,
                updated_at: db_layout.updated_at,
            };
            cache.put(page_slug, layout);
        }

        info!(count = count, "Page layout cache hydrated");
        Ok(count)
    }

    /// List all currently active page layouts.
    pub async fn list_active_pages(&self) -> AppResult<Vec<PageLayout>> {
        let db_layouts = self.repo.find_all_active().await?;
        
        let layouts = db_layouts.into_iter().map(|layout_row| {
            let scenario_slugs: Vec<String> = serde_json::from_value(layout_row.scenario_slugs)
                .unwrap_or_default();
            
            PageLayout {
                page_slug: PageSlug(layout_row.page_slug),
                scenario_slugs,
                is_active: layout_row.is_active,
                updated_at: layout_row.updated_at,
            }
        }).collect();

        Ok(layouts)
    }

    /// Retrieve the layout for a specific page.
    pub async fn get_layout(&self, slug: &str) -> AppResult<Option<PageLayout>> {
        let page_slug = PageSlug(slug.to_string());

        // 1. Try cache first
        {
            let mut cache = self.cache.write().await;
            if let Some(layout) = cache.get(&page_slug) {
                return Ok(Some(layout.clone()));
            }
        }

        // 2. Fetch from repository
        info!(page = %slug, "Cache miss for page layout, fetching from DB");
        let db_layout = self.repo.find_by_slug(slug).await?;

        if let Some(layout_row) = db_layout {
            let scenario_slugs: Vec<String> = serde_json::from_value(layout_row.scenario_slugs)
                .unwrap_or_default();
            
            let layout = PageLayout {
                page_slug: page_slug.clone(),
                scenario_slugs,
                is_active: layout_row.is_active,
                updated_at: layout_row.updated_at,
            };

            // 3. Hydrate cache
            let mut cache = self.cache.write().await;
            cache.put(page_slug, layout.clone());
            
            return Ok(Some(layout));
        }

        Ok(None)
    }

    /// Invalidate a specific page layout in the cache.
    pub async fn invalidate_cache(&self, slug: &str) {
        let mut cache = self.cache.write().await;
        cache.pop(&PageSlug(slug.to_string()));
    }

    /// Soft-delete a page layout and invalidate cache.
    pub async fn delete_layout(&self, slug: &str) -> AppResult<bool> {
        let success = self.repo.soft_delete(slug).await?;
        if success {
            self.invalidate_cache(slug).await;
        }
        Ok(success)
    }

    /// Create or update a page layout.
    pub async fn save_layout(&self, req: SavePageLayoutRequest) -> AppResult<PageLayout> {
        let is_active = req.is_active.unwrap_or(true);
        let scenario_slugs = serde_json::to_value(&req.scenario_slugs)
            .map_err(|e| crate::error::AppError::Internal(format!("Failed to serialize scenario slugs: {}", e)))?;

        let db_row = self.repo.upsert(&req.page_slug, scenario_slugs, is_active).await?;
        
        let page_slug = PageSlug(req.page_slug.clone());
        let layout = PageLayout {
            page_slug: page_slug.clone(),
            scenario_slugs: req.scenario_slugs,
            is_active: db_row.is_active,
            updated_at: db_row.updated_at,
        };

        // Invalidate cache
        {
            let mut cache = self.cache.write().await;
            cache.put(page_slug, layout.clone());
        }

        Ok(layout)
    }
}
