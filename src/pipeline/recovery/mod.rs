//! Data fetching stages.

pub mod fetch_because_you_watched;
pub mod fetch_by_category;
pub mod fetch_clickhouse_trending;
pub mod fetch_clickhouse_watch_progress;
pub mod fetch_new_releases;
pub mod fetch_popular_content;
pub mod fetch_recent_watches;
pub mod fetch_seasonal_content;
pub mod fetch_similar_content;
pub mod fetch_user_preferences;

pub use fetch_because_you_watched::FetchBecauseYouWatchedStage;
pub use fetch_by_category::FetchByCategoryStage;
pub use fetch_clickhouse_trending::FetchClickHouseTrendingStage;
pub use fetch_clickhouse_watch_progress::FetchClickHouseWatchProgressStage;
pub use fetch_new_releases::FetchNewReleasesStage;
pub use fetch_popular_content::FetchPopularContentStage;
pub use fetch_recent_watches::FetchRecentWatchesStage;
pub use fetch_seasonal_content::FetchSeasonalContentStage;
pub use fetch_similar_content::FetchSimilarContentStage;
pub use fetch_user_preferences::FetchUserPreferencesStage;
pub mod fetch_seasonal_content;
pub mod fetch_new_releases;
pub mod fetch_because_you_watched;
pub mod fetch_clickhouse_watch_progress;
pub mod fetch_by_category;
pub mod fetch_popular_content;
pub mod fetch_user_preferences;
pub mod fetch_similar_content;
pub mod fetch_clickhouse_trending;
pub mod fetch_recent_watches;
