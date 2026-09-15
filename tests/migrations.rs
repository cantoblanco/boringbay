mod common;

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Text};
use naive::schema::{daily_routes, feed_items, feed_sources, product_events, site_health};

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct JournalMode {
    #[diesel(sql_type = Text)]
    journal_mode: String,
}

#[derive(QueryableByName)]
struct BusyTimeout {
    #[diesel(sql_type = Integer)]
    timeout: i32,
}

#[tokio::test]
async fn additive_migrations_create_site_health_without_touching_statistics() {
    let (_tmp, app) = common::temporary_app().await;
    drop(app);

    let database_url = _tmp.path().join("test.db");
    let pool = naive::establish_connection(database_url.to_str().unwrap());
    let mut connection = pool.get().unwrap();
    let journal = diesel::sql_query("PRAGMA journal_mode")
        .get_result::<JournalMode>(&mut connection)
        .unwrap();
    assert_eq!(journal.journal_mode.to_ascii_lowercase(), "wal");
    let busy_timeout = diesel::sql_query("PRAGMA busy_timeout")
        .get_result::<BusyTimeout>(&mut connection)
        .unwrap();
    assert_eq!(busy_timeout.timeout, 5_000);
    drop(connection);
    let held_connections = (0..5)
        .map(|_| {
            pool.get()
                .expect("every pooled SQLite connection initializes")
        })
        .collect::<Vec<_>>();
    assert_eq!(held_connections.len(), 5);
    drop(held_connections);
    let mut connection = pool.get().unwrap();
    let health_count = site_health::table
        .count()
        .get_result::<i64>(&mut connection);
    assert_eq!(health_count.unwrap(), 0);
    assert_eq!(
        daily_routes::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        0
    );
    assert_eq!(
        product_events::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        0
    );
    assert_eq!(
        feed_sources::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        0
    );
    assert_eq!(
        feed_items::table
            .count()
            .get_result::<i64>(&mut connection)
            .unwrap(),
        0
    );

    let statistics_count = diesel::sql_query("SELECT COUNT(*) AS count FROM statistics")
        .get_result::<Count>(&mut connection)
        .unwrap();
    assert_eq!(statistics_count.count, 0);
}
