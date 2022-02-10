table! {
    statistics (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        membership_id -> BigInt,
        unique_visitor -> BigInt,
        referrer -> BigInt,
    }
}
