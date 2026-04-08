use config::{Config, ConfigError, Environment, File};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::level_filters::LevelFilter;

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
            port: 0,
        }
    }
}

impl HostingConfig {
    pub fn address(&self) -> String {
        format!("{}://{}:{}", self.scheme, self.host, self.port)
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    #[serde(with = "level_filter_serde")]
    pub min_level: LevelFilter,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            min_level: LevelFilter::DEBUG,
        }
    }
}

mod level_filter_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use tracing::level_filters::LevelFilter;

    pub fn serialize<S: Serializer>(level: &LevelFilter, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&level.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<LevelFilter, D::Error> {
        let s = String::deserialize(d)?;
        s.parse::<LevelFilter>().map_err(serde::de::Error::custom)
    }
}

impl LogConfig {
    pub fn init_tracing(&self) {
        tracing_subscriber::fmt()
            .with_max_level(self.min_level)
            .init();
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

pub fn load_config<T: DeserializeOwned>(file_name: &str) -> Result<T, ConfigLoadError> {
    Config::builder()
        .add_source(File::with_name(file_name))
        .add_source(Environment::with_prefix("CIPHER").separator("_"))
        .build()
        .map_err(ConfigLoadError::from)?
        .try_deserialize()
        .map_err(ConfigLoadError::from)
}
