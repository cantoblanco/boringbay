use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::anyhow;
use askama::Template;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Extension, Path, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{Headers, Html, IntoResponse, Response},
};
use headers::HeaderMap;
use tokio::select;

use crate::{
    app_model::{Context, DynContext},
    boring_face::BoringFace,
    membership_model::{Membership, RankAndMembership},
    now_shanghai, GIT_HASH,
};

pub async fn ws_upgrade(
    Extension(ctx): Extension<DynContext>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(ctx, socket))
}

async fn handle_socket(ctx: Arc<Context>, mut socket: WebSocket) {
    let mut rx = ctx.visitor_rx.clone();
    let mut interval = tokio::time::interval(Duration::from_secs(8));

    loop {
        select! {
            Ok(()) = rx.changed() => {
                let msg = rx.borrow().to_string();
                let res = socket.send(Message::Text(msg.clone())).await;
                if res.is_err() {
                    break;
                }
            }
            _ = interval.tick() => {
                let res = socket.send(Message::Ping(vec![])).await;
                if res.is_err() {
                    break;
                }
            }
        }
    }
}

pub async fn show_badge(
    Path(mut domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> Response {
    let mut v_type = crate::app_model::VisitorType::Badge;

    let domain_referrer = get_domain_from_referrer(&headers).unwrap_or("".to_string());
    if domain_referrer.ne(&domain) {
        if domain.eq("[domain]") {
            domain = domain_referrer;
        } else {
            v_type = crate::app_model::VisitorType::ICON;
        }
    }

    let tend = ctx.boring_visitor(v_type, &domain, &headers).await;
    if tend.is_err() {
        return (
            StatusCode::NOT_FOUND,
            Headers([("content-type", "text/plain")]),
            tend.err().unwrap().to_string(),
        )
            .into_response();
    }

    render_svg(tend.unwrap(), &ctx.badge).await
}

pub async fn show_favicon(
    Path(domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> Response {
    let tend = ctx
        .boring_visitor(crate::app_model::VisitorType::ICON, &domain, &headers)
        .await;
    if tend.is_err() {
        return (
            StatusCode::NOT_FOUND,
            Headers([("content-type", "text/plain")]),
            tend.err().unwrap().to_string(),
        )
            .into_response();
    }
    render_svg(tend.unwrap(), &ctx.favicon).await
}

pub async fn show_icon(
    Path(domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> Response {
    let tend = ctx
        .boring_visitor(crate::app_model::VisitorType::ICON, &domain, &headers)
        .await;
    if tend.is_err() {
        return (
            StatusCode::NOT_FOUND,
            Headers([("content-type", "text/plain")]),
            tend.err().unwrap().to_string(),
        )
            .into_response();
    }

    render_svg(tend.unwrap(), &ctx.icon).await
}

#[derive(Template)]
#[template(path = "index.html")]
struct HomeTemplate {
    version: String,
    membership: Vec<Membership>,
    uv: HashMap<i64, i64>,
    referrer: HashMap<i64, i64>,
    rank: Vec<RankAndMembership>,
    to_be_remove: Vec<RankAndMembership>,
    level: HashMap<i64, i64>,
}

pub async fn home_page(
    Extension(ctx): Extension<DynContext>,
    headers: HeaderMap,
) -> Result<Html<String>, String> {
    let domain = get_domain_from_referrer(&headers);
    if domain.is_ok() {
        let _ = ctx
            .boring_visitor(
                crate::app_model::VisitorType::Referrer,
                &domain.unwrap(),
                &headers,
            )
            .await;
    }
    let referrer_read = ctx.referrer.read().await;
    let uv_read = ctx.unique_visitor.read().await;

    let mut level: HashMap<i64, i64> = HashMap::new();
    let mut rank_vec: Vec<(i64, i64)> = Vec::new();

    for k in ctx.id2member.keys() {
        let rank_svg = ctx.rank_svg.read().await.to_owned();
        let uv = uv_read.get(k).unwrap_or(&0).to_owned();
        let rv = referrer_read.get(k).unwrap_or(&0).to_owned();
        if uv > 0 || rv > 0 {
            rank_vec.push((k.to_owned(), (rv + uv) / rank_svg));
            level.insert(k.to_owned(), ctx.get_tend_from_uv_and_rv(uv, rv).await);
        }
    }

    rank_vec.sort_by(|a, b| b.1.cmp(&a.1));

    let mut membership = Vec::new();
    for v in rank_vec {
        membership.push(ctx.id2member.get(&v.0).unwrap().to_owned());
    }

    let mut rank_and_membership_to_be_remove = Vec::new();
    let mut rank_and_membership = Vec::new();

    let monthly_rank = ctx.monthly_rank.read().await.to_owned();
    monthly_rank
        .iter()
        .filter(|r| ctx.id2member.contains_key(&r.membership_id))
        .for_each(|r| {
            if rank_and_membership.len() >= 10
                || r.updated_at < now_shanghai() - chrono::Duration::days(30)
            {
                return;
            }
            let m = ctx.id2member.get(&r.membership_id).unwrap().to_owned();
            rank_and_membership.push(RankAndMembership {
                rank: r.to_owned(),
                membership: m,
            });
        });

    let rank = ctx.rank.read().await.to_owned();
    rank.iter()
        .filter(|r| ctx.id2member.contains_key(&r.membership_id))
        .for_each(|r| {
            if r.updated_at < now_shanghai() - chrono::Duration::days(30) {
                let m = ctx.id2member.get(&r.membership_id).unwrap().to_owned();
                rank_and_membership_to_be_remove.push(RankAndMembership {
                    rank: r.to_owned(),
                    membership: m,
                });
            }
        });

    let tpl = HomeTemplate {
        membership,
        uv: uv_read.clone(),
        referrer: referrer_read.clone(),
        rank: rank_and_membership,
        to_be_remove: rank_and_membership_to_be_remove,
        level,
        version: GIT_HASH[0..8].to_string(),
    };
    let html = tpl.render().map_err(|err| err.to_string())?;
    Ok(Html(html))
}

#[derive(Template)]
#[template(path = "join_us.html")]
struct JoinUsTemplate {
    version: String,
}

pub async fn join_us_page() -> Result<Html<String>, String> {
    let tpl = JoinUsTemplate {
        version: GIT_HASH[0..8].to_string(),
    };
    let html = tpl.render().map_err(|err| err.to_string())?;
    Ok(Html(html))
}

#[derive(Template)]
#[template(path = "rank.html")]
struct RankTemplate {
    version: String,
    rank: Vec<RankAndMembership>,
    to_be_remove: Vec<RankAndMembership>,
}

pub async fn rank_page(
    Extension(ctx): Extension<DynContext>,
    headers: HeaderMap,
) -> Result<Html<String>, String> {
    let domain = get_domain_from_referrer(&headers);
    if domain.is_ok() {
        let _ = ctx
            .boring_visitor(
                crate::app_model::VisitorType::Referrer,
                &domain.unwrap(),
                &headers,
            )
            .await;
    }

    let rank = ctx.rank.read().await.to_owned();

    let mut rank_and_membership_to_be_remove = Vec::new();

    let mut rank_and_membership = Vec::new();

    rank.iter()
        .filter(|r| ctx.id2member.contains_key(&r.membership_id))
        .for_each(|r| {
            if r.updated_at > now_shanghai() - chrono::Duration::days(30) {
                let m = ctx.id2member.get(&r.membership_id).unwrap().to_owned();
                rank_and_membership.push(RankAndMembership {
                    rank: r.to_owned(),
                    membership: m,
                });
            } else {
                let m = ctx.id2member.get(&r.membership_id).unwrap().to_owned();
                rank_and_membership_to_be_remove.push(RankAndMembership {
                    rank: r.to_owned(),
                    membership: m,
                });
            }
        });

    let tpl = RankTemplate {
        rank: rank_and_membership,
        to_be_remove: rank_and_membership_to_be_remove,
        version: GIT_HASH[0..8].to_string(),
    };
    let html = tpl.render().map_err(|err| err.to_string())?;
    Ok(Html(html))
}

fn get_domain_from_referrer(headers: &HeaderMap) -> Result<String, anyhow::Error> {
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

async fn render_svg(tend: (&str, i64, i64, i64), render: &BoringFace) -> Response {
    let headers = Headers([("content-type", "image/svg+xml")]);
    (
        StatusCode::OK,
        headers,
        render.render_svg(tend.0, tend.1, tend.2, tend.3),
    )
        .into_response()
}
