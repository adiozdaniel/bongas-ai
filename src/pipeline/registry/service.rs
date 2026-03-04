use std::collections::HashMap;
use std::sync::Arc;
use crate::pipeline::types::PipelineStage;
use crate::engine::governance::strategy::dynamic::stages as dynamic_stages;

// Category 1: Data Fetching
use crate::pipeline::stages::fetch::fetch_clickhouse_watch_progress::service::FetchClickHouseWatchProgressStage;
use crate::pipeline::stages::fetch::fetch_clickhouse_trending::service::FetchClickHouseTrendingStage;
use crate::pipeline::stages::fetch::fetch_recent_watches::service::FetchRecentWatchesStage;
use crate::pipeline::stages::fetch::fetch_popular_content::service::FetchPopularContentStage;
use crate::pipeline::stages::fetch::fetch_by_category::service::FetchByCategoryStage;
use crate::pipeline::stages::fetch::fetch_because_you_watched::service::FetchBecauseYouWatchedStage;
use crate::pipeline::stages::fetch::fetch_similar_content::service::FetchSimilarContentStage;
use crate::pipeline::stages::fetch::fetch_new_releases::service::FetchNewReleasesStage;

// Category 2: Processing & Filtering
use crate::pipeline::stages::filter::filter_already_watched::service::FilterAlreadyWatchedStage;
use crate::pipeline::stages::filter::maturity_filter::service::MaturityFilterStage;
use crate::pipeline::stages::sort::sort_by_score::service::SortByScoreStage;
use crate::pipeline::stages::sort::deduplicate::service::DeduplicateStage;
use crate::pipeline::stages::diversify::diversify_genres::service::DiversifyGenresStage;
use crate::pipeline::stages::sort::limit::service::LimitStage;

// Category 3: ML & Inference
use crate::pipeline::stages::ml::onnx_inference::service::ONNXInferenceStage;
use crate::pipeline::stages::ml::ml_inference_similarity::service::MLInferenceSimilarityStage;
use crate::pipeline::stages::ml::multi_action_ranker::service::MultiActionRankerStage;

/// Global registry of all available pipeline stages.
#[derive(Clone)]
pub struct PipelineRegistry {
    stages: HashMap<String, Arc<dyn PipelineStage>>,
}

impl PipelineRegistry {
    pub fn new() -> Self {
        let mut registry = HashMap::new();
        register_static_stages(&mut registry);
        register_dynamic_stages(&mut registry);
        Self { stages: registry }
    }

    pub fn with_stages(stages: HashMap<String, Arc<dyn PipelineStage>>) -> Self {
        Self { stages }
    }

    pub fn get(&self, r#type: &str) -> Option<Arc<dyn PipelineStage>> {
        self.stages.get(r#type).cloned()
    }

    pub fn list_stages(&self) -> Vec<String> {
        self.stages.keys().cloned().collect()
    }
}

fn register_static_stages(registry: &mut HashMap<String, Arc<dyn PipelineStage>>) {
    // Fetch
    registry.insert("fetch_clickhouse_watch_progress".into(), Arc::new(FetchClickHouseWatchProgressStage));
    registry.insert("fetch_clickhouse_trending".into(), Arc::new(FetchClickHouseTrendingStage));
    registry.insert("fetch_recent_watches".into(), Arc::new(FetchRecentWatchesStage));
    registry.insert("fetch_popular_content".into(), Arc::new(FetchPopularContentStage));
    registry.insert("fetch_by_category".into(), Arc::new(FetchByCategoryStage));
    registry.insert("fetch_because_you_watched".into(), Arc::new(FetchBecauseYouWatchedStage));
    registry.insert("fetch_similar_content".into(), Arc::new(FetchSimilarContentStage));
    registry.insert("fetch_new_releases".into(), Arc::new(FetchNewReleasesStage));

    // Processing
    registry.insert("filter_already_watched".into(), Arc::new(FilterAlreadyWatchedStage));
    registry.insert("filter_seen_items".into(), Arc::new(FilterAlreadyWatchedStage)); // Alias
    registry.insert("maturity_filter".into(), Arc::new(MaturityFilterStage));
    registry.insert("filter_maturity_rating".into(), Arc::new(MaturityFilterStage)); // Alias
    registry.insert("sort_by_score".into(), Arc::new(SortByScoreStage));
    registry.insert("deduplicate".into(), Arc::new(DeduplicateStage));
    registry.insert("deduplicate_items".into(), Arc::new(DeduplicateStage)); // Alias
    registry.insert("diversify_genres".into(), Arc::new(DiversifyGenresStage));
    registry.insert("limit".into(), Arc::new(LimitStage));
    registry.insert("top_k_selector".into(), Arc::new(LimitStage)); // Alias

    // ML
    registry.insert("onnx_inference".into(), Arc::new(ONNXInferenceStage));
    registry.insert("ml_inference_similarity".into(), Arc::new(MLInferenceSimilarityStage));
    registry.insert("multi_action_ranker".into(), Arc::new(MultiActionRankerStage));
}

fn register_dynamic_stages(registry: &mut HashMap<String, Arc<dyn PipelineStage>>) {
    registry.insert("collaborative_filtering".into(), Arc::new(dynamic_stages::collaborative_filtering::service::CollaborativeFilteringStage));
    registry.insert("content_based".into(), Arc::new(dynamic_stages::content_based::service::ContentBasedStage));
    registry.insert("hybrid".into(), Arc::new(dynamic_stages::hybrid::service::HybridStage));
    registry.insert("dynamic_filters".into(), Arc::new(dynamic_stages::filters::service::DynamicFiltersStage));
    registry.insert("dynamic_boosters".into(), Arc::new(dynamic_stages::boosters::service::DynamicBoostersStage));
    registry.insert("dynamic_diversifiers".into(), Arc::new(dynamic_stages::diversifiers::service::DynamicDiversifiersStage));
    registry.insert("onnx_dynamic".into(), Arc::new(dynamic_stages::onnx_stages::service::ONNXDynamicStage));
}
