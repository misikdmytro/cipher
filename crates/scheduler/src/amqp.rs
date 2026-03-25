use std::sync::Arc;

use anyhow::Result;
use lapin::{
    Channel, Connection, ConnectionProperties, ExchangeKind, options::ExchangeDeclareOptions,
    types::FieldTable,
};

use crate::config::RabbitMqConfig;

pub async fn connect(config: &RabbitMqConfig) -> Result<Arc<Channel>> {
    let conn =
        Connection::connect(&config.connection_string(), ConnectionProperties::default()).await?;
    let channel = Arc::new(conn.create_channel().await?);

    channel
        .exchange_declare(
            "rotation".into(),
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    Ok(channel)
}
