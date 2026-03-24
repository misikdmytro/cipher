pub mod mocks;

use std::sync::Arc;

use api::config::{AppConfig, DatabaseConfig, HostingConfig};
use api::grpc::api_service::new_api_grpc_service;
use api::handlers;
use api::repositories::secrets::new_secrets_repository;
use api::services::secrets::new_secrets_service;
use api::state::AppState;
use mocks::MockSchedulerService;
use proto::api::api_service_client::ApiServiceClient;
use proto::api::api_service_server::ApiServiceServer;
use proto::scheduler::scheduler_service_server::SchedulerServiceServer;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Channel;

pub struct TestApp {
    pub base_url: String,
    pub http: reqwest::Client,
    pub db: sqlx::PgPool,
    pub scheduler: Arc<MockSchedulerService>,
    pub api_grpc_client: ApiServiceClient<Channel>,
}

impl TestApp {
    pub async fn spawn() -> Self {
        Self::build(MockSchedulerService::success()).await
    }

    pub async fn spawn_with_failing_scheduler() -> Self {
        Self::build(MockSchedulerService::failing()).await
    }

    async fn build(scheduler_mock: MockSchedulerService) -> Self {
        let mock = Arc::new(scheduler_mock);

        // Start mock scheduler gRPC server
        let scheduler_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let scheduler_port = scheduler_listener.local_addr().unwrap().port();

        let mock_clone = Arc::clone(&mock);
        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(SchedulerServiceServer::from_arc(mock_clone))
                .serve_with_incoming(TcpListenerStream::new(scheduler_listener))
                .await
                .unwrap();
        });

        let config = AppConfig {
            database: test_db_config(),
            scheduler: HostingConfig {
                port: scheduler_port,
                ..Default::default()
            },
            api: Default::default(),
            grpc: Default::default(),
        };

        let repository = new_secrets_repository(&config)
            .await
            .expect("failed to connect to test database — is PostgreSQL running?");

        let db = sqlx::PgPool::connect(&config.database.connection_string())
            .await
            .unwrap();

        let scheduler_endpoint = format!("http://127.0.0.1:{}", scheduler_port);
        let channel = tonic::transport::Endpoint::new(scheduler_endpoint)
            .unwrap()
            .connect_lazy();
        let scheduler_client = Arc::new(tokio::sync::Mutex::new(
            proto::scheduler::scheduler_service_client::SchedulerServiceClient::new(channel),
        ));
        let secrets_service = new_secrets_service(Box::new(repository), scheduler_client);

        let state = Arc::new(AppState {
            config,
            secrets_service: Box::new(secrets_service),
        });

        // Start API gRPC server
        let api_grpc_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let api_grpc_port = api_grpc_listener.local_addr().unwrap().port();

        let grpc_svc = new_api_grpc_service(state.clone());
        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(ApiServiceServer::new(grpc_svc))
                .serve_with_incoming(TcpListenerStream::new(api_grpc_listener))
                .await
                .unwrap();
        });

        let api_grpc_endpoint = format!("http://127.0.0.1:{}", api_grpc_port);
        let api_grpc_channel = tonic::transport::Endpoint::new(api_grpc_endpoint)
            .unwrap()
            .connect_lazy();
        let api_grpc_client = ApiServiceClient::new(api_grpc_channel);

        // Start HTTP server
        let http_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http_port = http_listener.local_addr().unwrap().port();
        let base_url = format!("http://127.0.0.1:{}", http_port);

        let router = handlers::router(state);
        tokio::spawn(async move {
            axum::serve(http_listener, router).await.unwrap();
        });

        TestApp {
            base_url,
            http: reqwest::Client::new(),
            db,
            scheduler: mock,
            api_grpc_client,
        }
    }

    pub async fn post_secret(&self, body: serde_json::Value) -> reqwest::Response {
        self.http
            .post(format!("{}/secrets", self.base_url))
            .json(&body)
            .send()
            .await
            .expect("HTTP request to /secrets failed")
    }

    pub async fn secret_exists(&self, id: uuid::Uuid) -> bool {
        sqlx::query("SELECT 1 FROM secrets WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db)
            .await
            .unwrap()
            .is_some()
    }

    pub async fn secret_exists_by_path(&self, path: &str) -> bool {
        sqlx::query("SELECT 1 FROM secrets WHERE path = $1")
            .bind(path)
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
        database: std::env::var("TEST_DB_NAME").unwrap_or_else(|_| "api".into()),
        use_ssl: false,
    }
}
