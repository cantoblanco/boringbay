mod app_error;
mod boring_face;
mod app_model;
mod app_router;

use axum::{
    routing::{get, post},
    AddExtensionLayer, Router,
};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use app_model::DbPool;
use std::{env, net::SocketAddr, sync::Arc};

use crate::{
    app_model::{DynUserRepo, ExampleUserRepo},
    app_router::{users_create, users_show},
};

#[tokio::main]
async fn main() {
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

    let user_repo = Arc::new(ExampleUserRepo) as DynUserRepo;

    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/users/:id", get(users_show))
                .route("/users", post(users_create)),
        )
        // Add our `user_repo` to all request's extensions so handlers can access
        // it.
        .layer(AddExtensionLayer::new(user_repo));

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
