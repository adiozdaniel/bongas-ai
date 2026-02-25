//! Result diversification stages.

pub mod diversify_by_creator;
pub mod diversify_by_release_year;
pub mod diversify_genres;
pub mod diversify_mmr;
pub mod serendipity;

pub use diversify_by_creator::DiversifyByCreatorStage;
pub use diversify_by_release_year::DiversifyByReleaseYearStage;
pub use diversify_genres::DiversifyGenresStage;
pub use diversify_mmr::DiversifyMMRStage;
pub use serendipity::SerendipityStage;
