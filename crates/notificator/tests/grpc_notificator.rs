mod common;

use std::sync::Arc;

use common::mocks::MockWebhookDeliveryService;
use notificator::config::{AppConfig, DatabaseConfig, HostingConfig};
use notificator::grpc::notificator_service::new_notificator_grpc_service;
use notificator::repositories::webhooks::new_webhooks_repository;
use notificator::state::AppState;
use proto::notificator::RegisterWebhookRequest;
use proto::notificator::notificator_service_client::NotificatorServiceClient;
use proto::notificator::notificator_service_server::NotificatorServiceServer;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use uuid::Uuid;

struct TestApp {
    client: NotificatorServiceClient<tonic::transport::Channel>,
    db: sqlx::PgPool,
}

impl TestApp {
    async fn spawn() -> Self {
        let config = AppConfig {
            database: test_db_config(),
            grpc: HostingConfig::default(),
            rabbitmq: Default::default(),
        };

        let repository = new_webhooks_repository(&config)
            .await
            .expect("failed to connect to test database — is PostgreSQL running?");

        let db = sqlx::PgPool::connect(&config.database.connection_string())
            .await
            .unwrap();

        let delivery_service = MockWebhookDeliveryService::success();

        let amqp_channel = Arc::new(
            lapin::Connection::connect(
                &config.rabbitmq.connection_string(),
                lapin::ConnectionProperties::default(),
            )
            .await
            .expect("failed to connect to RabbitMQ — is it running?")
            .create_channel()
            .await
            .unwrap(),
        );

        let state = Arc::new(AppState {
            config,
            webhooks_repository: Box::new(repository),
            delivery_service: Box::new(delivery_service),
            amqp_channel,
        });

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let grpc_svc = new_notificator_grpc_service(state);
        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(NotificatorServiceServer::new(grpc_svc))
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap();
        });

        let endpoint = format!("http://127.0.0.1:{}", port);
        let channel = tonic::transport::Endpoint::new(endpoint)
            .unwrap()
            .connect_lazy();
        let client = NotificatorServiceClient::new(channel);

        TestApp { client, db }
    }

    async fn webhook_exists(&self, webhook_id: Uuid) -> bool {
        sqlx::query("SELECT 1 FROM webhooks WHERE id = $1")
            .bind(webhook_id)
            .fetch_optional(&self.db)
            .await
            .unwrap()
            .is_some()
    }
}

fn test_db_config() -> DatabaseConfig {
    DatabaseConfig {
        username: std::env::var("TEST_DB_USER").unwrap_or_else(|_| "cipher".into()),
        password: std::env::var("TEST_DB_PASSWORD").unwrap_or_else(|_| "cipher".into()),
        host: std::env::var("TEST_DB_HOST").unwrap_or_else(|_| "localhost".into()),
        port: std::env::var("TEST_DB_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5432),
        database: std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "notificator".into()),
        use_ssl: false,
    }
}

#[tokio::test]
async fn register_webhook_returns_valid_uuid() {
    let mut app = TestApp::spawn().await;

    let response = app
        .client
        .register_webhook(RegisterWebhookRequest {
            secret_id: Uuid::new_v4().to_string(),
            url: "https://example.com/webhook".to_string(),
        })
        .await
        .unwrap();

    let webhook_id = Uuid::parse_str(&response.into_inner().webhook_id)
        .expect("webhook_id should be a valid UUID");

    assert!(app.webhook_exists(webhook_id).await);
}

#[tokio::test]
async fn register_webhook_with_invalid_uuid_returns_invalid_argument() {
    let mut app = TestApp::spawn().await;

    let result = app
        .client
        .register_webhook(RegisterWebhookRequest {
            secret_id: "not-a-uuid".to_string(),
            url: "https://example.com/webhook".to_string(),
        })
        .await;

    let err = result.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn register_webhook_with_empty_url_returns_invalid_argument() {
    let mut app = TestApp::spawn().await;

    let result = app
        .client
        .register_webhook(RegisterWebhookRequest {
            secret_id: Uuid::new_v4().to_string(),
            url: "".to_string(),
        })
        .await;

    let err = result.unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}
