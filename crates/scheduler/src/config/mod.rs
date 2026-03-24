use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub grpc: HostingConfig,
    pub worker: WorkerConfig,
    pub rabbitmq: RabbitMqConfig,
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
pub struct DatabaseConfig {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub use_ssl: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            username: "cipher".to_string(),
            password: "cipher".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            database: "cipher".to_string(),
            use_ssl: false,
        }
    }
}

impl DatabaseConfig {
    pub fn connection_string(&self) -> String {
        let ssl_mode = if self.use_ssl { "require" } else { "disable" };
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.username, self.password, self.host, self.port, self.database, ssl_mode
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostingConfig {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

impl Default for HostingConfig {
    fn default() -> Self {
        Self {
            scheme: "http".to_string(),
            host: "[::1]".to_string(),
            port: 50052,
        }
    }
}

impl HostingConfig {
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub concurrency: usize,
    #[serde(with = "duration_millis")]
    pub poll_interval: std::time::Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            concurrency: 10,
            poll_interval: std::time::Duration::from_millis(1000),
        }
    }
}

mod duration_millis {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        d.as_millis().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        Ok(Duration::from_millis(u64::deserialize(d)?))
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
            .add_source(File::with_name("scheduler.config.toml"))
            .add_source(Environment::with_prefix("CIPHER").separator("_"))
            .build()
            .map_err(ConfigLoadError::from)?
            .try_deserialize()
            .map_err(ConfigLoadError::from)
    }
}
