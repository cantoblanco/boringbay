use std::collections::HashMap;

use axum::{
    extract::{Extension, Path},
    response::{Headers, IntoResponse},
};

use crate::app_model::DynContext;

pub async fn badge_show(
    Path(domain): Path<String>,
    Headers(headers): Headers<HashMap<String, String>>,
    Extension(ctx): Extension<DynContext>,
) -> impl IntoResponse {
    let tend = ctx
        .boring_vistor(&domain, headers.get("X-Forwarded-For").unwrap())
        .await;
    let headers = Headers([("content-type", "image/svg+xml")]);
    let len: usize = 10;
    let read = ctx.render_cache.read().await;
    let cache = read.get(&len);
    let content = if let Some(v) = cache {
        v.clone()
    } else {
        drop(read);
        let v = ctx.badge.render_svg(tend as usize);
        let mut write = ctx.render_cache.write().await;
        write.insert(len, v.clone());
        v
    };
    (headers, content)
}

pub async fn badge_reverse_show(
    Path(domain): Path<String>,
    Headers(headers): Headers<HashMap<String, String>>,
    Extension(ctx): Extension<DynContext>,
) -> impl IntoResponse {
    let tend = ctx
        .boring_vistor(&domain, headers.get("X-Forwarded-For").unwrap())
        .await;
    let headers = Headers([("content-type", "image/svg+xml")]);
    let len: usize = 10;
    let read = ctx.render_reverse_cache.read().await;
    let cache = read.get(&len);
    let content = if let Some(v) = cache {
        v.clone()
    } else {
        drop(read);
        let v = ctx.badge_reverse.render_svg(tend as usize);
        let mut write = ctx.render_reverse_cache.write().await;
        write.insert(len, v.clone());
        v
    };
    (headers, content)
}
