use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize)]
pub struct PaginationParams {
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct PaginatedResponse<T: Serialize + utoipa::ToSchema> {
    pub items: Vec<T>,
    pub total: u64,
}
