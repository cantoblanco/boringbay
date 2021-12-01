use crate::schema::membership::dsl::*;
use anyhow::anyhow;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{Queryable, SqliteConnection};

#[derive(Queryable, Debug, Clone)]
pub struct Membership {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub homepage: String,
    pub domain: String,
    pub contact: String,
    pub total_referrer: i64,
    pub description: String,
}

pub fn all_memberships(conn: &SqliteConnection) -> Result<Vec<Membership>, anyhow::Error> {
    let res = membership.load::<Membership>(conn);
    match res {
        Ok(all) => Ok(all),
        Err(e) => Err(anyhow!("{:?}", e)),
    }
}
