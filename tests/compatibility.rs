mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn old_pages_and_api_routes_remain_registered() {
    let (_tmp, app) = common::temporary_app().await;
    for uri in [
        "/",
        "/rank",
        "/join-us",
        "/api/icon/boringbay.com",
        "/api/favicon/boringbay.com",
        "/api/badge/boringbay.com",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND, "{uri}");
    }
}

#[tokio::test]
async fn missing_cloudflare_headers_never_crash_svg_endpoints() {
    let (_tmp, app) = common::temporary_app().await;
    for uri in [
        "/api/icon/boringbay.com",
        "/api/favicon/boringbay.com",
        "/api/badge/boringbay.com",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
    }
}
