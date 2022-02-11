use chrono::{NaiveDateTime, Utc};
use chrono_tz::Asia::Shanghai;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};

pub mod app_model;
pub mod app_router;
pub mod boring_face;
pub mod membership_model;
pub mod schema;
pub mod statistics_model;

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

pub fn now_shanghai() -> NaiveDateTime {
    Utc::now().with_timezone(&Shanghai).naive_local()
}
