use anyhow::Result;
use lapin::{
    Channel, ExchangeKind,
    options::{
        BasicAckOptions, BasicConsumeOptions, BasicNackOptions, ExchangeDeclareOptions,
        QueueBindOptions, QueueDeclareOptions,
    },
    types::{AMQPValue, FieldTable},
};
use tracing::error;

pub struct ConsumerConfig {
    pub queue_name: String,
    pub consumer_tag: String,
    pub exchange: String,
    pub routing_keys: Vec<String>,
    /// If set, messages that are nack-rejected will be routed to a dead-letter
    /// exchange + queue instead of being silently dropped.
    pub dead_letter_exchange: Option<String>,
    pub dead_letter_routing_key: Option<String>,
}

pub async fn setup(channel: &Channel, config: &ConsumerConfig) -> Result<lapin::Consumer> {
    let mut queue_args = FieldTable::default();

    if let Some(ref dlx) = config.dead_letter_exchange {
        channel
            .exchange_declare(
                dlx.clone().into(),
                ExchangeKind::Direct,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        queue_args.insert(
            "x-dead-letter-exchange".into(),
            AMQPValue::LongString(dlx.clone().into()),
        );

        let dlrk = config
            .dead_letter_routing_key
            .as_deref()
            .unwrap_or(&config.queue_name);

        queue_args.insert(
            "x-dead-letter-routing-key".into(),
            AMQPValue::LongString(dlrk.into()),
        );

        // Declare and bind the dead-letter queue.
        let dlq_name = format!("{}.dlq", config.queue_name);
        channel
            .queue_declare(
                dlq_name.clone().into(),
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        channel
            .queue_bind(
                dlq_name.into(),
                dlx.clone().into(),
                dlrk.into(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;
    }

    channel
        .queue_declare(
            config.queue_name.clone().into(),
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            queue_args,
        )
        .await?;

    for routing_key in &config.routing_keys {
        channel
            .queue_bind(
                config.queue_name.clone().into(),
                config.exchange.clone().into(),
                routing_key.clone().into(),
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;
    }

    let consumer = channel
        .basic_consume(
            config.queue_name.clone().into(),
            config.consumer_tag.clone().into(),
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    Ok(consumer)
}

pub async fn ack(delivery: &lapin::message::Delivery) {
    if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
        error!(error = ?e, "Failed to ack delivery");
    }
}

pub async fn nack_reject(delivery: &lapin::message::Delivery) {
    if let Err(e) = delivery
        .nack(BasicNackOptions {
            requeue: false,
            ..Default::default()
        })
        .await
    {
        error!(error = ?e, "Failed to nack (reject) delivery");
    }
}

pub async fn nack_requeue(delivery: &lapin::message::Delivery) {
    if let Err(e) = delivery
        .nack(BasicNackOptions {
            requeue: true,
            ..Default::default()
        })
        .await
    {
        error!(error = ?e, "Failed to nack (requeue) delivery");
    }
}
