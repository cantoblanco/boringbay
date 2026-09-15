use std::sync::Arc;

use anyhow::anyhow;
use axum::{
    http::StatusCode,
    routing::{get, get_service, post},
    AddExtensionLayer, Router,
};
use chrono::{NaiveDateTime, Utc};
use chrono_tz::Asia::Shanghai;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tower_http::services::ServeDir;

pub mod app_model;
pub mod app_router;
pub mod boring_face;
pub mod config;
pub mod discovery;
pub mod membership_model;
pub mod network_policy;
pub mod product_events;
pub mod ranking;
pub mod schema;
pub mod site_health;
pub mod statistics_model;
pub mod visitor;

extern crate diesel;

pub const GIT_HASH: &str = env!("GIT_HASH");
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/");

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

pub fn establish_connection(database_url: &str) -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn run_migrations(conn: &mut SqliteConnection) -> anyhow::Result<()> {
    conn.run_pending_migrations(MIGRATIONS)
        .map(|_| ())
        .map_err(|err| anyhow!(err.to_string()))
}

pub fn build_router(ctx: app_model::DynContext, config: Arc<config::AppConfig>) -> Router {
    use app_router::{
        discovery_today, home_page, join_us_page, rank_page, record_event, route_page, show_badge,
        show_favicon, show_icon, ws_upgrade,
    };

    Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/badge/:domain", get(show_badge))
                .route("/favicon/:domain", get(show_favicon))
                .route("/icon/:domain", get(show_icon))
                .route("/ws", get(ws_upgrade))
                .route("/events", post(record_event)),
        )
        .route("/", get(home_page))
        .route("/join-us", get(join_us_page))
        .route("/rank", get(rank_page))
        .route("/route/:date", get(route_page))
        .route("/api/discovery/today", get(discovery_today))
        .nest(
            "/static",
            get_service(ServeDir::new("resources/static")).handle_error(|error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("static file error: {error}"),
                )
            }),
        )
        .layer(AddExtensionLayer::new(ctx))
        .layer(AddExtensionLayer::new(config))
}

pub fn now_shanghai() -> NaiveDateTime {
    Utc::now().with_timezone(&Shanghai).naive_local()
}
