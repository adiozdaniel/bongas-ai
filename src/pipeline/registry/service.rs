use std::collections::HashMap;
use std::sync::Arc;
use crate::pipeline::types::models::PipelineStage;
use crate::engine::governance::strategy::dynamic::stages as dynamic_stages;

// Category 1: Recovery (The Stage)
use crate::pipeline::recovery::fetch_clickhouse_watch_progress::service::FetchClickHouseWatchProgressStage;
use crate::pipeline::recovery::fetch_clickhouse_trending::service::FetchClickHouseTrendingStage;
use crate::pipeline::recovery::fetch_recent_watches::service::FetchRecentWatchesStage;
use crate::pipeline::recovery::fetch_popular_content::service::FetchPopularContentStage;
use crate::pipeline::recovery::fetch_by_category::service::FetchByCategoryStage;
use crate::pipeline::recovery::fetch_because_you_watched::service::FetchBecauseYouWatchedStage;
use crate::pipeline::recovery::fetch_similar_content::service::FetchSimilarContentStage;
use crate::pipeline::recovery::fetch_new_releases::service::FetchNewReleasesStage;
use crate::pipeline::recovery::fetch_user_preferences::service::FetchUserPreferencesStage;
use crate::pipeline::recovery::fetch_seasonal_content::service::FetchSeasonalContentStage;
use crate::pipeline::recovery::fetch_behavioral_tribe::service::FetchBehavioralTribeStage;

// Category 2: Processing (The Backstage)
use crate::pipeline::processing::filter_already_watched::service::FilterAlreadyWatchedStage;
use crate::pipeline::processing::maturity_filter::service::MaturityFilterStage;
use crate::pipeline::processing::deduplicate::service::DeduplicateStage;

// Category 3: Ranking (The Pulse)
use crate::pipeline::ranking::sort_by_score::service::SortByScoreStage;
use crate::pipeline::ranking::diversify_genres::service::DiversifyGenresStage;
use crate::pipeline::ranking::limit::service::LimitStage;
use crate::pipeline::ranking::onnx_inference::service::ONNXInferenceStage;
use crate::pipeline::ranking::ml_inference_similarity::service::MLInferenceSimilarityStage;
use crate::pipeline::ranking::multi_action_ranker::service::MultiActionRankerStage;
use crate::pipeline::ranking::ml_inference_two_tower::service::MLInferenceTwoTowerStage;
use crate::pipeline::ranking::ml_inference_bert4rec::service::MLInferenceBERT4RecStage;
use crate::pipeline::ranking::onnx_inference_similarity::service::ONNXInferenceSimilarityStage;
use crate::pipeline::ranking::boost_by_popularity::service::BoostByPopularityStage;
use crate::pipeline::ranking::boost_by_recency::service::BoostByRecencyStage;
use crate::pipeline::ranking::boost_trending::service::BoostTrendingStage;
use crate::pipeline::ranking::boost_promoted::service::BoostPromotedStage;
use crate::pipeline::ranking::boost_personalization::service::BoostPersonalizationStage;
use crate::pipeline::ranking::boost_engagement::service::BoostEngagementStage;
use crate::pipeline::ranking::boost_completion_rate::service::BoostCompletionRateStage;
use crate::pipeline::ranking::boost_new_content::service::BoostNewContentStage;
use crate::pipeline::ranking::boost_user_affinity::service::BoostUserAffinityStage;
use crate::pipeline::ranking::diversify_mmr::service::DiversifyMMRStage;
use crate::pipeline::ranking::diversify_by_release_year::service::DiversifyByReleaseYearStage;
use crate::pipeline::ranking::diversify_by_creator::service::DiversifyByCreatorStage;
use crate::pipeline::ranking::serendipity::service::SerendipityStage;
use crate::pipeline::ranking::affinity_freshness::service::AffinityFreshnessStage;
use crate::pipeline::ranking::paginate_results::service::PaginateResultsStage;
use crate::pipeline::ranking::meta_scorer::service::MetaScorerStage;
use crate::pipeline::ranking::heuristic_aggregator::service::HeuristicAggregatorStage;
use crate::pipeline::ranking::sort_by_relevance::service::SortByRelevanceStage;

// Category 2 additions
use crate::pipeline::processing::filter_by_availability::service::FilterByAvailabilityStage;
use crate::pipeline::processing::filter_by_duration::service::FilterByDurationStage;
use crate::pipeline::processing::filter_by_country::service::FilterByCountryStage;
use crate::pipeline::processing::filter_by_rating::service::FilterByRatingStage;
use crate::pipeline::processing::filter_by_release_year::service::FilterByReleaseYearStage;
use crate::pipeline::processing::filter_by_age_rating::service::FilterByAgeRatingStage;
use crate::pipeline::processing::filter_by_quality::service::FilterByQualityStage;
use crate::pipeline::processing::filter_explicit_content::service::FilterExplicitContentStage;
use crate::pipeline::processing::filter_by_genre::service::FilterByGenreStage;
use crate::pipeline::processing::filter_by_language::service::FilterByLanguageStage;
use crate::pipeline::processing::filter_by_subscription_tier::service::FilterBySubscriptionTierStage;
use crate::pipeline::processing::enrich_time_remaining::service::EnrichTimeRemainingStage;

/// Global registry of all available pipeline stages.
#[derive(Clone)]
pub struct PipelineRegistry {
    stages: HashMap<String, Arc<dyn PipelineStage>>,
}

impl Default for PipelineRegistry {
    fn default() -> Self {
        Self::new()
    }
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
    // Recovery
    registry.insert("fetch_clickhouse_watch_progress".into(), Arc::new(FetchClickHouseWatchProgressStage));
    registry.insert("fetch_clickhouse_trending".into(), Arc::new(FetchClickHouseTrendingStage));
    registry.insert("fetch_recent_watches".into(), Arc::new(FetchRecentWatchesStage));
    registry.insert("fetch_popular_content".into(), Arc::new(FetchPopularContentStage));
    registry.insert("fetch_by_category".into(), Arc::new(FetchByCategoryStage));
    registry.insert("fetch_because_you_watched".into(), Arc::new(FetchBecauseYouWatchedStage));
    registry.insert("fetch_similar_content".into(), Arc::new(FetchSimilarContentStage));
    registry.insert("fetch_new_releases".into(), Arc::new(FetchNewReleasesStage));
    registry.insert("fetch_user_preferences".into(), Arc::new(FetchUserPreferencesStage));
    registry.insert("fetch_seasonal_content".into(), Arc::new(FetchSeasonalContentStage));
    registry.insert("fetch_behavioral_tribe".into(), Arc::new(FetchBehavioralTribeStage));

    // Processing
    registry.insert("filter_already_watched".into(), Arc::new(FilterAlreadyWatchedStage));
    registry.insert("filter_seen_items".into(), Arc::new(FilterAlreadyWatchedStage)); // Alias
    registry.insert("maturity_filter".into(), Arc::new(MaturityFilterStage));
    registry.insert("filter_maturity_rating".into(), Arc::new(MaturityFilterStage)); // Alias
    registry.insert("deduplicate".into(), Arc::new(DeduplicateStage));
    registry.insert("deduplicate_items".into(), Arc::new(DeduplicateStage)); // Alias
    registry.insert("filter_by_availability".into(), Arc::new(FilterByAvailabilityStage));
    registry.insert("filter_by_duration".into(), Arc::new(FilterByDurationStage));
    registry.insert("filter_by_country".into(), Arc::new(FilterByCountryStage));
    registry.insert("filter_by_rating".into(), Arc::new(FilterByRatingStage));
    registry.insert("filter_by_release_year".into(), Arc::new(FilterByReleaseYearStage));
    registry.insert("filter_by_age_rating".into(), Arc::new(FilterByAgeRatingStage));
    registry.insert("filter_by_quality".into(), Arc::new(FilterByQualityStage));
    registry.insert("filter_explicit_content".into(), Arc::new(FilterExplicitContentStage));
    registry.insert("filter_by_genre".into(), Arc::new(FilterByGenreStage));
    registry.insert("filter_by_language".into(), Arc::new(FilterByLanguageStage));
    registry.insert("filter_by_subscription_tier".into(), Arc::new(FilterBySubscriptionTierStage));
    registry.insert("enrich_time_remaining".into(), Arc::new(EnrichTimeRemainingStage));

    // Ranking
    registry.insert("sort_by_score".into(), Arc::new(SortByScoreStage));
    registry.insert("diversify_genres".into(), Arc::new(DiversifyGenresStage));
    registry.insert("limit".into(), Arc::new(LimitStage));
    registry.insert("top_k_selector".into(), Arc::new(LimitStage)); // Alias
    registry.insert("onnx_inference".into(), Arc::new(ONNXInferenceStage));
    registry.insert("ml_inference_similarity".into(), Arc::new(MLInferenceSimilarityStage));
    registry.insert("multi_action_ranker".into(), Arc::new(MultiActionRankerStage));
    registry.insert("ml_inference_two_tower".into(), Arc::new(MLInferenceTwoTowerStage));
    registry.insert("ml_inference_bert4rec".into(), Arc::new(MLInferenceBERT4RecStage));
    registry.insert("onnx_inference_similarity".into(), Arc::new(ONNXInferenceSimilarityStage));
    registry.insert("boost_by_popularity".into(), Arc::new(BoostByPopularityStage));
    registry.insert("boost_by_recency".into(), Arc::new(BoostByRecencyStage));
    registry.insert("boost_trending".into(), Arc::new(BoostTrendingStage));
    registry.insert("boost_promoted".into(), Arc::new(BoostPromotedStage));
    registry.insert("boost_personalization".into(), Arc::new(BoostPersonalizationStage));
    registry.insert("boost_engagement".into(), Arc::new(BoostEngagementStage));
    registry.insert("boost_completion_rate".into(), Arc::new(BoostCompletionRateStage));
    registry.insert("boost_new_content".into(), Arc::new(BoostNewContentStage));
    registry.insert("boost_user_affinity".into(), Arc::new(BoostUserAffinityStage));
    registry.insert("diversify_mmr".into(), Arc::new(DiversifyMMRStage));
    registry.insert("diversify_by_release_year".into(), Arc::new(DiversifyByReleaseYearStage));
    registry.insert("diversify_by_creator".into(), Arc::new(DiversifyByCreatorStage));
    registry.insert("serendipity".into(), Arc::new(SerendipityStage));
    registry.insert("affinity_freshness".into(), Arc::new(AffinityFreshnessStage));
    registry.insert("paginate_results".into(), Arc::new(PaginateResultsStage));
    registry.insert("meta_scorer".into(), Arc::new(MetaScorerStage));
    registry.insert("heuristic_aggregator".into(), Arc::new(HeuristicAggregatorStage));
    registry.insert("sort_by_relevance".into(), Arc::new(SortByRelevanceStage));
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
