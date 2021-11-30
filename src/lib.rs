use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};

pub mod app_error;
pub mod app_model;
pub mod app_router;
pub mod boring_face;
pub mod membership_model;
pub mod schema;

#[macro_use]
extern crate diesel;

pub const GIT_HASH: &'static str = env!("GIT_HASH");

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;

pub fn establish_connection(database_url: &str) -> DbPool {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
