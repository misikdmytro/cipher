// @generated automatically by Diesel CLI.

diesel::table! {
    secrets (id) {
        id -> Uuid,
        #[max_length = 255]
        path -> Varchar,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}
