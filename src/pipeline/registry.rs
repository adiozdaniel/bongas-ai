use std::collections::HashMap;
use std::sync::Arc;
use crate::pipeline::PipelineStage;
use crate::pipeline::stages;
use crate::scenarios::dynamic::stages as dynamic_stages;

/// Build the registry of all available pipeline stages
pub fn build_stage_registry() -> HashMap<String, Arc<dyn PipelineStage>> {
    let mut registry: HashMap<String, Arc<dyn PipelineStage>> = HashMap::new();

    // Category 1: Data Fetching (10 stages)
    registry.insert("fetch_clickhouse_watch_progress".into(), Arc::new(stages::fetch::FetchClickHouseWatchProgressStage));
    registry.insert("fetch_clickhouse_trending".into(), Arc::new(stages::fetch::FetchClickHouseTrendingStage));
    registry.insert("fetch_recent_watches".into(), Arc::new(stages::fetch::FetchRecentWatchesStage));
    registry.insert("fetch_popular_content".into(), Arc::new(stages::fetch::FetchPopularContentStage));
    registry.insert("fetch_user_preferences".into(), Arc::new(stages::fetch::FetchUserPreferencesStage));
    registry.insert("fetch_similar_content".into(), Arc::new(stages::fetch::FetchSimilarContentStage));
    registry.insert("fetch_new_releases".into(), Arc::new(stages::fetch::FetchNewReleasesStage));
    registry.insert("fetch_seasonal_content".into(), Arc::new(stages::fetch::FetchSeasonalContentStage));
    registry.insert("fetch_by_category".into(), Arc::new(stages::fetch::FetchByCategoryStage));
    registry.insert("fetch_because_you_watched".into(), Arc::new(stages::fetch::FetchBecauseYouWatchedStage));

    // Category 2: ML Inference (8 stages) - WITH ONNX SUPPORT
    registry.insert("onnx_inference".into(), Arc::new(stages::ml::ONNXInferenceStage));
    registry.insert("onnx_inference_similarity".into(), Arc::new(stages::ml::ONNXInferenceSimilarityStage));
    registry.insert("ml_inference_two_tower".into(), Arc::new(stages::ml::MLInferenceTwoTowerStage));
    registry.insert("ml_inference_bert4rec".into(), Arc::new(stages::ml::MLInferenceBERT4RecStage));
    registry.insert("ml_inference_similarity".into(), Arc::new(stages::ml::MLInferenceSimilarityStage));
    registry.insert("heuristic_aggregator".into(), Arc::new(stages::ml::HeuristicAggregatorStage));
    registry.insert("meta_scorer".into(), Arc::new(stages::ml::MetaScorerStage));
    registry.insert("multi_action_ranker".into(), Arc::new(stages::ml::MultiActionRankerStage));

    // Category 3: Filtering (12 stages)
    registry.insert("filter_already_watched".into(), Arc::new(stages::filter::FilterAlreadyWatchedStage));
    registry.insert("filter_by_genre".into(), Arc::new(stages::filter::FilterByGenreStage));
    registry.insert("filter_by_age_rating".into(), Arc::new(stages::filter::FilterByAgeRatingStage));
    registry.insert("filter_by_language".into(), Arc::new(stages::filter::FilterByLanguageStage));
    registry.insert("filter_by_duration".into(), Arc::new(stages::filter::FilterByDurationStage));
    registry.insert("filter_by_country".into(), Arc::new(stages::filter::FilterByCountryStage));
    registry.insert("filter_by_release_year".into(), Arc::new(stages::filter::FilterByReleaseYearStage));
    registry.insert("filter_by_rating".into(), Arc::new(stages::filter::FilterByRatingStage));
    registry.insert("filter_by_availability".into(), Arc::new(stages::filter::FilterByAvailabilityStage));
    registry.insert("filter_by_subscription_tier".into(), Arc::new(stages::filter::FilterBySubscriptionTierStage));
    registry.insert("filter_explicit_content".into(), Arc::new(stages::filter::FilterExplicitContentStage));
    registry.insert("filter_by_quality".into(), Arc::new(stages::filter::FilterByQualityStage));

    // Category 4: Boosting/Scoring (10 stages)
    registry.insert("boost_by_recency".into(), Arc::new(stages::boost::BoostByRecencyStage));
    registry.insert("boost_by_popularity".into(), Arc::new(stages::boost::BoostByPopularityStage));
    registry.insert("boost_trending".into(), Arc::new(stages::boost::BoostTrendingStage));
    registry.insert("boost_new_content".into(), Arc::new(stages::boost::BoostNewContentStage));
    registry.insert("boost_user_affinity".into(), Arc::new(stages::boost::BoostUserAffinityStage));
    registry.insert("boost_seasonal".into(), Arc::new(stages::boost::BoostSeasonalStage));
    registry.insert("boost_engagement".into(), Arc::new(stages::boost::BoostEngagementStage));
    registry.insert("boost_completion_rate".into(), Arc::new(stages::boost::BoostCompletionRateStage));
    registry.insert("boost_promoted".into(), Arc::new(stages::boost::BoostPromotedStage));
    registry.insert("boost_personalization".into(), Arc::new(stages::boost::BoostPersonalizationStage));

    // Category 5: Diversification (5 stages)
    registry.insert("diversify_genres".into(), Arc::new(stages::diversify::DiversifyGenresStage));
    registry.insert("diversify_by_creator".into(), Arc::new(stages::diversify::DiversifyByCreatorStage));
    registry.insert("diversify_by_release_year".into(), Arc::new(stages::diversify::DiversifyByReleaseYearStage));
    registry.insert("serendipity".into(), Arc::new(stages::diversify::SerendipityStage));
    registry.insert("diversify_mmr".into(), Arc::new(stages::diversify::DiversifyMMRStage));

    // Category 6: Sorting/Limiting (5 stages)
    registry.insert("sort_by_score".into(), Arc::new(stages::sort::SortByScoreStage));
    registry.insert("limit".into(), Arc::new(stages::sort::LimitStage));
    registry.insert("deduplicate".into(), Arc::new(stages::sort::DeduplicateStage));
    registry.insert("sort_by_relevance".into(), Arc::new(stages::sort::SortByRelevanceStage));
    registry.insert("paginate_results".into(), Arc::new(stages::sort::PaginateResultsStage));

    // Category 7: Enrichment (1 stage)
    registry.insert("enrich_time_remaining".into(), Arc::new(stages::enrich::EnrichTimeRemainingStage));

    // Category 8: Dynamic Stages (7 stages)
    registry.insert("collaborative_filtering".into(), Arc::new(dynamic_stages::collaborative_filtering::CollaborativeFilteringStage));
    registry.insert("content_based".into(), Arc::new(dynamic_stages::content_based::ContentBasedStage));
    registry.insert("hybrid".into(), Arc::new(dynamic_stages::hybrid::HybridStage));
    registry.insert("dynamic_filters".into(), Arc::new(dynamic_stages::filters::DynamicFiltersStage));
    registry.insert("dynamic_boosters".into(), Arc::new(dynamic_stages::boosters::DynamicBoostersStage));
    registry.insert("dynamic_diversifiers".into(), Arc::new(dynamic_stages::diversifiers::DynamicDiversifiersStage));
    registry.insert("onnx_dynamic".into(), Arc::new(dynamic_stages::onnx_stages::ONNXDynamicStage));

    registry
}
