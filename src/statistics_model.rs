use std::ops::Sub;

use crate::now_shanghai;
use crate::schema::statistics::{self, dsl::*};
use anyhow::anyhow;
use chrono::{Duration, NaiveDateTime, NaiveTime};
use diesel::dsl::sql;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::sqlite::Sqlite;
use diesel::{debug_query, prelude::*};
use diesel::{Queryable, SqliteConnection};

#[derive(Queryable, Debug, Clone, Insertable, serde::Serialize, serde::Deserialize)]
#[diesel(table_name = statistics)]
pub struct Statistics {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub membership_id: i64,
    pub unique_visitor: i64,
    pub referrer: i64,
    pub latest_referrer_at: NaiveDateTime,
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
                unique_visitor.eq(stat.unique_visitor),
                referrer.eq(stat.referrer),
                latest_referrer_at.eq(stat.latest_referrer_at),
            ))
            .on_conflict((membership_id, created_at))
            .do_update()
            .set((
                unique_visitor.eq(stat.unique_visitor),
                referrer.eq(stat.referrer),
                updated_at.eq(stat.updated_at),
                latest_referrer_at.eq(stat.latest_referrer_at),
            ));
        println!("sql: {}", debug_query::<Sqlite, _>(&statement));
        statement.execute(&mut conn)
    }

    pub fn today(
        conn: PooledConnection<ConnectionManager<SqliteConnection>>,
    ) -> Result<Vec<Statistics>, anyhow::Error> {
        load_statistics_by_created_at(
            conn,
            NaiveDateTime::new(now_shanghai().date(), NaiveTime::from_hms(0, 0, 0)),
        )
    }

    pub fn prev_day_rank_avg(conn: PooledConnection<ConnectionManager<SqliteConnection>>) -> i64 {
        let res = load_statistics_by_created_at(
            conn,
            NaiveDateTime::new(now_shanghai().date(), NaiveTime::from_hms(0, 0, 0))
                .sub(Duration::hours(24)),
        );
        if let Ok(res) = res {
            let mut sum = 0;
            let mut count = 0;
            res.iter().for_each(|s| {
                let view = s.referrer + s.unique_visitor;
                sum += view;
                if view > 0 {
                    count += 1;
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

    pub fn rank_between(
        mut conn: PooledConnection<ConnectionManager<SqliteConnection>>,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<Vec<Statistics>, anyhow::Error> {
        let res = statistics
            .select((
                membership_id,
                sql::<diesel::sql_types::Timestamp>("MIN(created_at) as m_created_at"),
                sql::<diesel::sql_types::BigInt>("SUM(unique_visitor) as s_unique_visitor"),
                sql::<diesel::sql_types::BigInt>("SUM(referrer) as s_referrer"),
            ))
            .filter(created_at.between(start, end))
            .group_by(membership_id)
            .order(sql::<diesel::sql_types::BigInt>("s_referrer DESC"))
            .load::<(i64, NaiveDateTime, i64, i64)>(&mut conn);

        let updated_at_list = statistics
            .select((
                membership_id,
                sql::<diesel::sql_types::Timestamp>("MAX(updated_at) as m_updated_at"),
            ))
            .filter(unique_visitor.gt(0).or(referrer.gt(0)))
            .group_by(membership_id)
            .order(sql::<diesel::sql_types::BigInt>("m_updated_at"))
            .load::<(i64, NaiveDateTime)>(&mut conn);

        let id_to_updated_at = updated_at_list
            .unwrap_or(Vec::new())
            .iter()
            .map(|s| (s.0, s.1))
            .collect::<std::collections::HashMap<i64, NaiveDateTime>>();

        let latest_referrer_at_list = statistics
            .select((
                membership_id,
                sql::<diesel::sql_types::Timestamp>(
                    "MAX(latest_referrer_at) as m_latest_referrer_at",
                ),
            ))
            .filter(unique_visitor.gt(0).or(referrer.gt(0)))
            .group_by(membership_id)
            .order(sql::<diesel::sql_types::BigInt>("m_latest_referrer_at"))
            .load::<(i64, NaiveDateTime)>(&mut conn);

        let id_to_latest_referrer_at = latest_referrer_at_list
            .unwrap_or(Vec::new())
            .iter()
            .map(|s| (s.0, s.1))
            .collect::<std::collections::HashMap<i64, NaiveDateTime>>();

        match res {
            Ok(all) => {
                let mut result = Vec::new();
                all.iter().for_each(|s| {
                    result.push(Statistics {
                        id: 0,
                        created_at: s.1,
                        updated_at: id_to_updated_at
                            .get(&s.0)
                            .unwrap_or(&NaiveDateTime::from_timestamp(0, 0))
                            .to_owned(),
                        latest_referrer_at: id_to_latest_referrer_at
                            .get(&s.0)
                            .unwrap_or(&NaiveDateTime::from_timestamp(0, 0))
                            .to_owned(),
                        membership_id: s.0,
                        unique_visitor: s.2,
                        referrer: s.3,
                    })
                });
                Ok(result)
            }
            Err(e) => Err(anyhow!("{:?}", e)),
        }
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
        debug_query::<Sqlite, _>(&statistics.filter(created_at.eq(_created_at)))
    );
    let res = statistics
        .filter(created_at.eq(_created_at))
        .load::<Statistics>(&mut conn);
    match res {
        Ok(all) => Ok(all),
        Err(e) => Err(anyhow!("{:?}", e)),
    }
}
