use std::collections::HashMap;
use std::sync::Arc;
use crate::pipeline::types::PipelineStage;
use crate::engine::governance::strategy::dynamic::stages as dynamic_stages;

// Category 1: Data Fetching
use crate::pipeline::stages::fetch::fetch_clickhouse_watch_progress::FetchClickHouseWatchProgressStage;
use crate::pipeline::stages::fetch::fetch_clickhouse_trending::FetchClickHouseTrendingStage;
use crate::pipeline::stages::fetch::fetch_recent_watches::FetchRecentWatchesStage;
use crate::pipeline::stages::fetch::fetch_popular_content::FetchPopularContentStage;
use crate::pipeline::stages::fetch::fetch_user_preferences::FetchUserPreferencesStage;
use crate::pipeline::stages::fetch::fetch_similar_content::FetchSimilarContentStage;
use crate::pipeline::stages::fetch::fetch_new_releases::FetchNewReleasesStage;
use crate::pipeline::stages::fetch::fetch_seasonal_content::FetchSeasonalContentStage;
use crate::pipeline::stages::fetch::fetch_by_category::FetchByCategoryStage;
use crate::pipeline::stages::fetch::fetch_because_you_watched::FetchBecauseYouWatchedStage;

// Category 2: ML Inference
use crate::pipeline::stages::ml::onnx_inference::ONNXInferenceStage;
use crate::pipeline::stages::ml::onnx_inference_similarity::ONNXInferenceSimilarityStage;
use crate::pipeline::stages::ml::ml_inference_two_tower::MLInferenceTwoTowerStage;
use crate::pipeline::stages::ml::ml_inference_bert4rec::MLInferenceBERT4RecStage;
use crate::pipeline::stages::ml::ml_inference_similarity::MLInferenceSimilarityStage;
use crate::pipeline::stages::ml::heuristic_aggregator::HeuristicAggregatorStage;
use crate::pipeline::stages::ml::meta_scorer::MetaScorerStage;
use crate::pipeline::stages::ml::multi_action_ranker::MultiActionRankerStage;

// Category 3: Filtering
use crate::pipeline::stages::filter::filter_already_watched::FilterAlreadyWatchedStage;
use crate::pipeline::stages::filter::filter_by_genre::FilterByGenreStage;
use crate::pipeline::stages::filter::filter_by_age_rating::FilterByAgeRatingStage;
use crate::pipeline::stages::filter::filter_by_language::FilterByLanguageStage;
use crate::pipeline::stages::filter::filter_by_duration::FilterByDurationStage;
use crate::pipeline::stages::filter::filter_by_country::FilterByCountryStage;
use crate::pipeline::stages::filter::filter_by_release_year::FilterByReleaseYearStage;
use crate::pipeline::stages::filter::filter_by_rating::FilterByRatingStage;
use crate::pipeline::stages::filter::filter_by_availability::FilterByAvailabilityStage;
use crate::pipeline::stages::filter::filter_by_subscription_tier::FilterBySubscriptionTierStage;
use crate::pipeline::stages::filter::filter_explicit_content::FilterExplicitContentStage;
use crate::pipeline::stages::filter::filter_by_quality::FilterByQualityStage;
use crate::pipeline::stages::filter::maturity_filter::MaturityFilterStage;

// Category 4: Boosting
use crate::pipeline::stages::boost::boost_by_recency::BoostByRecencyStage;
use crate::pipeline::stages::boost::boost_by_popularity::BoostByPopularityStage;
use crate::pipeline::stages::boost::boost_trending::BoostTrendingStage;
use crate::pipeline::stages::boost::boost_new_content::BoostNewContentStage;
use crate::pipeline::stages::boost::boost_user_affinity::BoostUserAffinityStage;
use crate::pipeline::stages::boost::boost_seasonal::BoostSeasonalStage;
use crate::pipeline::stages::boost::boost_engagement::BoostEngagementStage;
use crate::pipeline::stages::boost::boost_completion_rate::BoostCompletionRateStage;
use crate::pipeline::stages::boost::boost_promoted::BoostPromotedStage;
use crate::pipeline::stages::boost::boost_personalization::BoostPersonalizationStage;
use crate::pipeline::stages::boost::affinity_freshness::AffinityFreshnessStage;

// Category 5: Diversification
use crate::pipeline::stages::diversify::diversify_genres::DiversifyGenresStage;
use crate::pipeline::stages::diversify::diversify_by_creator::DiversifyByCreatorStage;
use crate::pipeline::stages::diversify::diversify_by_release_year::DiversifyByReleaseYearStage;
use crate::pipeline::stages::diversify::serendipity::SerendipityStage;
use crate::pipeline::stages::diversify::diversify_mmr::DiversifyMMRStage;

// Category 6: Sorting
use crate::pipeline::stages::sort::sort_by_score::SortByScoreStage;
use crate::pipeline::stages::sort::limit::LimitStage;
use crate::pipeline::stages::sort::deduplicate::DeduplicateStage;
use crate::pipeline::stages::sort::sort_by_relevance::SortByRelevanceStage;
use crate::pipeline::stages::sort::paginate_results::PaginateResultsStage;

// Category 7: Enrichment
use crate::pipeline::stages::enrich::enrich_time_remaining::EnrichTimeRemainingStage;

/// Central registry for all available pipeline stages.
#[derive(Clone)]
pub struct PipelineRegistry {
    stages: HashMap<String, Arc<dyn PipelineStage>>,
}

impl PipelineRegistry {
    /// Create a new registry populated with all standard and dynamic stages.
    pub fn new() -> Self {
        Self {
            stages: build_stage_registry(),
        }
    }

    /// Create a new registry with custom stages (primarily for testing/benchmarking).
    pub fn with_stages(stages: HashMap<String, Arc<dyn PipelineStage>>) -> Self {
        Self { stages }
    }

    /// Get a stage implementation by its unique type identifier.
    pub fn get(&self, stage_type: &str) -> Option<Arc<dyn PipelineStage>> {
        self.stages.get(stage_type).cloned()
    }

    /// List all registered stage names.
    pub fn list_stages(&self) -> Vec<String> {
        self.stages.keys().cloned().collect()
    }
}

impl Default for PipelineRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal helper to build the flat stage map.
fn build_stage_registry() -> HashMap<String, Arc<dyn PipelineStage>> {
    let mut registry: HashMap<String, Arc<dyn PipelineStage>> = HashMap::new();

    // Category 1: Data Fetching
    registry.insert("fetch_clickhouse_watch_progress".into(), Arc::new(FetchClickHouseWatchProgressStage));
    registry.insert("fetch_clickhouse_trending".into(), Arc::new(FetchClickHouseTrendingStage));
    registry.insert("fetch_recent_watches".into(), Arc::new(FetchRecentWatchesStage));
    registry.insert("fetch_popular_content".into(), Arc::new(FetchPopularContentStage));
    registry.insert("hot_items".into(), Arc::new(FetchPopularContentStage));
    registry.insert("fetch_user_preferences".into(), Arc::new(FetchUserPreferencesStage));
    registry.insert("fetch_similar_content".into(), Arc::new(FetchSimilarContentStage));
    registry.insert("fetch_new_releases".into(), Arc::new(FetchNewReleasesStage));
    registry.insert("fetch_seasonal_content".into(), Arc::new(FetchSeasonalContentStage));
    registry.insert("fetch_by_category".into(), Arc::new(FetchByCategoryStage));
    registry.insert("fetch_because_you_watched".into(), Arc::new(FetchBecauseYouWatchedStage));

    // Category 2: ML Inference
    registry.insert("onnx_inference".into(), Arc::new(ONNXInferenceStage));
    registry.insert("onnx_ranker".into(), Arc::new(ONNXInferenceStage));
    registry.insert("onnx_inference_similarity".into(), Arc::new(ONNXInferenceSimilarityStage));
    registry.insert("ml_inference_two_tower".into(), Arc::new(MLInferenceTwoTowerStage));
    registry.insert("personalized_recommender".into(), Arc::new(MLInferenceTwoTowerStage));
    registry.insert("ml_inference_bert4rec".into(), Arc::new(MLInferenceBERT4RecStage));
    registry.insert("ml_inference_similarity".into(), Arc::new(MLInferenceSimilarityStage));
    registry.insert("vector_search".into(), Arc::new(MLInferenceSimilarityStage));
    registry.insert("heuristic_aggregator".into(), Arc::new(HeuristicAggregatorStage));
    registry.insert("meta_scorer".into(), Arc::new(MetaScorerStage));
    registry.insert("multi_action_ranker".into(), Arc::new(MultiActionRankerStage));

    // Category 3: Filtering
    registry.insert("filter_already_watched".into(), Arc::new(FilterAlreadyWatchedStage));
    registry.insert("staleness_filter".into(), Arc::new(FilterAlreadyWatchedStage));
    registry.insert("filter_by_genre".into(), Arc::new(FilterByGenreStage));
    registry.insert("filter_by_age_rating".into(), Arc::new(FilterByAgeRatingStage));
    registry.insert("filter_by_language".into(), Arc::new(FilterByLanguageStage));
    registry.insert("filter_by_duration".into(), Arc::new(FilterByDurationStage));
    registry.insert("filter_by_country".into(), Arc::new(FilterByCountryStage));
    registry.insert("filter_by_release_year".into(), Arc::new(FilterByReleaseYearStage));
    registry.insert("filter_by_rating".into(), Arc::new(FilterByRatingStage));
    registry.insert("filter_by_availability".into(), Arc::new(FilterByAvailabilityStage));
    registry.insert("filter_by_subscription_tier".into(), Arc::new(FilterBySubscriptionTierStage));
    registry.insert("filter_explicit_content".into(), Arc::new(FilterExplicitContentStage));
    registry.insert("filter_by_quality".into(), Arc::new(FilterByQualityStage));
    registry.insert("maturity_filter".into(), Arc::new(MaturityFilterStage));

    // Category 4: Boosting
    registry.insert("boost_by_recency".into(), Arc::new(BoostByRecencyStage));
    registry.insert("business_logic".into(), Arc::new(BoostByRecencyStage));
    registry.insert("boost_by_popularity".into(), Arc::new(BoostByPopularityStage));
    registry.insert("boost_trending".into(), Arc::new(BoostTrendingStage));
    registry.insert("boost_new_content".into(), Arc::new(BoostNewContentStage));
    registry.insert("boost_user_affinity".into(), Arc::new(BoostUserAffinityStage));
    registry.insert("boost_seasonal".into(), Arc::new(BoostSeasonalStage));
    registry.insert("boost_engagement".into(), Arc::new(BoostEngagementStage));
    registry.insert("boost_completion_rate".into(), Arc::new(BoostCompletionRateStage));
    registry.insert("boost_promoted".into(), Arc::new(BoostPromotedStage));
    registry.insert("boost_personalization".into(), Arc::new(BoostPersonalizationStage));
    registry.insert("affinity_freshness".into(), Arc::new(AffinityFreshnessStage));

    // Category 5: Diversification
    registry.insert("diversify_genres".into(), Arc::new(DiversifyGenresStage));
    registry.insert("diversify_by_creator".into(), Arc::new(DiversifyByCreatorStage));
    registry.insert("diversify_by_release_year".into(), Arc::new(DiversifyByReleaseYearStage));
    registry.insert("serendipity".into(), Arc::new(SerendipityStage));
    registry.insert("diversify_mmr".into(), Arc::new(DiversifyMMRStage));
    registry.insert("diversity_reranker".into(), Arc::new(DiversifyMMRStage));

    // Category 6: Sorting
    registry.insert("sort_by_score".into(), Arc::new(SortByScoreStage));
    registry.insert("limit".into(), Arc::new(LimitStage));
    registry.insert("deduplicate".into(), Arc::new(DeduplicateStage));
    registry.insert("sort_by_relevance".into(), Arc::new(SortByRelevanceStage));
    registry.insert("paginate_results".into(), Arc::new(PaginateResultsStage));

    // Category 7: Enrichment
    registry.insert("enrich_time_remaining".into(), Arc::new(EnrichTimeRemainingStage));

    // Category 8: Dynamic Stages
    registry.insert("collaborative_filtering".into(), Arc::new(dynamic_stages::collaborative_filtering::CollaborativeFilteringStage));
    registry.insert("collaborative_filter".into(), Arc::new(dynamic_stages::collaborative_filtering::CollaborativeFilteringStage));
    registry.insert("content_based".into(), Arc::new(dynamic_stages::content_based::ContentBasedStage));
    registry.insert("hybrid".into(), Arc::new(dynamic_stages::hybrid::HybridStage));
    registry.insert("dynamic_filters".into(), Arc::new(dynamic_stages::filters::DynamicFiltersStage));
    registry.insert("dynamic_boosters".into(), Arc::new(dynamic_stages::boosters::DynamicBoostersStage));
    registry.insert("dynamic_diversifiers".into(), Arc::new(dynamic_stages::diversifiers::DynamicDiversifiersStage));
    registry.insert("onnx_dynamic".into(), Arc::new(dynamic_stages::onnx_stages::ONNXDynamicStage));

    registry
}
