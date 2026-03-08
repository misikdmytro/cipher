diesel::table! {
    secrets (id) {
        id -> Uuid,
        path -> Varchar,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}
