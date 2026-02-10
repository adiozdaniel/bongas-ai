use anyhow::Result;
use crate::config::settings::SecuritySettings;

pub struct HeartbeatManager {
    _config: SecuritySettings,
}

impl HeartbeatManager {
    pub fn new(config: &SecuritySettings) -> Result<Self> {
        Ok(Self {
            _config: config.clone(),
        })
    }
}
