pub use common::config::{ConfigLoadError, HostingConfig, LogConfig, RabbitMqConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub log: LogConfig,
    pub rabbitmq: RabbitMqConfig,
    pub api: HostingConfig,
    pub grpc: HostingConfig,
    pub health: HostingConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigLoadError> {
        common::config::load_config("rotator.config.toml")
    }
}
