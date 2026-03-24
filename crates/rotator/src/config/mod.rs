use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub rabbitmq: RabbitMqConfig,
    pub api: ApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RabbitMqConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

impl Default for RabbitMqConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5672,
            username: "cipher".to_string(),
            password: "cipher".to_string(),
        }
    }
}

impl RabbitMqConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "amqp://{}:{}@{}:{}/%2f",
            self.username, self.password, self.host, self.port
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            scheme: "http".to_string(),
            host: "[::1]".to_string(),
            port: 50051,
        }
    }
}

impl ApiConfig {
    pub fn address(&self) -> String {
        format!("{}://{}:{}", self.scheme, self.host, self.port)
    }
}

#[derive(Debug, Error)]
#[error("failed to load configuration: {0}")]
pub struct ConfigLoadError(anyhow::Error);

impl From<ConfigError> for ConfigLoadError {
    fn from(e: ConfigError) -> Self {
        ConfigLoadError(e.into())
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigLoadError> {
        Config::builder()
            .add_source(File::with_name("rotator.config.toml"))
            .add_source(Environment::with_prefix("CIPHER").separator("_"))
            .build()
            .map_err(ConfigLoadError::from)?
            .try_deserialize()
            .map_err(ConfigLoadError::from)
    }
}
