use axum::{routing::get, AddExtensionLayer, Router};
use chrono::{NaiveDateTime, NaiveTime};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenv::dotenv;
use naive::{
    app_model::{Context, DynContext},
    app_router::{
        home_page, join_us_page, rank_page, show_badge, show_favicon, show_icon, ws_upgrade,
    },
    establish_connection, now_shanghai,
    statistics_model::Statistics,
    DbPool,
};
use std::{env, net::SocketAddr, sync::Arc};
use tokio::signal;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/");

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_pool: DbPool = establish_connection(&env::var("DATABASE_URL").unwrap());

    tracing::info!(
        "migration {:?}",
        db_pool
            .get()
            .unwrap()
            .run_pending_migrations(MIGRATIONS)
            .unwrap()
    );

    let context = Arc::new(Context::default(db_pool).await) as DynContext;

    // 定时存入数据库
    let ctx_clone = context.clone();
    tokio::spawn(async move {
        ctx_clone.save_per_5_minutes().await;
    });

    let ctx_clone_for_shutdown = context.clone();

    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/badge/:domain", get(show_badge))
                .route("/favicon/:domain", get(show_favicon))
                .route("/icon/:domain", get(show_icon))
                .route("/ws", get(ws_upgrade)),
        )
        .route("/", get(home_page))
        .route("/join-us", get(join_us_page))
        .route("/rank", get(rank_page))
        .layer(AddExtensionLayer::new(context));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal(ctx_clone_for_shutdown))
        .await
        .unwrap();
}

async fn shutdown_signal(ctx: Arc<Context>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, running cleanup tasks..");

    let _today = NaiveDateTime::new(now_shanghai().date(), NaiveTime::from_hms(0, 0, 0));
    let page_view_read = ctx.unique_visitor.read().await;
    let referrer_read = ctx.referrer.read().await;
    ctx.id2member.keys().for_each(|id| {
        let uv = *page_view_read.get(id).unwrap_or(&(0, now_shanghai()));
        let referrer = *referrer_read.get(id).unwrap_or(&(0, now_shanghai()));
        Statistics::insert_or_update(
            ctx.db_pool.get().unwrap(),
            &Statistics {
                created_at: _today,
                membership_id: *id,
                unique_visitor: uv.0,
                updated_at: uv.1,
                referrer: referrer.0,
                latest_referrer_at: referrer.1,
                id: 0,
            },
        )
        .unwrap();
    })
}
