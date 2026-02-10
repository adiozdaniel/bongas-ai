pub mod clickhouse;
pub mod watch_progress;
pub mod trending;
pub mod user_behavior;
pub mod metrics;

pub use self::clickhouse::ClickHouseClient;
pub use watch_progress::{WatchProgressQuery, WatchProgressItem};
pub use trending::{TrendingQuery, TrendingItem};
pub use user_behavior::{UserBehaviorQuery, UserBehaviorStats};

use anyhow::Result;
use std::sync::Arc;

use crate::config::settings::ClickHouseSettings;

/// Analytics facade providing access to all ClickHouse query modules
pub struct Analytics {
    pub client: Arc<ClickHouseClient>,
    pub watch_progress: WatchProgressQuery,
    pub trending: TrendingQuery,
    pub user_behavior: UserBehaviorQuery,
}

impl Analytics {
    pub fn new(config: &ClickHouseSettings) -> Self {
        let client = Arc::new(ClickHouseClient::new(config));

        Self {
            client: client.clone(),
            watch_progress: WatchProgressQuery::new(client.clone()),
            trending: TrendingQuery::new(client.clone()),
            user_behavior: UserBehaviorQuery::new(client),
        }
    }

    pub async fn ping(&self) -> Result<()> {
        self.client.ping().await
    }
}
