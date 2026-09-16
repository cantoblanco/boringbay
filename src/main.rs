use chrono::{NaiveDateTime, NaiveTime};
use dotenvy::dotenv;
use naive::{
    app_model::{Context, DynContext},
    build_router,
    config::AppConfig,
    establish_connection,
    feed::FeedFetcher,
    now_shanghai, run_migrations,
    site_health::SiteHealthService,
    statistics_model::Statistics,
    DbPool,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::signal;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Arc::new(AppConfig::from_env().expect("invalid application configuration"));
    let db_pool: DbPool = establish_connection(&config.database_url);

    run_migrations(&mut db_pool.get().expect("database pool unavailable"))
        .expect("database migration failed");

    let context = Arc::new(Context::new(db_pool, &config).await) as DynContext;

    if config.v2_enabled {
        let members = context.id2member.values().cloned().collect::<Vec<_>>();
        let health_service = SiteHealthService::new(context.db_pool.clone())
            .expect("site health client configuration failed");
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            loop {
                health_service.run_once(members.clone().into_iter()).await;
                tokio::time::sleep(std::time::Duration::from_secs(60 * 60 * 24)).await;
            }
        });

        let feed_members = context
            .id2member
            .values()
            .filter(|member| member.feed_url.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let feed_fetcher = FeedFetcher::new(context.db_pool.clone());
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(90)).await;
            loop {
                for member in &feed_members {
                    if let Err(error) = feed_fetcher.refresh_member(member).await {
                        tracing::warn!(member_id = member.id, %error, "feed refresh failed");
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(60 * 30)).await;
            }
        });
    }

    // 定时存入数据库
    let ctx_clone = context.clone();
    tokio::spawn(async move {
        ctx_clone.save_per_5_minutes().await;
    });

    let ctx_clone_for_shutdown = context.clone();

    let app = build_router(context, config);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind HTTP listener");
    axum::serve(listener, app)
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

    let _today = NaiveDateTime::new(
        now_shanghai().date(),
        NaiveTime::from_hms_opt(0, 0, 0).expect("midnight"),
    );
    let page_view_read = ctx.unique_visitor.read().await;
    let referrer_read = ctx.referrer.read().await;
    ctx.id2member.keys().for_each(|id| {
        let uv = *page_view_read.get(id).unwrap_or(&(
            0,
            chrono::DateTime::from_timestamp(0, 0)
                .expect("unix epoch")
                .naive_utc(),
        ));
        let referrer = *referrer_read.get(id).unwrap_or(&(
            0,
            chrono::DateTime::from_timestamp(0, 0)
                .expect("unix epoch")
                .naive_utc(),
        ));
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
