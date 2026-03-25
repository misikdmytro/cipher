pub use common::config::{ConfigLoadError, HostingConfig, RabbitMqConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub rabbitmq: RabbitMqConfig,
    pub api: HostingConfig,
    pub health: HostingConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigLoadError> {
        common::config::load_config("rotator.config.toml")
    }
}
