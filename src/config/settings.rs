use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub redis: RedisSettings,
    pub clickhouse: ClickHouseSettings,
    pub kafka: KafkaSettings,
    pub security: SecuritySettings,
    pub ml: MlSettings,
    pub cache: CacheSettings,
    pub spring_cloud: Option<SpringCloudSettings>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub environment: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseSettings {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RedisSettings {
    pub url: String,
    pub pool_size: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClickHouseSettings {
    pub url: String,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KafkaSettings {
    pub brokers: String,
    pub group_id: String,
    pub profile_topic: String,
    pub reaction_topic: String,
    pub notification_topic: String,
    pub playback_topic: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecuritySettings {
    pub license_key: String,
    pub license_server_url: String,
    pub hardware_id_salt: String,
    pub enable_anti_debug: bool,
    pub enable_integrity_check: bool,
    pub heartbeat_interval_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MlSettings {
    pub model_path: PathBuf,
    pub batch_size: usize,
    pub device: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheSettings {
    pub l1_ttl_seconds: u64,
    pub l2_ttl_seconds: u64,
    pub warming_interval_minutes: Option<u64>,
    pub warm_scenarios: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SpringCloudSettings {
    pub enabled: bool,
    pub url: String,
    pub app_name: String,
    pub profile: String,
    pub label: Option<String>,
}

impl Settings {
    /// Load settings from config file and environment variables
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        let settings = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::Environment::default().separator("__"))
            .build()?
            .try_deserialize::<Settings>()?;

        Ok(settings)
    }

    /// Load settings, then override from Spring Cloud Config if enabled
    pub async fn load_with_spring_cloud() -> Result<Self> {
        let mut settings = Self::load()?;

        if let Some(spring_cloud) = settings.spring_cloud.clone() {
            if spring_cloud.enabled {
                settings = crate::config::spring_cloud::fetch_config(&spring_cloud, settings).await?;
            }
        }

        Ok(settings)
    }
}
