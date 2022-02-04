use anyhow::anyhow;
use askama::Template;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{Headers, Html, IntoResponse},
};
use headers::HeaderMap;

use crate::{app_model::DynContext, membership_model::Membership};

pub async fn show_badge(
    Path(domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> impl IntoResponse {
    let tend = ctx
        .boring_vistor(crate::app_model::VistorType::Badge, &domain, &headers)
        .await;
    if tend.is_err() {
        return (
            StatusCode::NOT_FOUND,
            Headers([("content-type", "text/plain")]),
            tend.err().unwrap().to_string(),
        );
    }
    let headers = Headers([("content-type", "image/svg+xml")]);
    let len: usize = 10;
    let read = ctx.badge_render_cache.read().await;
    let cache = read.get(&len);
    let content = if let Some(v) = cache {
        v.clone()
    } else {
        drop(read);
        let v = ctx.badge.render_svg(tend.unwrap() as usize);
        let mut write = ctx.badge_render_cache.write().await;
        write.insert(len, v.clone());
        v
    };
    (StatusCode::OK, headers, content)
}

pub async fn show_favicon(
    Path(domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> impl IntoResponse {
    let tend = ctx
        .boring_vistor(crate::app_model::VistorType::ICON, &domain, &headers)
        .await;
    if tend.is_err() {
        return (
            StatusCode::NOT_FOUND,
            Headers([("content-type", "text/plain")]),
            tend.err().unwrap().to_string(),
        );
    }
    let headers = Headers([("content-type", "image/svg+xml")]);
    let len: usize = 10;
    let read = ctx.favicon_render_cache.read().await;
    let cache = read.get(&len);
    let content = if let Some(v) = cache {
        v.clone()
    } else {
        drop(read);
        let v = ctx.favicon.render_svg(tend.unwrap() as usize);
        let mut write = ctx.favicon_render_cache.write().await;
        write.insert(len, v.clone());
        v
    };
    (StatusCode::OK, headers, content)
}

pub async fn show_icon(
    Path(domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> impl IntoResponse {
    let tend = ctx
        .boring_vistor(crate::app_model::VistorType::ICON, &domain, &headers)
        .await;
    if tend.is_err() {
        return (
            StatusCode::NOT_FOUND,
            Headers([("content-type", "text/plain")]),
            tend.err().unwrap().to_string(),
        );
    }
    let headers = Headers([("content-type", "image/svg+xml")]);
    let len: usize = 10;
    let read = ctx.icon_render_cache.read().await;
    let cache = read.get(&len);
    let content = if let Some(v) = cache {
        v.clone()
    } else {
        drop(read);
        let v = ctx.icon.render_svg(tend.unwrap() as usize);
        let mut write = ctx.icon_render_cache.write().await;
        write.insert(len, v.clone());
        v
    };
    (StatusCode::OK, headers, content)
}

pub async fn home_page(
    Extension(ctx): Extension<DynContext>,
    headers: HeaderMap,
) -> Result<Html<String>, String> {
    let domain = get_domain_from_headers(&headers);
    if domain.is_ok() {
        let _ = ctx
            .boring_vistor(
                crate::app_model::VistorType::Referrer,
                &domain.unwrap(),
                &headers,
            )
            .await;
    }
    let referrer_read = ctx.referrer.read().await;
    let pv_read = ctx.page_view.read().await;

    let mut rank_vec: Vec<(i64, i64)> = Vec::new();

    for k in ctx.id2member.keys() {
        rank_vec.push((
            k.to_owned(),
            referrer_read.get(k).unwrap_or(&0).to_owned() * 5
                + pv_read.get(k).unwrap_or(&0).to_owned(),
        ));
    }

    rank_vec.sort_by(|a, b| b.1.cmp(&a.1));

    let mut membership = Vec::new();
    for v in rank_vec {
        membership.push(ctx.id2member.get(&v.0).unwrap().to_owned());
    }

    let tpl = HomeTemplate { membership };
    let html = tpl.render().map_err(|err| err.to_string())?;
    Ok(Html(html))
}

fn get_domain_from_headers(headers: &HeaderMap) -> Result<String, anyhow::Error> {
    let referrer_header = headers.get("Referer");
    if referrer_header.is_none() {
        return Err(anyhow!("no referrer header"));
    }

    let referrer_str = String::from_utf8(referrer_header.unwrap().as_bytes().to_vec());
    if referrer_str.is_err() {
        return Err(anyhow!("referrer header is not valid utf-8 string"));
    }

    let referrer_url = url::Url::parse(&referrer_str.unwrap());
    if referrer_url.is_err() {
        return Err(anyhow!("referrer header is not valid URL"));
    }

    let referrer_url = referrer_url.unwrap();
    if referrer_url.domain().is_none() {
        return Err(anyhow!("referrer header doesn't contains a valid domain"));
    }

    return Ok(referrer_url.domain().unwrap().to_string());
}

#[derive(Template)]
#[template(path = "index.html")]
struct HomeTemplate {
    membership: Vec<Membership>,
}
