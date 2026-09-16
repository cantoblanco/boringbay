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

diesel::table! {
    site_health (id) {
        id -> Integer,
        member_id -> BigInt,
        checked_at -> Timestamp,
        reachable -> Bool,
        status_code -> Nullable<Integer>,
        badge_state -> Text,
    }
}

diesel::table! {
    daily_routes (route_date) {
        route_date -> Date,
        member_ids -> Text,
        generated_at -> Timestamp,
    }
}

diesel::table! {
    product_events (event_date, event_kind, member_id) {
        event_date -> Date,
        event_kind -> Text,
        member_id -> BigInt,
        count -> BigInt,
    }
}

diesel::table! {
    feed_sources (member_id) {
        member_id -> BigInt,
        feed_url -> Text,
        last_attempt -> Nullable<Timestamp>,
        last_success -> Nullable<Timestamp>,
        last_error -> Nullable<Text>,
    }
}

diesel::table! {
    feed_items (id) {
        id -> Integer,
        member_id -> BigInt,
        item_key -> Text,
        title -> Text,
        url -> Text,
        summary -> Text,
        published_at -> Timestamp,
        fetched_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    statistics,
    site_health,
    daily_routes,
    product_events,
    feed_sources,
    feed_items
);
