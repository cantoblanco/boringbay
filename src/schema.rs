// @generated automatically by Diesel CLI.

diesel::table! {
    statistics (id) {
        id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        membership_id -> BigInt,
        unique_visitor -> BigInt,
        referrer -> BigInt,
        latest_referrer_at -> Timestamp,
    }
}
