use std::sync::Arc;

use axum::Router;
use naive::{
    app_model::{Context, DynContext},
    build_router,
    config::AppConfig,
    establish_connection, run_migrations,
};
use tempfile::TempDir;

pub async fn temporary_app() -> (TempDir, Router) {
    temporary_app_with_v2(true).await
}

pub async fn temporary_app_with_v2(v2_enabled: bool) -> (TempDir, Router) {
    let temp = tempfile::tempdir().expect("temporary directory");
    let database_url = temp.path().join("test.db").display().to_string();
    let mut config = AppConfig::for_test(database_url.clone());
    config.v2_enabled = v2_enabled;
    let config = Arc::new(config);
    let pool = establish_connection(&database_url);
    run_migrations(&mut pool.get().expect("database connection")).expect("test migrations");
    let ctx = Arc::new(Context::new(pool, &config).await) as DynContext;
    (temp, build_router(ctx, config))
}
