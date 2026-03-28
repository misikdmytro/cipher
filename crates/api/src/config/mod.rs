pub use common::config::{ConfigLoadError, DatabaseConfig, HostingConfig, LogConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub log: LogConfig,
    pub database: DatabaseConfig,
    pub scheduler: HostingConfig,
    pub api: HostingConfig,
    pub grpc: HostingConfig,
    pub notificator: HostingConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigLoadError> {
        common::config::load_config("api.config.toml")
    }
}
