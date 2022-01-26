use axum::{routing::get, AddExtensionLayer, Router};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenv::dotenv;
use naive::{
    app_model::{Context, DynContext},
    app_router::{home_page, show_badge, show_favicon, show_icon},
    establish_connection, DbPool,
};
use std::{env, net::SocketAddr, sync::Arc};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/");

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_pool: DbPool = establish_connection(&env::var("DATABASE_URL").unwrap());

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "migration" {
        tracing::info!(
            "migration {:?}",
            db_pool.get().unwrap().run_pending_migrations(MIGRATIONS)
        );
        return;
    }

    let context = Arc::new(Context::default(db_pool).await) as DynContext;

    // 定时存入数据库
    let ctx_clone = context.clone();
    tokio::spawn(async move {
        ctx_clone.save_per_5_minutes().await;
    });

    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/badge/:domain", get(show_badge))
                .route("/favicon/:domain", get(show_favicon))
                .route("/icon/:domain", get(show_icon)),
        )
        .route("/", get(home_page))
        .layer(AddExtensionLayer::new(context));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
