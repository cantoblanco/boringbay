use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::anyhow;
use askama::Template;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Extension, Path, Query, WebSocketUpgrade,
    },
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;
use tokio::select;

use crate::{
    analytics::{AnalyticsChannel, AnalyticsEvent, AnalyticsEventKind, AnalyticsService},
    app_model::{Context, DynContext},
    boring_face::BoringFace,
    config::AppConfig,
    discovery::{DailyRoute, DiscoveryService},
    feed::{FeedItem, FeedRepository},
    membership_model::Membership,
    now_shanghai,
    product_events::{EventInput, ProductEventKind, ProductEventService},
    ranking::{RankingEntry, RankingService},
    share::{render_member_badge_svg, render_route_svg},
    site_health::{
        status_from_evidence, status_reason, HealthEvidence, MemberStatus, SiteHealthService,
    },
    GIT_HASH,
};

pub async fn record_event(
    Extension(ctx): Extension<DynContext>,
    Extension(config): Extension<Arc<AppConfig>>,
    headers: HeaderMap,
    Json(input): Json<EventInput>,
) -> StatusCode {
    if !config.v2_enabled {
        return StatusCode::NOT_FOUND;
    }
    let kind = match ProductEventKind::try_from(input.kind.as_str()) {
        Ok(kind) => kind,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    if input
        .member_id
        .map(|id| id <= 0 || !ctx.id2member.contains_key(&id))
        .unwrap_or(false)
    {
        return StatusCode::BAD_REQUEST;
    }
    let channel = match input.channel.as_deref() {
        Some(value) => match AnalyticsChannel::try_from(value) {
            Ok(channel) => channel,
            Err(_) => return StatusCode::BAD_REQUEST,
        },
        None if kind == ProductEventKind::FeedOutbound => AnalyticsChannel::Feed,
        None => AnalyticsChannel::Unknown,
    };
    match ProductEventService::new(ctx.db_pool.clone()).increment(
        now_shanghai().date(),
        kind,
        input.member_id,
    ) {
        Ok(()) => {
            if matches!(kind, ProductEventKind::MemberOutbound | ProductEventKind::FeedOutbound) {
                if let Some(member_id) = input.member_id {
                    let identity = crate::visitor::VisitorIdentity::from_headers(
                        &headers,
                        ctx.trusted_proxy_mode,
                        &ctx.visitor_hasher,
                    );
                    let event = AnalyticsEvent {
                        at: now_shanghai(),
                        member_id,
                        kind: AnalyticsEventKind::OutboundClick,
                        country: identity.map(|value| value.country),
                        referrer_domain: None,
                        channel: Some(channel),
                    };
                    if let Err(error) = AnalyticsService::new(ctx.db_pool.clone()).record(event) {
                        tracing::warn!(member_id, event_kind = "outbound_click", %error, "traffic aggregate write failed");
                    }
                }
            }
            StatusCode::NO_CONTENT
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

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
                let res = socket.send(Message::Text(msg.clone().into())).await;
                if res.is_err() {
                    break;
                }
            }
            _ = interval.tick() => {
                let res = socket.send(Message::Ping(Vec::new().into())).await;
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
    let mut v_type = Some(crate::app_model::VisitorType::Badge);

    let domain_referrer = get_domain_from_referrer(&headers).unwrap_or("".to_string());
    if domain_referrer.ne(&domain) {
        if domain.eq("[domain]") {
            domain = domain_referrer;
        } else {
            v_type = None;
        }
    }

    let tend = ctx.boring_visitor(v_type, &domain, &headers).await;
    if tend.is_err() {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain")],
            tend.err().unwrap().to_string(),
        )
            .into_response();
    }

    render_svg(tend.unwrap(), &ctx.badge).await
}

pub async fn show_badge_v2(
    Path(mut domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
    Extension(config): Extension<Arc<AppConfig>>,
) -> Response {
    if !config.v2_enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let mut visitor_type = Some(crate::app_model::VisitorType::Badge);
    let referrer = get_domain_from_referrer(&headers).unwrap_or_default();
    if referrer != domain {
        if domain == "[domain]" {
            domain = referrer;
        } else {
            visitor_type = None;
        }
    }
    match ctx.boring_visitor(visitor_type, &domain, &headers).await {
        Ok((name, uv, rv, level)) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
                (header::CACHE_CONTROL, "public, max-age=300"),
            ],
            render_member_badge_svg(name, uv, rv, level),
        )
            .into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain")],
            error.to_string(),
        )
            .into_response(),
    }
}

pub async fn show_favicon(
    Path(domain): Path<String>,
    headers: HeaderMap,
    Extension(ctx): Extension<DynContext>,
) -> Response {
    let tend = ctx
        .boring_visitor(Some(crate::app_model::VisitorType::ICON), &domain, &headers)
        .await;
    if tend.is_err() {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain")],
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
        .boring_visitor(Some(crate::app_model::VisitorType::ICON), &domain, &headers)
        .await;
    if tend.is_err() {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain")],
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
    rank: Vec<RankedMember>,
    status_attention: Vec<StatusMember>,
    level: HashMap<i64, i64>,
    v2_enabled: bool,
    today_route: String,
    feeds: Vec<FeedView>,
}

#[derive(Clone)]
struct FeedView {
    item: FeedItem,
    membership: Membership,
    published: String,
}

pub async fn home_page(
    Extension(ctx): Extension<DynContext>,
    Extension(config): Extension<Arc<AppConfig>>,
    headers: HeaderMap,
) -> Result<Html<String>, String> {
    if let Ok(domain) = get_domain_from_referrer(&headers) {
        let _ = ctx
            .boring_visitor(
                Some(crate::app_model::VisitorType::Referer),
                &domain,
                &headers,
            )
            .await;
    }
    let referrer_read = ctx.referrer.read().await;
    let uv_read = ctx.unique_visitor.read().await;

    let mut level: HashMap<i64, i64> = HashMap::new();
    let mut rank_vec: Vec<(i64, NaiveDateTime, i64)> = Vec::new();

    for k in ctx.id2member.keys() {
        let uv = uv_read
            .get(k)
            .unwrap_or(&(
                0,
                chrono::DateTime::from_timestamp(0, 0)
                    .expect("unix epoch")
                    .naive_utc(),
            ))
            .to_owned();
        let rv = referrer_read
            .get(k)
            .unwrap_or(&(
                0,
                chrono::DateTime::from_timestamp(0, 0)
                    .expect("unix epoch")
                    .naive_utc(),
            ))
            .to_owned();
        if uv.0 > 0 || rv.0 > 0 {
            rank_vec.push((k.to_owned(), rv.1, uv.0));
            level.insert(k.to_owned(), ctx.get_tend_from_uv_and_rv(uv.0, rv.0).await);
        }
    }

    rank_vec.sort_by(|a, b| match b.1.cmp(&a.1) {
        std::cmp::Ordering::Equal => b.2.cmp(&a.2),
        _ => b.1.cmp(&a.1),
    });

    let mut membership = Vec::new();
    for v in rank_vec {
        if let Some(member) = ctx.id2member.get(&v.0) {
            membership.push(member.to_owned());
        }
    }

    let ranking_service = RankingService::new(ctx.db_pool.clone());
    let rank_and_membership = ranked_members(
        ranking_service
            .activity_30d(now_shanghai())
            .unwrap_or_default(),
        &ctx,
    )
    .into_iter()
    .take(10)
    .collect();
    let status_attention = status_members(&ctx)
        .into_iter()
        .filter(|entry| {
            matches!(
                entry.status,
                MemberStatus::Observation | MemberStatus::RemovalCandidate
            )
        })
        .take(10)
        .collect();

    let feeds = if config.v2_enabled {
        FeedRepository::new(ctx.db_pool.clone())
            .latest(12)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| {
                ctx.id2member
                    .get(&item.member_id)
                    .cloned()
                    .map(|membership| {
                        let published = item.published_at.format("%Y-%m-%d").to_string();
                        FeedView {
                            item,
                            membership,
                            published,
                        }
                    })
            })
            .collect()
    } else {
        Vec::new()
    };

    let tpl = HomeTemplate {
        membership,
        uv: uv_read
            .iter()
            .map(|(k, v)| (k.to_owned(), v.0))
            .collect::<HashMap<i64, i64>>(),
        referrer: referrer_read
            .iter()
            .map(|(k, v)| (k.to_owned(), v.0))
            .collect::<HashMap<i64, i64>>(),
        rank: rank_and_membership,
        status_attention,
        level,
        version: GIT_HASH[0..8].to_string(),
        v2_enabled: config.v2_enabled,
        today_route: format!("/route/{}", now_shanghai().date().format("%Y-%m-%d")),
        feeds,
    };
    let html = tpl.render().map_err(|err| err.to_string())?;
    Ok(Html(html))
}

#[derive(Template)]
#[template(path = "route.html")]
struct RouteTemplate {
    version: String,
    date: String,
    members: Vec<Membership>,
    canonical_url: String,
    share_image_url: String,
}

#[derive(serde::Serialize)]
pub struct DailyRouteResponse {
    date: NaiveDate,
    members: Vec<Membership>,
}

pub async fn route_page(
    Path(date): Path<String>,
    Extension(ctx): Extension<DynContext>,
    Extension(config): Extension<Arc<AppConfig>>,
) -> Result<Html<String>, StatusCode> {
    if !config.v2_enabled {
        return Err(StatusCode::NOT_FOUND);
    }
    let date = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|_| StatusCode::NOT_FOUND)?;
    let response = daily_route_response(&ctx, date).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let template = RouteTemplate {
        version: GIT_HASH[0..8].to_string(),
        date: date.format("%Y-%m-%d").to_string(),
        members: response.members,
        canonical_url: format!("https://{}/route/{}", config.system_domain, date),
        share_image_url: format!(
            "https://{}/api/share/route/{}.svg",
            config.system_domain, date
        ),
    };
    template
        .render()
        .map(Html)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn route_share_image(
    Path(date): Path<String>,
    Extension(ctx): Extension<DynContext>,
    Extension(config): Extension<Arc<AppConfig>>,
) -> Response {
    if !config.v2_enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    let date = match NaiveDate::parse_from_str(date.trim_end_matches(".svg"), "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    match daily_route_response(&ctx, date) {
        Ok(route) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
                (header::CACHE_CONTROL, "public, max-age=86400, immutable"),
            ],
            render_route_svg(date, &route.members),
        )
            .into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub async fn discovery_today(
    Extension(ctx): Extension<DynContext>,
    Extension(config): Extension<Arc<AppConfig>>,
) -> Result<Json<DailyRouteResponse>, StatusCode> {
    if !config.v2_enabled {
        return Err(StatusCode::NOT_FOUND);
    }
    daily_route_response(&ctx, now_shanghai().date())
        .map(Json)
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

fn daily_route_response(ctx: &Context, date: NaiveDate) -> anyhow::Result<DailyRouteResponse> {
    let eligible = status_members(ctx)
        .into_iter()
        .filter(|member| matches!(member.status, MemberStatus::Active | MemberStatus::Quiet))
        .map(|entry| entry.membership)
        .collect::<Vec<_>>();
    let DailyRoute { member_ids, .. } =
        DiscoveryService::new(ctx.db_pool.clone()).daily_route(date, &eligible)?;
    let by_id = eligible
        .into_iter()
        .map(|member| (member.id, member))
        .collect::<HashMap<_, _>>();
    let members = member_ids
        .into_iter()
        .filter_map(|id| by_id.get(&id).cloned())
        .collect::<Vec<_>>();
    if members.len() != 5 {
        return Err(anyhow!("daily route members are no longer eligible"));
    }
    Ok(DailyRouteResponse { date, members })
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
    view: String,
    title: String,
    formula: String,
    window: String,
    rank: Vec<RankedMember>,
    statuses: Vec<StatusMember>,
}

#[derive(Clone)]
struct RankedMember {
    membership: Membership,
    entry: RankingEntry,
    growth_percent: String,
}

#[derive(Clone)]
struct StatusMember {
    membership: Membership,
    status: MemberStatus,
    status_label: String,
    reason: String,
    last_activity: String,
    last_check: String,
    failures: u32,
}

#[derive(Default, Deserialize)]
pub struct RankQuery {
    view: Option<String>,
}

pub async fn rank_page(
    Extension(ctx): Extension<DynContext>,
    headers: HeaderMap,
    Query(query): Query<RankQuery>,
) -> Result<Html<String>, String> {
    if let Ok(domain) = get_domain_from_referrer(&headers) {
        let _ = ctx
            .boring_visitor(
                Some(crate::app_model::VisitorType::Referer),
                &domain,
                &headers,
            )
            .await;
    }

    let now = now_shanghai();
    let service = RankingService::new(ctx.db_pool.clone());
    let view = query.view.as_deref().unwrap_or("classic");
    let (view, title, formula, window, entries) = match view {
        "activity" => (
            "activity",
            "30 天活跃榜",
            "得分 = 近 30 天 UV + RV；同分依次比较 RV、UV、最后活动时间。",
            format!(
                "{} 至 {}",
                (now - chrono::Duration::days(30)).format("%Y-%m-%d"),
                now.format("%Y-%m-%d")
            ),
            service.activity_30d(now),
        ),
        "rising" => (
            "rising",
            "7 天上升榜",
            "增长率 =（本 7 天互动 - 前 7 天互动）/ max（前 7 天互动, 5）；本期至少 5 次互动。",
            format!(
                "{} 至 {}，对比此前 7 天",
                (now - chrono::Duration::days(7)).format("%Y-%m-%d"),
                now.format("%Y-%m-%d")
            ),
            service.rising_7d(now),
        ),
        _ => (
            "classic",
            "经典总榜",
            "保留原有规则：优先按累计 RV，其次累计 UV 排序。",
            "自建站以来的历史数据".to_string(),
            service.classic(now),
        ),
    };
    let rank_and_membership = ranked_members(entries.map_err(|err| err.to_string())?, &ctx);

    let tpl = RankTemplate {
        view: view.to_string(),
        title: title.to_string(),
        formula: formula.to_string(),
        window,
        rank: rank_and_membership,
        statuses: status_members(&ctx),
        version: GIT_HASH[0..8].to_string(),
    };
    let html = tpl.render().map_err(|err| err.to_string())?;
    Ok(Html(html))
}

fn ranked_members(entries: Vec<RankingEntry>, ctx: &Context) -> Vec<RankedMember> {
    entries
        .into_iter()
        .filter_map(|entry| {
            let membership = ctx.id2member.get(&entry.membership_id)?.clone();
            let growth_percent = entry
                .growth_rate
                .map(|rate| format!("{:+.0}%", rate * 100.0))
                .unwrap_or_else(|| "—".to_string());
            Some(RankedMember {
                membership,
                entry,
                growth_percent,
            })
        })
        .collect()
}

fn status_members(ctx: &Context) -> Vec<StatusMember> {
    let now = now_shanghai();
    let classic = RankingService::new(ctx.db_pool.clone())
        .classic(now)
        .unwrap_or_default();
    let activity = classic
        .into_iter()
        .map(|entry| (entry.membership_id, entry.last_activity))
        .collect::<HashMap<_, _>>();
    let evidence = SiteHealthService::new(ctx.db_pool.clone())
        .and_then(|service| service.evidence_by_member())
        .unwrap_or_default();
    let epoch = chrono::DateTime::from_timestamp(0, 0)
        .expect("unix epoch must exist")
        .naive_utc();

    let mut rows = ctx
        .id2member
        .values()
        .cloned()
        .map(|membership| {
            let last_activity = activity.get(&membership.id).copied().unwrap_or(epoch);
            let member_evidence = evidence
                .get(&membership.id)
                .cloned()
                .unwrap_or_else(HealthEvidence::default);
            let status = status_from_evidence(now, last_activity, &member_evidence);
            StatusMember {
                membership,
                status,
                status_label: status.label().to_string(),
                reason: status_reason(status, member_evidence.consecutive_failures),
                last_activity: if last_activity == epoch {
                    "暂无记录".to_string()
                } else {
                    last_activity.format("%Y-%m-%d %H:%M").to_string()
                },
                last_check: member_evidence
                    .last_checked_at
                    .map(|value| value.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "尚未检测".to_string()),
                failures: member_evidence.consecutive_failures,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        status_priority(b.status)
            .cmp(&status_priority(a.status))
            .then_with(|| a.membership.id.cmp(&b.membership.id))
    });
    rows
}

fn status_priority(status: MemberStatus) -> u8 {
    match status {
        MemberStatus::Active => 0,
        MemberStatus::Quiet => 1,
        MemberStatus::Observation => 2,
        MemberStatus::RemovalCandidate => 3,
    }
}

fn get_domain_from_referrer(headers: &HeaderMap) -> Result<String, anyhow::Error> {
    let referrer = headers
        .get("Referer")
        .ok_or_else(|| anyhow!("no referrer header"))?
        .to_str()
        .map_err(|_| anyhow!("referrer header is not valid utf-8 string"))?;
    let referrer_url =
        url::Url::parse(referrer).map_err(|_| anyhow!("referrer header is not valid URL"))?;
    referrer_url
        .domain()
        .map(str::to_string)
        .ok_or_else(|| anyhow!("referrer header doesn't contain a valid domain"))
}

async fn render_svg(tend: (&str, i64, i64, i64), render: &BoringFace) -> Response {
    let headers = [(header::CONTENT_TYPE, "image/svg+xml")];
    (
        StatusCode::OK,
        headers,
        render.render_svg(tend.0, tend.1, tend.2, tend.3),
    )
        .into_response()
}
