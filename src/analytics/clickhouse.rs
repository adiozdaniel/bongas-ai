use anyhow::Result;
use serde::Deserialize;

use crate::config::settings::ClickHouseSettings;

pub struct ClickHouseClient {
    client: clickhouse::Client,
}

impl ClickHouseClient {
    pub fn new(config: &ClickHouseSettings) -> Self {
        let client = clickhouse::Client::default()
            .with_url(&config.url)
            .with_user(&config.user)
            .with_password(&config.password)
            .with_database(&config.database);

        Self { client }
    }

    /// Get a reference to the inner clickhouse client for direct queries
    pub fn inner(&self) -> &clickhouse::Client {
        &self.client
    }

    /// Check if ClickHouse is available
    pub async fn ping(&self) -> Result<()> {
        #[derive(clickhouse::Row, Deserialize)]
        struct PingRow {
            #[allow(dead_code)]
            value: u8,
        }
        self.client
            .query("SELECT 1 as value")
            .fetch_one::<PingRow>()
            .await?;
        Ok(())
    }
}
