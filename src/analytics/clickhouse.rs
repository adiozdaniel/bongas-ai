//! ClickHouse database client implementation.
//!
//! Provides a configured wrapper around `clickhouse::Client` with
//! application settings applied at initialization.

use crate::config::settings::ClickHouseSettings;

/// Configured ClickHouse client with connection parameters applied.
pub struct ClickHouseClient {
    client: clickhouse::Client,
}

impl ClickHouseClient {
    /// Creates a new client instance from the provided configuration.
    ///
    /// # Arguments
    /// * `config` - Application ClickHouse settings containing URL, user, password, and database
    ///
    /// # Returns
    /// * `Self` - Initialized client ready for database operations
    ///
    /// Applies URL, credentials, and default database to the underlying client.
    pub fn new(config: &ClickHouseSettings) -> Self {
        let client = clickhouse::Client::default()
            .with_url(&config.url)
            .with_user(&config.user)
            .with_password(&config.password)
            .with_database(&config.database);

        Self { client }
    }

    /// Accessor for the inner client to perform direct queries.
    ///
    /// # Returns
    /// * `&clickhouse::Client` - Reference to the underlying ClickHouse client
    pub fn inner(&self) -> &clickhouse::Client {
        &self.client
    }
}
