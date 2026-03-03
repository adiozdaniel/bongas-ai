//! Logic for managing and serving UI page layouts with SDUI support, algorithmic reordering, and Navigation Mesh resolution.

use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use tracing::{info, debug};
use dashmap::DashMap;
use std::collections::HashMap;

use crate::pages::types::{PageLayout, PageSlug, SavePageLayoutRequest, PageCompositionItem, NavType};
use crate::db::repositories::page_layout_repository::PageLayoutRepository;
use crate::error::AppResult;
use crate::ingestion::types::UserActivity;

/// Represents the assembled application structure.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NavigationMesh {
    pub main: Vec<PageLayout>,
    pub sub: Vec<PageLayout>,
}

/// The central manager for UI orchestration, layout resolution, and feedback ranking.
pub struct PagesManager {
    repo: Arc<PageLayoutRepository>,
    /// Cache for frequently accessed page layouts to avoid DB roundtrips.
    cache: Arc<RwLock<LruCache<(PageSlug, Option<String>, Option<String>), PageLayout>>>,
    /// Optimized map for landing page resolution: (Device, Maturity) -> PageLayout
    landing_pages: Arc<RwLock<HashMap<(String, String), PageLayout>>>,
    /// Pre-calculated navigation mesh (Main + Sub pages)
    nav_mesh: Arc<RwLock<NavigationMesh>>,
    /// Real-time engagement scores for (Visitor/User, Scenario Slug or Page Slug) to drive ranking.
    engagement_scores: Arc<DashMap<(String, String), f32>>,
}

impl PagesManager {
    /// Create a new PagesManager with a default cache size.
    pub fn new(repo: Arc<PageLayoutRepository>) -> Self {
        Self {
            repo,
            cache: Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(200).unwrap()))),
            landing_pages: Arc::new(RwLock::new(HashMap::new())),
            nav_mesh: Arc::new(RwLock::new(NavigationMesh { main: vec![], sub: vec![] })),
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

    /// HOT-RELOAD: Load all active pages and pre-calculate the Navigation Mesh.
    pub async fn load_all_active(&self) -> AppResult<usize> {
        info!("Hydrating Navigation Mesh and Page layouts...");
        let db_layouts = self.repo.find_all_active().await?;
        let count = db_layouts.len();

        let mut main_nav = Vec::new();
        let mut sub_nav = Vec::new();
        let mut landings = HashMap::new();
        let mut page_cache = LruCache::new(NonZeroUsize::new(200).unwrap());

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
                nav_type: nav_type.clone(),
                device_type: db_row.device_type.clone(),
                maturity_rating: db_row.maturity_rating.clone(),
                priority: db_row.priority,
                composition,
                is_active: db_row.is_active,
                updated_at: db_row.updated_at,
            };

            // 1. Update Global Page Cache
            page_cache.put((page_slug, db_row.device_type.clone(), db_row.maturity_rating.clone()), layout.clone());

            // 2. Track Landing Pages
            if db_row.is_landing {
                let device = db_row.device_type.clone().unwrap_or_else(|| "all".to_string());
                let maturity = db_row.maturity_rating.clone().unwrap_or_else(|| "all".to_string());
                landings.insert((device, maturity), layout.clone());
            }

            // 3. Assemble Nav-Mesh
            match nav_type {
                NavType::Main => main_nav.push(layout),
                NavType::Sub => sub_nav.push(layout),
                _ => {}
            }
        }

        // Atomic Swaps
        {
            let mut cache = self.cache.write().await;
            *cache = page_cache;
        }
        {
            let mut l_cache = self.landing_pages.write().await;
            *l_cache = landings;
        }
        {
            let mut mesh = self.nav_mesh.write().await;
            *mesh = NavigationMesh { main: main_nav, sub: sub_nav };
        }

        info!(count, "Symphony Navigation Mesh hydrated successfully");
        Ok(count)
    }

    /// Retrieve the personalized Navigation Mesh for a user context.
    pub async fn get_nav_mesh_contextual(&self, identity_key: Option<&str>) -> NavigationMesh {
        let mut mesh = self.nav_mesh.read().await.clone();
        
        if let Some(id) = identity_key {
            // Rank Sub-Navigation Hubs based on engagement
            mesh.sub.sort_by(|a, b| {
                let score_a = self.engagement_scores.get(&(id.to_string(), a.page_slug.0.clone()))
                    .map(|v| *v).unwrap_or(0.0);
                let score_b = self.engagement_scores.get(&(id.to_string(), b.page_slug.0.clone()))
                    .map(|v| *v).unwrap_or(0.0);
                
                score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        
        mesh
    }

    /// Resolve the best landing page for a user's context (Zero-DB path).
    pub async fn get_landing_page_contextual(
        &self,
        device_type: Option<&str>,
        maturity_rating: Option<&str>,
        identity_key: Option<&str>
    ) -> AppResult<Option<PageLayout>> {
        let device = device_type.unwrap_or("all");
        let maturity = maturity_rating.unwrap_or("all");

        let landing = {
            let landings = self.landing_pages.read().await;
            
            // Hierarchical resolution:
            // 1. Exact match
            // 2. Device match + global maturity
            // 3. Global default
            landings.get(&(device.to_string(), maturity.to_string()))
                .or_else(|| landings.get(&(device.to_string(), "all".to_string())))
                .or_else(|| landings.get(&("all".to_string(), "all".to_string())))
                .cloned()
        };

        if let (Some(mut l), Some(id)) = (landing, identity_key) {
            self.reorder_composition(&mut l.composition, id);
            return Ok(Some(l));
        }
        
        Ok(landing)
    }

    /// Retrieve the best matching layout for a specific page and context.
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

        // 2. Fetch from repository if cache miss (and hydrate)
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
        if success { 
            self.load_all_active().await?; // Full refresh for Nav-Mesh integrity
        }
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
        
        self.load_all_active().await?; // Full refresh for Nav-Mesh integrity

        Ok(layout)
    }
}
