table! {
    membership (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        homepage -> Text,
        domain -> Text,
        contact -> Text,
        total_referrer -> BigInt,
        description -> Text,
    }
}

table! {
    trending (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        membership_id -> BigInt,
        page_view -> BigInt,
        referrer -> BigInt,
    }
}

allow_tables_to_appear_in_same_query!(
    membership,
    trending,
);
