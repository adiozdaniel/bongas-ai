//! Logic for managing and serving UI page layouts with SDUI support, algorithmic reordering, and Navigation Mesh resolution.

use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use tracing::{info, debug};
use dashmap::DashMap;

use crate::pages::types::{PageLayout, PageSlug, SavePageLayoutRequest, PageCompositionItem, NavType};
use crate::db::repositories::page_layout_repository::PageLayoutRepository;
use crate::error::AppResult;
use crate::ingestion::types::UserActivity;

/// The central manager for UI orchestration, layout resolution, and feedback ranking.
pub struct PagesManager {
    repo: Arc<PageLayoutRepository>,
    /// Cache for frequently accessed page layouts to avoid DB roundtrips.
    cache: Arc<RwLock<LruCache<(PageSlug, Option<String>, Option<String>), PageLayout>>>,
    /// Real-time engagement scores for (Visitor/User, Scenario Slug) to drive row ranking.
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
                UserActivity::Impression { .. } => -0.05,
                _ => 0.0,
            };

            let key = (identity, slug.to_string());
            self.engagement_scores.entry(key)
                .and_modify(|old| *old += score_delta)
                .or_insert(score_delta);
        }
    }

    /// HOT-RELOAD: Load all active pages from database into cache.
    pub async fn load_all_active(&self) -> AppResult<usize> {
        info!("Hydrating enriched Symphony page layout cache...");
        let db_layouts = self.repo.find_all_active().await?;
        let count = db_layouts.len();

        let mut cache = self.cache.write().await;
        for db_row in db_layouts {
            let page_slug = PageSlug(db_row.page_slug.clone());
            let composition: Vec<PageCompositionItem> = serde_json::from_value(db_row.composition)
                .unwrap_or_default();
            
            let nav_type = match db_row.nav_type.as_str() {
                "main" => NavType::Main,
                "sub" => NavType::Sub,
                _ => NavType::Hidden,
            };

            let layout = PageLayout {
                page_slug: page_slug.clone(),
                is_landing: db_row.is_landing,
                nav_type,
                device_type: db_row.device_type.clone(),
                maturity_rating: db_row.maturity_rating.clone(),
                priority: db_row.priority,
                composition,
                is_active: db_row.is_active,
                updated_at: db_row.updated_at,
            };
            cache.put((page_slug, db_row.device_type, db_row.maturity_rating), layout);
        }

        Ok(count)
    }

    /// List all currently active page layouts.
    pub async fn list_active_pages(&self) -> AppResult<Vec<PageLayout>> {
        let db_layouts = self.repo.find_all_active().await?;
        Ok(db_layouts.into_iter().map(|r| {
            let composition: Vec<PageCompositionItem> = serde_json::from_value(r.composition).unwrap_or_default();
            let nav_type = match r.nav_type.as_str() {
                "main" => NavType::Main,
                "sub" => NavType::Sub,
                _ => NavType::Hidden,
            };
            PageLayout {
                page_slug: PageSlug(r.page_slug),
                is_landing: r.is_landing,
                nav_type,
                device_type: r.device_type,
                maturity_rating: r.maturity_rating,
                priority: r.priority,
                composition,
                is_active: r.is_active,
                updated_at: r.updated_at,
            }
        }).collect())
    }

    /// Resolve the best landing page for a user's context.
    pub async fn get_landing_page_contextual(
        &self,
        device_type: Option<&str>,
        maturity_rating: Option<&str>,
        identity_key: Option<&str>
    ) -> AppResult<Option<PageLayout>> {
        info!(device = ?device_type, maturity = ?maturity_rating, "Resolving Genesis landing page");
        let db_layout = self.repo.find_landing_page(device_type, maturity_rating).await?;
        
        if let Some(layout_row) = db_layout {
            return self.get_layout_contextual(&layout_row.page_slug, device_type, maturity_rating, identity_key).await;
        }
        
        Ok(None)
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
            if let Some(db_layout) = self.repo.find_best_match(slug, device_type, maturity_rating).await? {
                let composition: Vec<PageCompositionItem> = serde_json::from_value(db_layout.composition)
                    .unwrap_or_default();
                
                let nav_type = match db_layout.nav_type.as_str() {
                    "main" => NavType::Main,
                    "sub" => NavType::Sub,
                    _ => NavType::Hidden,
                };

                let resolved = PageLayout {
                    page_slug: page_slug.clone(),
                    is_landing: db_layout.is_landing,
                    nav_type,
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

    fn reorder_composition(&self, composition: &mut Vec<PageCompositionItem>, identity: &str) {
        if composition.len() < 2 { return; }
        composition.sort_by(|a, b| {
            let score_a = self.engagement_scores.get(&(identity.to_string(), a.slug.clone())).map(|v| *v).unwrap_or(0.0);
            let score_b = self.engagement_scores.get(&(identity.to_string(), b.slug.clone())).map(|v| *v).unwrap_or(0.0);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
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
        let nav_type_str = serde_json::to_value(&req.nav_type.clone().unwrap_or_default())
            .unwrap_or_default().as_str().unwrap_or("hidden").to_string();

        let db_row = self.repo.upsert(
            &req.page_slug, 
            req.is_landing.unwrap_or(false),
            &nav_type_str,
            composition_json, 
            req.device_type.clone(), 
            req.maturity_rating.clone(), 
            req.priority.unwrap_or(0), 
            req.is_active.unwrap_or(true)
        ).await?;

        let layout = PageLayout {
            page_slug: PageSlug(req.page_slug.clone()),
            is_landing: db_row.is_landing,
            nav_type: req.nav_type.unwrap_or_default(),
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
