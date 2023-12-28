use std::env;

use chrono::{NaiveDateTime, Utc};
use chrono_tz::Asia::Shanghai;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use lazy_static::lazy_static;

pub mod app_model;
pub mod app_router;
pub mod boring_face;
pub mod membership_model;
pub mod schema;
pub mod statistics_model;

extern crate diesel;

pub const GIT_HASH: &str = env!("GIT_HASH");

// 系统域名，忽略 referrer 计数
lazy_static! {
    static ref SYSTEM_DOMAIN: String = env::var("SYSTEM_DOMAIN").unwrap();
}

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
