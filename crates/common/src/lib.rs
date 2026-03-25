pub mod amqp;
pub mod config;
pub mod consumer;
pub mod db;
pub mod health;
pub mod shutdown;

#[macro_export]
macro_rules! repo_error {
    ($name:ident) => {
        #[derive(Debug, thiserror::Error)]
        pub enum $name {
            #[error("infrastructure error: {0}")]
            Infrastructure(anyhow::Error),
        }

        impl From<sqlx::Error> for $name {
            fn from(e: sqlx::Error) -> Self {
                $name::Infrastructure(e.into())
            }
        }
    };
    ($name:ident, NotFound) => {
        #[derive(Debug, thiserror::Error)]
        pub enum $name {
            #[error("not found")]
            NotFound,

            #[error("infrastructure error: {0}")]
            Infrastructure(anyhow::Error),
        }

        impl From<sqlx::Error> for $name {
            fn from(e: sqlx::Error) -> Self {
                $name::Infrastructure(e.into())
            }
        }
    };
}
