mod app_error;
mod app_model;
mod app_router;
mod boring_face;

use app_model::DbPool;
use axum::{routing::get, AddExtensionLayer, Router};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use dotenv::dotenv;
use std::{collections::HashMap, env, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    app_model::{Context, DynContext},
    app_router::{badge_reverse_show, badge_show},
    boring_face::BoringFace,
};

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_pool: DbPool = establish_connection(&env::var("DATABASE_URL").unwrap());

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "migration" {
        tracing::info!(
            "migration {:?}",
            diesel_migrations::run_pending_migrations(&db_pool.get().unwrap())
        );
        return;
    }

    let context = Arc::new(Context {
        badge: BoringFace::new("#03081A".to_string(), "white".to_string()),
        badge_reverse: BoringFace::new("white".to_string(), "#03081A".to_string()),
        render_cache: RwLock::new(HashMap::new()),
    }) as DynContext;

    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/badge/:domain", get(badge_show))
                .route("/badge_reverse/:domain", get(badge_reverse_show)),
        )
        .layer(AddExtensionLayer::new(context));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub fn establish_connection(database_url: &str) -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
