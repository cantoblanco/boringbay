table! {
    statistics (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        membership_id -> BigInt,
        page_view -> BigInt,
        referrer -> BigInt,
    }
}
