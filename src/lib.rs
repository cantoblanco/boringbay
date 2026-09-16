use std::sync::Arc;

use anyhow::anyhow;
use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use chrono::{NaiveDateTime, Utc};
use chrono_tz::Asia::Shanghai;
use diesel::{
    connection::SimpleConnection,
    r2d2::{ConnectionManager, CustomizeConnection, Error as PoolConnectionError, Pool},
    Connection, SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tower_http::services::ServeDir;

pub mod app_model;
pub mod app_router;
pub mod analytics;
pub mod boring_face;
pub mod config;
pub mod discovery;
pub mod feed;
pub mod membership_model;
pub mod network_policy;
pub mod product_events;
pub mod ranking;
pub mod schema;
pub mod share;
pub mod site_health;
pub mod statistics_model;
pub mod visitor;

extern crate diesel;

pub const GIT_HASH: &str = env!("GIT_HASH");
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/");

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

#[derive(Debug)]
struct SqliteConnectionCustomizer;

impl CustomizeConnection<SqliteConnection, PoolConnectionError> for SqliteConnectionCustomizer {
    fn on_acquire(&self, connection: &mut SqliteConnection) -> Result<(), PoolConnectionError> {
        connection
            .batch_execute(
                "PRAGMA foreign_keys = ON; \
                 PRAGMA busy_timeout = 5000;",
            )
            .map_err(PoolConnectionError::QueryError)
    }
}

pub fn establish_connection(database_url: &str) -> DbPool {
    let mut bootstrap = SqliteConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
    bootstrap
        .batch_execute("PRAGMA journal_mode = WAL;")
        .unwrap_or_else(|_| panic!("Error enabling SQLite WAL for {}", database_url));
    drop(bootstrap);

    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    Pool::builder()
        .max_size(5)
        .min_idle(Some(1))
        .connection_customizer(Box::new(SqliteConnectionCustomizer))
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
        discovery_today, home_page, join_us_page, rank_page, record_event, route_page,
        route_share_image, show_badge, show_badge_v2, show_favicon, show_icon, ws_upgrade,
    };

    Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/badge/{domain}", get(show_badge))
                .route("/badge-v2/{domain}", get(show_badge_v2))
                .route("/favicon/{domain}", get(show_favicon))
                .route("/icon/{domain}", get(show_icon))
                .route("/ws", get(ws_upgrade))
                .route("/events", post(record_event)),
        )
        .route("/", get(home_page))
        .route("/join-us", get(join_us_page))
        .route("/rank", get(rank_page))
        .route("/route/{date}", get(route_page))
        .route("/api/discovery/today", get(discovery_today))
        .route("/api/share/route/{date}", get(route_share_image))
        .nest_service("/static", ServeDir::new("resources/static"))
        .layer(Extension(ctx))
        .layer(Extension(config))
}

pub fn now_shanghai() -> NaiveDateTime {
    Utc::now().with_timezone(&Shanghai).naive_local()
}
