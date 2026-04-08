use std::sync::Arc;

use anyhow::Result;
use lapin::{
    BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind,
    options::{BasicPublishOptions, ConfirmSelectOptions, ExchangeDeclareOptions},
    types::FieldTable,
};
use serde::Serialize;

use crate::config::RabbitMqConfig;

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("amqp error: {0}")]
    Amqp(#[from] lapin::Error),

    #[error("publish was not confirmed by broker")]
    NotConfirmed,
}

pub struct AmqpConnection {
    connection: Connection,
}

impl AmqpConnection {
    pub async fn create_channel(&self) -> Result<Channel> {
        Ok(self.connection.create_channel().await?)
    }

    pub async fn create_publish_channel(&self) -> Result<Arc<Channel>> {
        let channel = self.connection.create_channel().await?;
        channel
            .confirm_select(ConfirmSelectOptions::default())
            .await?;
        declare_rotation_exchange(&channel).await?;
        Ok(Arc::new(channel))
    }

    pub async fn create_consume_channel(&self) -> Result<Arc<Channel>> {
        let channel = self.connection.create_channel().await?;
        Ok(Arc::new(channel))
    }
}

pub async fn connect(config: &RabbitMqConfig) -> Result<AmqpConnection> {
    let connection =
        Connection::connect(&config.connection_string(), ConnectionProperties::default()).await?;
    Ok(AmqpConnection { connection })
}

pub async fn declare_rotation_exchange(channel: &Channel) -> Result<()> {
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
    Ok(())
}

pub async fn publish_event(
    channel: &Channel,
    routing_key: &str,
    payload: &impl Serialize,
) -> Result<(), PublishError> {
    let bytes = serde_json::to_vec(payload)?;

    let confirm = channel
        .basic_publish(
            "rotation".into(),
            routing_key.into(),
            BasicPublishOptions::default(),
            &bytes,
            BasicProperties::default()
                .with_content_type("application/json".into())
                .with_delivery_mode(2),
        )
        .await?;

    let confirmation = confirm.await?;
    if !confirmation.is_ack() {
        return Err(PublishError::NotConfirmed);
    }

    Ok(())
}
