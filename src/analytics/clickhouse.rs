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
}
