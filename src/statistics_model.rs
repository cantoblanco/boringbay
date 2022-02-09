use std::ops::Sub;

use crate::schema::statistics::{self, dsl::*};
use anyhow::anyhow;
use chrono::{Duration, NaiveDateTime, NaiveTime, Utc};
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::sqlite::Sqlite;
use diesel::{debug_query, prelude::*};
use diesel::{Queryable, SqliteConnection};

#[derive(Queryable, Debug, Clone, Insertable)]
#[diesel(table_name = statistics)]
pub struct Statistics {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub membership_id: i64,
    pub page_view: i64,
    pub referrer: i64,
}

impl Statistics {
    pub fn insert_or_update(
        mut conn: PooledConnection<ConnectionManager<SqliteConnection>>,
        stat: &Statistics,
    ) -> Result<usize, diesel::result::Error> {
        let statement = diesel::insert_into(statistics)
            .values((
                created_at.eq(stat.created_at),
                updated_at.eq(stat.updated_at),
                membership_id.eq(stat.membership_id),
                page_view.eq(stat.page_view),
                referrer.eq(stat.referrer),
            ))
            .on_conflict((membership_id, created_at))
            .do_update()
            .set((
                page_view.eq(stat.page_view),
                referrer.eq(stat.referrer),
                updated_at.eq(stat.updated_at),
            ));
        println!("sql: {}", debug_query::<Sqlite, _>(&statement).to_string());
        statement.execute(&mut conn)
    }

    pub fn today(
        conn: PooledConnection<ConnectionManager<SqliteConnection>>,
    ) -> Result<Vec<Statistics>, anyhow::Error> {
        load_statistics_by_created_at(
            conn,
            NaiveDateTime::new(Utc::now().date().naive_utc(), NaiveTime::from_hms(0, 0, 0)),
        )
    }

    pub fn prev_day_rank_avg(conn: PooledConnection<ConnectionManager<SqliteConnection>>) -> i64 {
        let res = load_statistics_by_created_at(
            conn,
            NaiveDateTime::new(Utc::now().date().naive_utc(), NaiveTime::from_hms(0, 0, 0))
                .sub(Duration::hours(24)),
        );
        if let Ok(res) = res {
            let mut sum = 0;
            let mut count = 0;
            res.iter().for_each(|s| {
                let view = s.referrer + (s.page_view / 5);
                sum += view;
                if view > 0 {
                    count = count + 1;
                }
            });
            if count > 0 {
                let rank_svg = sum / count / 10;
                if rank_svg > 0 {
                    return rank_svg;
                }
            }
        }
        1
    }

    pub fn all(
        mut conn: PooledConnection<ConnectionManager<SqliteConnection>>,
    ) -> Result<Vec<Statistics>, anyhow::Error> {
        let res = statistics.load::<Statistics>(&mut conn);
        match res {
            Ok(all) => Ok(all),
            Err(e) => Err(anyhow!("{:?}", e)),
        }
    }
}

fn load_statistics_by_created_at(
    mut conn: PooledConnection<ConnectionManager<SqliteConnection>>,
    _created_at: NaiveDateTime,
) -> Result<Vec<Statistics>, anyhow::Error> {
    println!(
        "sql: {}",
        debug_query::<Sqlite, _>(&statistics.filter(created_at.eq(_created_at))).to_string()
    );
    let res = statistics
        .filter(created_at.eq(_created_at))
        .load::<Statistics>(&mut conn);
    match res {
        Ok(all) => Ok(all),
        Err(e) => Err(anyhow!("{:?}", e)),
    }
}
