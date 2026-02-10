use anyhow::{Result, Context};
use serde_json::Value;
use crate::config::settings::{SpringCloudSettings, Settings};

/// Fetch configuration from Spring Cloud Config Server
pub async fn fetch_config(
    spring_cloud: &SpringCloudSettings,
    mut settings: Settings,
) -> Result<Settings> {
    let url = format!(
        "{}/{}/{}/{}",
        spring_cloud.url,
        spring_cloud.app_name,
        spring_cloud.profile,
        spring_cloud.label.as_deref().unwrap_or("master")
    );

    tracing::info!("Fetching configuration from Spring Cloud Config: {}", url);

    let response = reqwest::get(&url)
        .await
        .context("Failed to fetch from Spring Cloud Config")?
        .json::<Value>()
        .await
        .context("Failed to parse Spring Cloud Config response")?;

    // Parse property sources
    if let Some(property_sources) = response["propertySources"].as_array() {
        for source in property_sources {
            if let Some(source_obj) = source["source"].as_object() {
                // Override license key if present
                if let Some(license_key) = source_obj.get("security.license-key") {
                    if let Some(key_str) = license_key.as_str() {
                        settings.security.license_key = key_str.to_string();
                        tracing::info!("License key overridden from Spring Cloud Config");
                    }
                }

                // Override database URL if present
                if let Some(db_url) = source_obj.get("database.url") {
                    if let Some(url_str) = db_url.as_str() {
                        settings.database.url = url_str.to_string();
                        tracing::info!("Database URL overridden from Spring Cloud Config");
                    }
                }

                // Override Redis URL if present
                if let Some(redis_url) = source_obj.get("redis.url") {
                    if let Some(url_str) = redis_url.as_str() {
                        settings.redis.url = url_str.to_string();
                        tracing::info!("Redis URL overridden from Spring Cloud Config");
                    }
                }
            }
        }
    }

    Ok(settings)
}
