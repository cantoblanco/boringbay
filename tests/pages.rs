mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text, Timestamp};
use http_body_util::BodyExt as _;
use tower::ServiceExt;
use naive::analytics::{AnalyticsEvent, AnalyticsEventKind, AnalyticsService};

async fn body_text(body: Body) -> String {
    let bytes = body.collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn page(uri: &str) -> String {
    let (_tmp, app) = common::temporary_app().await;
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    body_text(response.into_body()).await
}

#[tokio::test]
async fn home_preserves_brand_member_metrics_and_join_paths() {
    let html = page("/").await;
    for marker in ["无聊湾", "今日无聊", "UV", "RV", "排行榜", "一起无聊？"] {
        assert!(html.contains(marker), "missing {marker}");
    }
    assert!(html.contains("/static/app.css"));
    assert!(html.contains("/static/app.js"));
    assert!(html.contains("/static/discovery.js"));
    assert!(html.contains("data-activity-toasts"));
    assert_eq!(html.matches("data-member-card").count(), 0);
    assert!(html.contains("当前没有需要处理的成员站点"));
}

#[tokio::test]
async fn home_only_lists_members_with_activity_today() {
    let (_tmp, app) = common::temporary_app_with_setup(true, |pool| {
        let today = naive::now_shanghai().date().and_hms_opt(0, 0, 0).unwrap();
        diesel::sql_query(
            "INSERT INTO statistics \
             (created_at, updated_at, membership_id, unique_visitor, referrer, latest_referrer_at) \
             VALUES (?1, ?2, 1, 3, 0, NULL)",
        )
        .bind::<Timestamp, _>(today)
        .bind::<Timestamp, _>(naive::now_shanghai())
        .execute(&mut pool.get().unwrap())
        .unwrap();
    })
    .await;

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = body_text(response.into_body()).await;
    assert_eq!(html.matches("data-member-card").count(), 1);
    assert!(html.contains("data-member-id=\"1\""));
    assert!(html.contains("UV3"));
    assert!(html.contains("RV0"));
}

#[tokio::test]
async fn route_page_has_five_member_links_and_passport_assets() {
    let html = page("/route/2026-09-16").await;
    assert_eq!(html.matches("class=\"route-stop\"").count(), 5);
    assert!(html.contains("0 / 5 枚邮戳"));
    assert!(html.contains("/static/discovery.js"));
}

#[tokio::test]
async fn v1_home_has_no_v2_controls_and_route_is_hidden() {
    let (_tmp, app) = common::temporary_app_with_v2(false).await;
    let home = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let html = body_text(home.into_body()).await;
    assert!(!html.contains("data-random-discovery"));

    let route = app
        .oneshot(
            Request::builder()
                .uri("/route/2026-09-16")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(route.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rankings_offer_classic_activity_and_rising_views() {
    for (uri, expected) in [
        ("/rank", "经典总榜"),
        ("/rank?view=activity", "30 天活跃榜"),
        ("/rank?view=rising", "7 天上升榜"),
    ] {
        let html = page(uri).await;
        assert!(html.contains(expected), "{uri} missing {expected}");
        assert!(html.contains("统计窗口"));
    }
}

#[tokio::test]
async fn rankings_accept_legacy_rows_with_null_referrer_timestamp() {
    let (_tmp, app) = common::temporary_app_with_setup(true, |pool| {
        let created = naive::now_shanghai() - chrono::Duration::days(1);
        diesel::sql_query(
            "INSERT INTO statistics \
             (created_at, updated_at, membership_id, unique_visitor, referrer, latest_referrer_at) \
             VALUES (?1, ?2, 1, 7, 0, NULL)",
        )
        .bind::<Timestamp, _>(created)
        .bind::<Timestamp, _>(created)
        .execute(&mut pool.get().unwrap())
        .unwrap();
    })
    .await;

    for uri in ["/rank", "/rank?view=activity", "/rank?view=rising"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        let html = body_text(response.into_body()).await;
        assert!(!html.contains("UnexpectedNullError"), "{uri}");
    }
}

#[tokio::test]
async fn status_observation_is_transparent_and_never_promises_auto_deletion() {
    let html = page("/rank").await;
    assert!(html.contains("成员状态观察"));
    assert!(html.contains("不会自动删除成员"));
    assert!(html.contains("尚未检测"));
    assert!(!html.contains("<del>"));
}

#[tokio::test]
async fn cached_feed_items_appear_as_safe_outbound_drift_bottles() {
    let (tmp, app) = common::temporary_app().await;
    let pool = naive::establish_connection(tmp.path().join("test.db").to_str().unwrap());
    let now = naive::now_shanghai();
    diesel::sql_query(
        "INSERT INTO feed_items (member_id, item_key, title, url, summary, published_at, fetched_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind::<BigInt, _>(1_i64)
    .bind::<Text, _>("test-item")
    .bind::<Text, _>("A cached post")
    .bind::<Text, _>("https://example.com/post")
    .bind::<Text, _>("Short safe summary")
    .bind::<Timestamp, _>(now)
    .bind::<Timestamp, _>(now)
    .execute(&mut pool.get().unwrap())
    .unwrap();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let html = body_text(response.into_body()).await;
    assert!(html.contains("A cached post"));
    assert!(html.contains("Short safe summary"));
    assert!(html.contains("class=\"feed-outbound\""));
    assert!(html.contains("rel=\"noopener noreferrer\""));
}

#[tokio::test]
async fn route_has_share_metadata_and_safe_svg_card() {
    let (_tmp, app) = common::temporary_app().await;
    let route = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/route/2026-09-16")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(route.status(), StatusCode::OK);
    let html = body_text(route.into_body()).await;
    assert!(html.contains("property=\"og:image\""));
    assert!(html.contains("data-share-route"));

    let image = app
        .oneshot(
            Request::builder()
                .uri("/api/share/route/2026-09-16.svg")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(image.status(), StatusCode::OK);
    assert_eq!(
        image.headers().get("content-type").unwrap(),
        "image/svg+xml; charset=utf-8"
    );
}

#[tokio::test]
async fn old_badge_stays_available_and_v2_endpoints_follow_feature_flag() {
    let (_tmp, app) = common::temporary_app().await;
    for uri in ["/api/badge/boringbay.com", "/api/badge-v2/boringbay.com"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
    }

    let (_tmp, v1) = common::temporary_app_with_v2(false).await;
    for uri in [
        "/api/badge-v2/boringbay.com",
        "/api/share/route/2026-09-16.svg",
    ] {
        let response = v1
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn public_analytics_pages_render_filtered_aggregate_data() {
    let (_tmp, app) = common::temporary_app_with_setup(true, |pool| {
        let service = AnalyticsService::new(pool.clone());
        for _ in 0..3 {
            service
                .record(AnalyticsEvent {
                    at: naive::now_shanghai(),
                    member_id: 1,
                    kind: AnalyticsEventKind::BadgeView,
                    country: Some("ES".to_string()),
                    referrer_domain: None,
                    channel: None,
                })
                .unwrap();
        }
    })
    .await;

    for uri in [
        "/analytics",
        "/analytics?range=7",
        "/analytics?range=nonsense",
        "/analytics/lifelonglearn.ing?range=90",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        let html = body_text(response.into_body()).await;
        assert!(html.contains("公开流量分析") || html.contains("MEMBER ANALYTICS"));
        assert!(html.contains("ES"));
        assert!(html.contains("不足 3 次"));
    }

    let missing = app
        .oneshot(
            Request::builder()
                .uri("/analytics/not-a-member.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn analytics_routes_follow_v2_feature_flag() {
    let (_tmp, app) = common::temporary_app_with_v2(false).await;
    for uri in ["/analytics", "/analytics/lifelonglearn.ing"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn badge_dedupe_writes_one_traffic_total() {
    let (tmp, app) = common::temporary_app().await;
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/badge/lifelonglearn.ing")
                    .header("referer", "https://lifelonglearn.ing/post")
                    .header("CF-Connecting-IP", "203.0.113.9")
                    .header("CF-IPCountry", "ES")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let pool = naive::establish_connection(tmp.path().join("test.db").to_str().unwrap());
    let count = naive::schema::traffic_daily::table
        .filter(naive::schema::traffic_daily::member_id.eq(1_i64))
        .filter(naive::schema::traffic_daily::event_kind.eq("badge_view"))
        .filter(naive::schema::traffic_daily::dimension_kind.eq("total"))
        .select(naive::schema::traffic_daily::count)
        .first::<i64>(&mut pool.get().unwrap())
        .unwrap();
    assert_eq!(count, 1);
}
