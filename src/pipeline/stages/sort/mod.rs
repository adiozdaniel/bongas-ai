//! Result sorting and limiting stages.

pub mod deduplicate;
pub mod limit;
pub mod paginate_results;
pub mod sort_by_relevance;
pub mod sort_by_score;

pub use deduplicate::DeduplicateStage;
pub use limit::LimitStage;
pub use paginate_results::PaginateResultsStage;
pub use sort_by_relevance::SortByRelevanceStage;
pub use sort_by_score::SortByScoreStage;
