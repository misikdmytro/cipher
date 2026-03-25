pub use common::config::{ConfigLoadError, DatabaseConfig, HostingConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
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
