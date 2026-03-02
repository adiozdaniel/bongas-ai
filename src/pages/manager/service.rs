//! Logic for managing and serving UI page layouts with SDUI support and algorithmic reordering.

use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use tracing::{info, debug};
use dashmap::DashMap;

use crate::pages::types::{PageLayout, PageSlug, SavePageLayoutRequest, PageCompositionItem};
use crate::db::repositories::page_layout_repository::PageLayoutRepository;
use crate::error::AppResult;
use crate::ingestion::types::UserActivity;

/// The central manager for UI orchestration, layout resolution, and feedback ranking.
pub struct PagesManager {
    repo: Arc<PageLayoutRepository>,
    /// Cache for frequently accessed page layouts to avoid DB roundtrips.
    cache: Arc<RwLock<LruCache<(PageSlug, Option<String>, Option<String>), PageLayout>>>,
    /// Real-time engagement scores for (Visitor/User, Scenario Slug) to drive row ranking.
    /// Key: (Identity_Key, Scenario_Slug), Value: Score
    engagement_scores: Arc<DashMap<(String, String), f32>>,
}

impl PagesManager {
    /// Create a new PagesManager with a default cache size.
    pub fn new(repo: Arc<PageLayoutRepository>) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap()))),
            engagement_scores: Arc::new(DashMap::new()),
        }
    }

    /// Process incoming user activities to update internal engagement scores (Feedback Loop).
    pub async fn process_activity(&self, activity: &UserActivity) {
        let identity = if let Some(vid) = activity.visitor_id() {
            vid.to_string()
        } else {
            activity.user_id().to_string()
        };

        if let Some(slug) = activity.scenario_slug() {
            let score_delta = match activity {
                UserActivity::Click { .. } => 1.0,
                UserActivity::Playback { watch_percentage, .. } => *watch_percentage,
                UserActivity::Reaction { reaction_type, .. } => {
                    if reaction_type == "like" { 2.0 } else { -2.0 }
                }
                UserActivity::Impression { .. } => -0.05, // Slight decay for ignored rows
                _ => 0.0,
            };

            let key = (identity, slug.to_string());
            self.engagement_scores.entry(key)
                .and_modify(|old| *old += score_delta)
                .or_insert(score_delta);
        }
    }

    /// Retrieve the best matching layout for a specific page and context, 
    /// with algorithmic reordering based on engagement.
    pub async fn get_layout_contextual(
        &self, 
        slug: &str, 
        device_type: Option<&str>, 
        maturity_rating: Option<&str>,
        identity_key: Option<&str>
    ) -> AppResult<Option<PageLayout>> {
        let page_slug = PageSlug(slug.to_string());
        let device = device_type.map(|s| s.to_string());
        let maturity = maturity_rating.map(|s| s.to_string());

        // 1. Try cache first
        let layout_from_cache = {
            let mut cache = self.cache.write().await;
            cache.get(&(page_slug.clone(), device.clone(), maturity.clone())).cloned()
        };

        // 2. Fetch from repository if cache miss
        let layout = if let Some(l) = layout_from_cache {
            Some(l)
        } else {
            info!(page = %slug, device = ?device_type, maturity = ?maturity_rating, "Contextual resolution for page layout");
            if let Some(db_layout) = self.repo.find_best_match(slug, device_type, maturity_rating).await? {
                let composition: Vec<PageCompositionItem> = serde_json::from_value(db_layout.composition)
                    .unwrap_or_default();
                
                let resolved = PageLayout {
                    page_slug: page_slug.clone(),
                    device_type: db_layout.device_type.clone(),
                    maturity_rating: db_layout.maturity_rating.clone(),
                    priority: db_layout.priority,
                    composition,
                    is_active: db_layout.is_active,
                    updated_at: db_layout.updated_at,
                };

                let mut cache = self.cache.write().await;
                cache.put((page_slug, device, maturity), resolved.clone());
                Some(resolved)
            } else {
                None
            }
        };

        // 3. Algorithmic Reordering (The Brain)
        if let (Some(mut l), Some(id)) = (layout.clone(), identity_key) {
            self.reorder_composition(&mut l.composition, id);
            return Ok(Some(l));
        }

        Ok(layout)
    }

    /// Perform algorithmic reordering of page rows based on engagement scores.
    fn reorder_composition(&self, composition: &mut Vec<PageCompositionItem>, identity: &str) {
        if composition.len() < 2 { return; }

        composition.sort_by(|a, b| {
            let score_a = self.engagement_scores.get(&(identity.to_string(), a.slug.clone()))
                .map(|v| *v).unwrap_or(0.0);
            let score_b = self.engagement_scores.get(&(identity.to_string(), b.slug.clone()))
                .map(|v| *v).unwrap_or(0.0);
            
            // Sort descending: highest engagement first
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        debug!(identity, "Layout reordered based on engagement scores");
    }

    /// HOT-RELOAD: Load all active pages from database into cache.
    pub async fn load_all_active(&self) -> AppResult<usize> {
        info!("Hydrating enriched page layout cache...");
        let layouts = self.repo.find_all_active().await?;
        let count = layouts.len();

        let mut cache = self.cache.write().await;
        for db_layout in layouts {
            let composition: Vec<PageCompositionItem> = serde_json::from_value(db_layout.composition)
                .unwrap_or_default();
            
            let layout = PageLayout {
                page_slug: PageSlug(db_layout.page_slug.clone()),
                device_type: db_layout.device_type.clone(),
                maturity_rating: db_layout.maturity_rating.clone(),
                priority: db_layout.priority,
                composition,
                is_active: db_layout.is_active,
                updated_at: db_layout.updated_at,
            };
            cache.put((PageSlug(db_layout.page_slug), db_layout.device_type, db_layout.maturity_rating), layout);
        }

        Ok(count)
    }

    pub async fn list_active_pages(&self) -> AppResult<Vec<PageLayout>> {
        let db_layouts = self.repo.find_all_active().await?;
        Ok(db_layouts.into_iter().map(|r| {
            let composition: Vec<PageCompositionItem> = serde_json::from_value(r.composition).unwrap_or_default();
            PageLayout {
                page_slug: PageSlug(r.page_slug),
                device_type: r.device_type,
                maturity_rating: r.maturity_rating,
                priority: r.priority,
                composition,
                is_active: r.is_active,
                updated_at: r.updated_at,
            }
        }).collect())
    }

    pub async fn invalidate_cache(&self, slug: &str) {
        let mut cache = self.cache.write().await;
        let keys: Vec<_> = cache.iter().filter(|((s, _, _), _)| s.0 == slug).map(|(k, _)| k.clone()).collect();
        for k in keys { cache.pop(&k); }
    }

    pub async fn delete_layout(&self, slug: &str) -> AppResult<bool> {
        let success = self.repo.soft_delete(slug).await?;
        if success { self.invalidate_cache(slug).await; }
        Ok(success)
    }

    pub async fn save_layout(&self, req: SavePageLayoutRequest) -> AppResult<PageLayout> {
        let composition_json = serde_json::to_value(&req.composition).unwrap_or_default();
        let db_row = self.repo.upsert(&req.page_slug, composition_json, req.device_type.clone(), req.maturity_rating.clone(), req.priority.unwrap_or(0), req.is_active.unwrap_or(true)).await?;
        let layout = PageLayout {
            page_slug: PageSlug(req.page_slug.clone()),
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
