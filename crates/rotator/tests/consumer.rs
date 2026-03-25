mod common;

use std::sync::Mutex;

use common::mocks::MockRotationEventPublisher;
use rotator::consumer::{ProcessError, process_rotation};
use rotator::services::rotation::RotationService;
use rotator::services::types::{ServiceError, ServiceResult};
use uuid::Uuid;

struct MockRotationService {
    result: Mutex<Option<ServiceResult<()>>>,
    calls: Mutex<Vec<Uuid>>,
}

impl MockRotationService {
    fn success() -> Self {
        Self {
            result: Mutex::new(Some(Ok(()))),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn not_found() -> Self {
        Self {
            result: Mutex::new(Some(Err(ServiceError::NotFound))),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn aws_error(msg: &str) -> Self {
        Self {
            result: Mutex::new(Some(Err(ServiceError::AwsError(msg.to_string())))),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn calls(&self) -> Vec<Uuid> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl RotationService for MockRotationService {
    async fn rotate(&self, secret_id: Uuid) -> ServiceResult<()> {
        self.calls.lock().unwrap().push(secret_id);
        self.result
            .lock()
            .unwrap()
            .take()
            .expect("MockRotationService: no result configured")
    }
}

#[tokio::test]
async fn publishes_started_and_done_on_success() {
    let secret_id = Uuid::new_v4();
    let rotation = MockRotationService::success();
    let publisher = MockRotationEventPublisher::success();

    let result = process_rotation(secret_id, &rotation, &publisher).await;

    assert!(result.is_ok());
    assert_eq!(publisher.started_calls(), vec![secret_id]);
    assert_eq!(publisher.done_calls(), vec![secret_id]);
    assert!(publisher.failed_calls().is_empty());
}

#[tokio::test]
async fn publishes_started_and_done_on_not_found() {
    let secret_id = Uuid::new_v4();
    let rotation = MockRotationService::not_found();
    let publisher = MockRotationEventPublisher::success();

    let result = process_rotation(secret_id, &rotation, &publisher).await;

    assert!(matches!(result, Err(ProcessError::NotFound)));
    assert_eq!(publisher.started_calls(), vec![secret_id]);
    assert_eq!(publisher.done_calls(), vec![secret_id]);
    assert!(publisher.failed_calls().is_empty());
}

#[tokio::test]
async fn publishes_started_and_failed_on_rotation_error() {
    let secret_id = Uuid::new_v4();
    let rotation = MockRotationService::aws_error("access denied");
    let publisher = MockRotationEventPublisher::success();

    let result = process_rotation(secret_id, &rotation, &publisher).await;

    assert!(matches!(result, Err(ProcessError::Rotation(_))));
    assert_eq!(publisher.started_calls(), vec![secret_id]);
    assert!(publisher.done_calls().is_empty());

    let failed = publisher.failed_calls();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].0, secret_id);
    assert_eq!(failed[0].1, "aws error: access denied");
}

#[tokio::test]
async fn does_not_rotate_when_publish_started_fails() {
    let secret_id = Uuid::new_v4();
    let rotation = MockRotationService::success();
    let publisher = MockRotationEventPublisher::started_fails();

    let result = process_rotation(secret_id, &rotation, &publisher).await;

    assert!(matches!(result, Err(ProcessError::Publish(_))));
    assert!(rotation.calls().is_empty());
    assert!(publisher.done_calls().is_empty());
    assert!(publisher.failed_calls().is_empty());
}
