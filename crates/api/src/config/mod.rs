use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub scheduler: HostingConfig,
    pub api: HostingConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub use_ssl: bool,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostingConfig {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

impl HostingConfig {
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
        let config = Config::builder()
            .add_source(File::with_name("api.config.toml"))
            .add_source(Environment::with_prefix("CIPHER").separator("_"))
            .build()
            .map_err(ConfigLoadError::from)?
            .try_deserialize()
            .map_err(ConfigLoadError::from)?;

        Ok(config)
    }
}
