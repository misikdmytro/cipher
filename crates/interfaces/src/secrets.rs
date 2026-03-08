use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret {
    pub id: Uuid,
    pub path: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}
