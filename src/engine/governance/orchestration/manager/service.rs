//! Logic for managing and serving UI page layouts with SDUI support, algorithmic reordering, and Navigation Mesh resolution.

use std::sync::Arc;
use tokio::sync::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use tracing::{info, debug};
use std::collections::HashMap;

use crate::engine::governance::orchestration::types::models::{PageLayout, PageSlug, SavePageLayoutRequest, NavType};
use crate::db::repositories::page_layout_repository::service::PageLayoutRepository;
use crate::db::repositories::interaction_repository::service::InteractionRepository;
use crate::engine::governance::factory::scenarios_manager::service::ScenariosManager;
use crate::ml::inference::features::service::FeatureStore;
use crate::error::AppResult;
use crate::ingestion::types::UserActivity;

/// Represents the assembled application structure.
/// Orchestrates the relationship between navigation mesh and page compositions.
pub struct PagesManager {
    repo: Arc<PageLayoutRepository>,
    interaction_repo: Arc<InteractionRepository>,
    feature_store: Arc<FeatureStore>,
    scenarios: Arc<ScenariosManager>,
    /// High-performance L1 cache for resolved layouts (target-aware)
    cache: Arc<RwLock<LruCache<(PageSlug, Option<String>, Option<String>), PageLayout>>>,
    /// In-memory landing page registry for ultra-fast genesis resolution
    landing_pages: Arc<RwLock<HashMap<(String, String), PageLayout>>>,
    /// Shared navigation mesh (cached globally)
    nav_mesh: Arc<RwLock<Vec<PageLayout>>>,
}

impl PagesManager {
    pub fn new(
        repo: Arc<PageLayoutRepository>, 
        interaction_repo: Arc<InteractionRepository>,
        feature_store: Arc<FeatureStore>, 
        scenarios: Arc<ScenariosManager>,
        cache_size: usize
    ) -> Self {
        Self {
            repo,
            interaction_repo,
            feature_store,
            scenarios,
            cache: Arc::new(RwLock::new(LruCache::new(NonZeroUsize::new(cache_size).unwrap()))),
            landing_pages: Arc::new(RwLock::new(HashMap::new())),
            nav_mesh: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn map_db_to_domain(db: crate::db::PageLayout) -> PageLayout {
        PageLayout {
            page_slug: PageSlug(db.page_slug),
            is_landing: db.is_landing,
            nav_type: match db.nav_type.as_str() {
                "main" => NavType::Main,
                "sub" => NavType::Sub,
                _ => NavType::Hidden,
            },
            device_type: db.device_type,
            maturity_rating: db.maturity_rating,
            priority: db.priority,
            composition: serde_json::from_value(db.composition).unwrap_or_default(),
            is_active: db.is_active,
            updated_at: db.updated_at,
        }
    }

    /// Load all active layouts into memory for genesis and nav-mesh resolution.
    pub async fn load_all_active(&self) -> AppResult<usize> {
        info!("Hydrating PagesManager: Loading all active layouts into memory...");
        
        let layouts = self.repo.find_all_active().await?;
        let count = layouts.len();

        let mut landing_map = HashMap::new();
        let mut nav_list = Vec::new();

        for db_layout in layouts {
            let layout = Self::map_db_to_domain(db_layout);
            
            if layout.is_landing {
                let device = layout.device_type.clone().unwrap_or_else(|| "all".to_string());
                let maturity = layout.maturity_rating.clone().unwrap_or_else(|| "all".to_string());
                landing_map.insert((device, maturity), layout.clone());
            }
            
            if layout.nav_type != NavType::Hidden {
                nav_list.push(layout);
            }
        }

        // Atomic Swaps
        {
            let mut landings = self.landing_pages.write().await;
            *landings = landing_map;
        }
        {
            let mut nav = self.nav_mesh.write().await;
            *nav = nav_list;
        }

        // Clear L1 cache on full reload to ensure consistency
        {
            let mut cache = self.cache.write().await;
            cache.clear();
        }

        info!(count, "PagesManager hydration complete");
        Ok(count)
    }

    /// Resolve the navigation mesh for a specific context.
    pub async fn get_nav_mesh_contextual(&self, identity_key: Option<&str>) -> Vec<crate::api::SymphonyNavigation> {
        let nav = self.nav_mesh.read().await;
        
        let mut symphony_nav: Vec<crate::api::SymphonyNavigation> = nav.iter().map(|l| crate::api::SymphonyNavigation {
            slug: l.page_slug.0.clone(),
            title: l.page_slug.0.replace('_', " "), 
            nav_type: match l.nav_type {
                NavType::Main => "main".to_string(),
                NavType::Sub => "sub".to_string(),
                NavType::Hidden => "hidden".to_string(),
            },
            nav_mesh: vec![],
            landing_slug: "".to_string(),
            total_rows: 0,
            request_id: "".to_string(),
        }).collect();

        // 🧠 Step 1: ML-Ranked Navigation Mesh
        if let Some(vid) = identity_key {
            if let Ok(affinities) = self.feature_store.get_visitor_affinities(vid).await {
                if !affinities.is_empty() {
                    // Sort 'sub' navigation hubs by affinity score
                    symphony_nav.sort_by(|a, b| {
                        if a.nav_type == "sub" && b.nav_type == "sub" {
                            let score_a = affinities.get(&a.slug).copied().unwrap_or(0.0);
                            let score_b = affinities.get(&b.slug).copied().unwrap_or(0.0);
                            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                        } else {
                            std::cmp::Ordering::Equal
                        }
                    });
                }
            }
        }

        symphony_nav
    }

    /// Resolve the landing page for a device/maturity context.
    pub async fn get_landing_page_contextual(
        &self,
        device_type: Option<&str>,
        maturity_rating: Option<&str>,
        _identity_key: Option<&str>
    ) -> AppResult<Option<PageLayout>> {
        let device = device_type.unwrap_or("all");
        let maturity = maturity_rating.unwrap_or("all");

        let landing = {
            let landings: tokio::sync::RwLockReadGuard<'_, HashMap<(String, String), PageLayout>> = self.landing_pages.read().await;
            
            landings.get(&(device.to_string(), maturity.to_string()))
                .or_else(|| landings.get(&(device.to_string(), "all".to_string())))
                .or_else(|| landings.get(&("all".to_string(), "all".to_string())))
                .cloned()
        };

        Ok(landing)
    }

    /// Resolve a full page layout by slug and context (with L1 caching).
    pub async fn get_layout_contextual(
        &self,
        page_slug: &str,
        device_type: Option<&str>,
        maturity_rating: Option<&str>,
        identity_key: Option<&str>
    ) -> AppResult<Option<PageLayout>> {
        let slug = PageSlug(page_slug.to_string());
        let device = device_type.map(|s| s.to_string());
        let maturity = maturity_rating.map(|s| s.to_string());

        // 1. Try cache first
        let layout_from_cache = {
            let mut cache = self.cache.write().await;
            cache.get(&(slug.clone(), device.clone(), maturity.clone())).cloned()
        };

        let mut layout = if let Some(cached) = layout_from_cache {
            Some(cached)
        } else if let Some(db_row) = self.repo.find_best_match(page_slug, device_type, maturity_rating).await? {
            let resolved = Self::map_db_to_domain(db_row);
            // Update cache
            {
                let mut cache = self.cache.write().await;
                cache.put((slug, device, maturity), resolved.clone());
            }
            Some(resolved)
        } else {
            None
        };

        // 🛡️ Phase 3: Zero-Touch Security & Intelligence
        if let Some(ref mut l) = layout {
            // Step 2: KFCB Early-Block (Maturity Ceiling)
            // If profile_id is missing, we assume Adult (18+)
            let profile_rating = maturity_rating.unwrap_or("18");
            
            let mut safe_composition = Vec::new();
            for item in l.composition.drain(..) {
                let scenario_rating = self.scenarios.get_scenario_rating(&item.slug).await.unwrap_or("G".to_string());
                
                if self.is_rating_allowed(profile_rating, &scenario_rating) {
                    safe_composition.push(item);
                } else {
                    debug!(
                        scenario = %item.slug, 
                        profile_rating = %profile_rating, 
                        scenario_rating = %scenario_rating, 
                        "KFCB Early-Block: Scenario restricted by maturity ceiling"
                    );
                    // Replace with fallback if available
                    if let Some(fallback) = item.fallback_slug {
                        safe_composition.push(crate::engine::governance::orchestration::types::models::PageCompositionItem {
                            slug: fallback,
                            fallback_slug: None,
                            row_type: item.row_type,
                            row_style: item.row_style,
                        });
                    }
                }
            }
            l.composition = safe_composition;

            // Step 3: Algorithmic Reordering (Engagement Feedback Loop)
            if let Some(vid) = identity_key {
                // Fetch engagement scores for all scenarios in the composition
                // We use None for user_id to prioritize visitor-level engagement if provided as key
                if let Ok(engagement) = self.interaction_repo.get_scenario_engagement_scores(None, Some(vid)).await {
                    if !engagement.is_empty() {
                        // Perform a STABLE SORT to bubble up high-engagement rows
                        l.composition.sort_by(|a, b| {
                            let score_a = engagement.get(&a.slug).copied().unwrap_or(0.0);
                            let score_b = engagement.get(&b.slug).copied().unwrap_or(0.0);
                            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
                        });
                    }
                }
            }
        }

        Ok(layout)
    }

    /// Internal helper to check if a scenario rating is allowed for a given profile rating.
    fn is_rating_allowed(&self, profile_rating: &str, scenario_rating: &str) -> bool {
        let profile_score = self.get_rating_score(profile_rating);
        let scenario_score = self.get_rating_score(scenario_rating);
        profile_score >= scenario_score
    }

    fn get_rating_score(&self, rating: &str) -> i32 {
        match rating.to_uppercase().as_str() {
            "G" | "GE" | "ALL" => 1,
            "PG" | "7+" => 2,
            "PG-13" | "13+" => 3,
            "16+" | "NC-17" => 4,
            "18" | "18+" | "R" | "ADULT" => 5,
            _ => 0, // Restricted by default if unknown
        }
    }

    /// Administrative: Save or update a layout.
    pub async fn save_layout(&self, req: SavePageLayoutRequest) -> AppResult<PageLayout> {
        let db_row = self.repo.upsert(
            &req.page_slug,
            req.is_landing.unwrap_or(false),
            &match req.nav_type.unwrap_or_default() {
                NavType::Main => "main".to_string(),
                NavType::Sub => "sub".to_string(),
                NavType::Hidden => "hidden".to_string(),
            },
            serde_json::to_value(&req.composition).unwrap_or_default(),
            req.device_type,
            req.maturity_rating,
            req.priority.unwrap_or(0),
            req.is_active.unwrap_or(true),
        ).await?;
        
        let layout = Self::map_db_to_domain(db_row);
        self.load_all_active().await?; 

        Ok(layout)
    }

    /// Administrative: Soft-delete a layout.
    pub async fn delete_layout(&self, slug: &str) -> AppResult<bool> {
        let success = self.repo.soft_delete(slug).await?;
        if success {
            self.invalidate_cache(slug).await;
            self.load_all_active().await?;
        }
        Ok(success)
    }

    pub async fn list_active_pages(&self) -> AppResult<Vec<PageLayout>> {
        let layouts = self.repo.find_all_active().await?;
        Ok(layouts.into_iter().map(Self::map_db_to_domain).collect())
    }

    pub async fn handle_activity(&self, _activity: &UserActivity) -> AppResult<()> {
        Ok(())
    }

    pub async fn invalidate_cache(&self, slug: &str) {
        let mut cache = self.cache.write().await;
        let keys: Vec<(PageSlug, Option<String>, Option<String>)> = cache.iter()
            .filter(|((s, _, _), _)| s.0 == slug)
            .map(|(k, _)| k.clone())
            .collect();
        for k in keys {
            cache.pop(&k);
        }
    }
}
