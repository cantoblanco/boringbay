mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body::Body as _;
use tower::ServiceExt;

async fn page(uri: &str) -> String {
    let (_tmp, app) = common::temporary_app().await;
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    while let Some(chunk) = body.data().await {
        bytes.extend_from_slice(&chunk.unwrap());
    }
    String::from_utf8(bytes).unwrap()
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
    let mut body = home.into_body();
    let mut bytes = Vec::new();
    while let Some(chunk) = body.data().await {
        bytes.extend_from_slice(&chunk.unwrap());
    }
    let html = String::from_utf8(bytes).unwrap();
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
async fn status_observation_is_transparent_and_never_promises_auto_deletion() {
    let html = page("/rank").await;
    assert!(html.contains("成员状态观察"));
    assert!(html.contains("不会自动删除成员"));
    assert!(html.contains("尚未检测"));
    assert!(!html.contains("<del>"));
}
