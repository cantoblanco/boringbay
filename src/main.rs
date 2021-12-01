use axum::{routing::get, AddExtensionLayer, Router};
use dotenv::dotenv;
use naive::{
    app_model::{Context, DynContext},
    app_router::{badge_reverse_show, badge_show},
    boring_face::BoringFace,
    establish_connection,
    membership_model::all_memberships,
    DbPool,
};
use std::{collections::HashMap, env, net::SocketAddr, sync::Arc, time};
use tokio::sync::RwLock;
use tracing::log::info;

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

    let all_members = all_memberships(&db_pool.get().unwrap()).unwrap();
    let geoip_db = ipdb::Reader::open_file("data/ipipfree.ipdb").unwrap();

    info!("{:?}", geoip_db.find("6.6.6.6", "CN"));

    let context = Arc::new(Context {
        badge: BoringFace::new("#d0273e".to_string(), "#f5acb9".to_string(), true),
        badge_reverse: BoringFace::new("#f5acb9".to_string(), "#d0273e".to_string(), false),
        render_cache: RwLock::new(HashMap::new()),
        render_reverse_cache: RwLock::new(HashMap::new()),
        db_pool,
        members: RwLock::new(all_members),
        page_view: RwLock::new(HashMap::new()),
        referrer: RwLock::new(HashMap::new()),
        geoip_db,
        last_sorted_at: RwLock::new(time::SystemTime::now()),

        domain2id: RwLock::new(HashMap::new()),
        id2index: RwLock::new(HashMap::new()),
    }) as DynContext;

    let app = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/badge/:domain", get(badge_show))
                .route("/favicon/:domain", get(badge_reverse_show)),
        )
        .layer(AddExtensionLayer::new(context));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
