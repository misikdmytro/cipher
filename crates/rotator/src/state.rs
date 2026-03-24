use crate::config::AppConfig;
use crate::services::rotation::RotationService;

pub struct AppState {
    pub config: AppConfig,
    pub rotation_service: Box<dyn RotationService>,
}
