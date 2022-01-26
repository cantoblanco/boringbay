use askama::Template;
use axum::{
    extract::{Extension, Path},
    response::{Headers, Html, IntoResponse},
};
use headers::HeaderMap;

use crate::app_model::DynContext;

pub async fn show_badge(
    Path(domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> impl IntoResponse {
    let tend = ctx.boring_vistor(&domain, &headers).await;
    let headers = Headers([("content-type", "image/svg+xml")]);
    let len: usize = 10;
    let read = ctx.badge_render_cache.read().await;
    let cache = read.get(&len);
    let content = if let Some(v) = cache {
        v.clone()
    } else {
        drop(read);
        let v = ctx.badge.render_svg(tend as usize);
        let mut write = ctx.badge_render_cache.write().await;
        write.insert(len, v.clone());
        v
    };
    (headers, content)
}

pub async fn show_favicon(
    Path(domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> impl IntoResponse {
    let tend = ctx.boring_vistor(&domain, &headers).await;
    let headers = Headers([("content-type", "image/svg+xml")]);
    let len: usize = 10;
    let read = ctx.favicon_render_cache.read().await;
    let cache = read.get(&len);
    let content = if let Some(v) = cache {
        v.clone()
    } else {
        drop(read);
        let v = ctx.favicon.render_svg(tend as usize);
        let mut write = ctx.favicon_render_cache.write().await;
        write.insert(len, v.clone());
        v
    };
    (headers, content)
}

pub async fn show_icon(
    Path(domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> impl IntoResponse {
    let tend = ctx.boring_vistor(&domain, &headers).await;
    let headers = Headers([("content-type", "image/svg+xml")]);
    let len: usize = 10;
    let read = ctx.icon_render_cache.read().await;
    let cache = read.get(&len);
    let content = if let Some(v) = cache {
        v.clone()
    } else {
        drop(read);
        let v = ctx.icon.render_svg(tend as usize);
        let mut write = ctx.icon_render_cache.write().await;
        write.insert(len, v.clone());
        v
    };
    (headers, content)
}

pub async fn home_page() -> Result<Html<String>, String> {
    let tpl = HelloTemplate {};
    let html = tpl.render().map_err(|err| err.to_string())?;
    Ok(Html(html))
}

#[derive(Template)]
#[template(path = "index.html")]
struct HelloTemplate {}
