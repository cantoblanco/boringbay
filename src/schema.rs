table! {
    membership (id) {
        id -> Nullable<Integer>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        homepage -> Text,
        domain -> Text,
        contact -> Text,
        total_referrer -> Nullable<Integer>,
        description -> Text,
    }
}

table! {
    trending (id) {
        id -> Nullable<Integer>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        membership_id -> Nullable<Integer>,
        page_view -> Nullable<Integer>,
        referrer -> Nullable<Integer>,
    }
}

allow_tables_to_appear_in_same_query!(
    membership,
    trending,
);
