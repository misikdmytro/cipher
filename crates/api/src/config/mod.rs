use std::fmt::{Display, Formatter};

use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub database: DatabaseConfig,
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

impl Display for DatabaseConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ssl_mode = if self.use_ssl { "require" } else { "disable" };
        write!(
            f,
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            self.username, self.password, self.host, self.port, self.database, ssl_mode
        )
    }
}

#[derive(Debug, Error)]
#[error("failed to load configuration: {0}")]
pub struct ConfigLoadError(anyhow::Error);

impl AppConfig {
    pub fn load() -> Result<Self, ConfigLoadError> {
        let config = Config::builder()
            .add_source(File::with_name("config.toml"))
            .add_source(Environment::with_prefix("CIPHER").separator("_"))
            .build()
            .map_err(|e| ConfigLoadError(e.into()))?
            .try_deserialize()
            .map_err(|e| ConfigLoadError(e.into()))?;

        Ok(config)
    }
}
