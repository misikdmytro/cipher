pub use common::config::{
    ConfigLoadError, DatabaseConfig, HostingConfig, LogConfig, RabbitMqConfig,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub log: LogConfig,
    pub database: DatabaseConfig,
    pub grpc: HostingConfig,
    pub rabbitmq: RabbitMqConfig,
    pub health: HostingConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigLoadError> {
        common::config::load_config("notificator.config.toml")
    }
}
