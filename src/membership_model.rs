use chrono::NaiveDateTime;
use diesel::Queryable;

#[derive(Queryable, Debug, Clone)]
pub struct Membership {
    pub id: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub homepage: String,
    pub domain: String,
    pub contact: String,
    pub total_referrer: i64,
    pub description: String,
}

