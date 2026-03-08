use diesel::prelude::*;
use uuid::Uuid;

use crate::repositories::models::schema::secrets;

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = secrets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Secret {
    pub id: Uuid,
    pub path: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<Secret> for interfaces::secrets::Secret {
    fn from(secret: Secret) -> Self {
        Self {
            id: secret.id,
            path: secret.path,
            created_at: secret.created_at,
            updated_at: secret.updated_at,
        }
    }
}
