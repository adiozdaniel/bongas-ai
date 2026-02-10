use std::collections::HashMap;
use std::sync::Arc;
use crate::pipeline::PipelineStage;
use crate::pipeline::stages;

/// Build the registry of all available pipeline stages
pub fn build_stage_registry() -> HashMap<String, Arc<dyn PipelineStage>> {
    let mut registry: HashMap<String, Arc<dyn PipelineStage>> = HashMap::new();

    // Category 1: Data Fetching (4 stages)
    registry.insert("fetch_clickhouse_watch_progress".into(), Arc::new(stages::fetch::FetchClickHouseWatchProgressStage));
    registry.insert("fetch_clickhouse_trending".into(), Arc::new(stages::fetch::FetchClickHouseTrendingStage));
    registry.insert("fetch_recent_watches".into(), Arc::new(stages::fetch::FetchRecentWatchesStage));
    registry.insert("fetch_popular_content".into(), Arc::new(stages::fetch::FetchPopularContentStage));

    // Category 2: ML Inference (7 stages) - WITH ONNX SUPPORT
    registry.insert("onnx_inference".into(), Arc::new(stages::ml::ONNXInferenceStage));
    registry.insert("onnx_inference_similarity".into(), Arc::new(stages::ml::ONNXInferenceSimilarityStage));
    registry.insert("onnx_inference_bandit".into(), Arc::new(stages::ml::ONNXInferenceBanditStage));
    registry.insert("ml_inference_two_tower".into(), Arc::new(stages::ml::MLInferenceTwoTowerStage));
    registry.insert("ml_inference_bert4rec".into(), Arc::new(stages::ml::MLInferenceBERT4RecStage));
    registry.insert("ml_inference_bandit".into(), Arc::new(stages::ml::MLInferenceBanditStage));
    registry.insert("ml_inference_similarity".into(), Arc::new(stages::ml::MLInferenceSimilarityStage));

    // Category 3: Filtering (2 stages)
    registry.insert("filter_already_watched".into(), Arc::new(stages::filter::FilterAlreadyWatchedStage));
    registry.insert("filter_by_genre".into(), Arc::new(stages::filter::FilterByGenreStage));

    // Category 4: Boosting/Scoring (2 stages)
    registry.insert("boost_by_recency".into(), Arc::new(stages::boost::BoostByRecencyStage));
    registry.insert("boost_by_popularity".into(), Arc::new(stages::boost::BoostByPopularityStage));

    // Category 5: Diversification (1 stage)
    registry.insert("diversify_genres".into(), Arc::new(stages::diversify::DiversifyGenresStage));

    // Category 6: Sorting/Limiting (3 stages)
    registry.insert("sort_by_score".into(), Arc::new(stages::sort::SortByScoreStage));
    registry.insert("limit".into(), Arc::new(stages::sort::LimitStage));
    registry.insert("deduplicate".into(), Arc::new(stages::sort::DeduplicateStage));

    // Category 7: Enrichment (1 stage)
    registry.insert("enrich_time_remaining".into(), Arc::new(stages::enrich::EnrichTimeRemainingStage));

    registry
}
