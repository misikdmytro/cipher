use std::sync::{Arc, Mutex};

use api::config::{AppConfig, DatabaseConfig, HostingConfig};
use api::handlers;
use api::repositories::secrets::new_secrets_repository;
use api::services::secrets::new_secrets_service;
use api::state::AppState;
use proto::scheduler::{
    ScheduleSecretRotationRequest,
    scheduler_service_server::{SchedulerService, SchedulerServiceServer},
};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

pub struct MockSchedulerService {
    should_fail: Mutex<bool>,
    received: Mutex<Vec<ScheduleSecretRotationRequest>>,
}

impl MockSchedulerService {
    pub fn new(should_fail: bool) -> Self {
        Self {
            should_fail: Mutex::new(should_fail),
            received: Mutex::new(Vec::new()),
        }
    }

    pub fn received_requests(&self) -> Vec<ScheduleSecretRotationRequest> {
        self.received.lock().unwrap().clone()
    }
}

#[tonic::async_trait]
impl SchedulerService for MockSchedulerService {
    async fn schedule_secret_rotation(
        &self,
        request: tonic::Request<ScheduleSecretRotationRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let req = request.into_inner();
        self.received.lock().unwrap().push(req);

        if *self.should_fail.lock().unwrap() {
            return Err(tonic::Status::internal("mock scheduler failure"));
        }

        Ok(tonic::Response::new(()))
    }
}

pub struct TestApp {
    pub base_url: String,
    pub http: reqwest::Client,
    pub db: sqlx::PgPool,
    pub scheduler: Arc<MockSchedulerService>,
}

impl TestApp {
    pub async fn spawn() -> Self {
        Self::build(false).await
    }

    pub async fn spawn_with_failing_scheduler() -> Self {
        Self::build(true).await
    }

    async fn build(scheduler_fails: bool) -> Self {
        let mock = Arc::new(MockSchedulerService::new(scheduler_fails));

        let grpc_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let grpc_port = grpc_listener.local_addr().unwrap().port();

        let mock_clone = Arc::clone(&mock);
        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(SchedulerServiceServer::new(mock_clone))
                .serve_with_incoming(TcpListenerStream::new(grpc_listener))
                .await
                .unwrap();
        });

        let config = AppConfig {
            database: test_db_config(),
            scheduler: HostingConfig {
                scheme: "http".into(),
                host: "127.0.0.1".into(),
                port: grpc_port,
            },
            api: HostingConfig {
                scheme: "http".into(),
                host: "127.0.0.1".into(),
                port: 0,
            },
        };

        let repository = new_secrets_repository(&config)
            .await
            .expect("failed to connect to test database — is PostgreSQL running?");

        let db = sqlx::PgPool::connect(&config.database.connection_string())
            .await
            .unwrap();

        let grpc_endpoint = format!("http://127.0.0.1:{}", grpc_port);
        let channel = tonic::transport::Endpoint::new(grpc_endpoint)
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
