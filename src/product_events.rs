use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use serde::{Deserialize, Serialize};

use crate::DbPool;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductEventKind {
    RouteStart,
    RouteComplete,
    RandomUse,
    MemberOutbound,
    FeedOutbound,
    ShareClick,
    ShareOpen,
    JoinView,
    JoinEdit,
    LocalReturn,
}

impl ProductEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RouteStart => "route_start",
            Self::RouteComplete => "route_complete",
            Self::RandomUse => "random_use",
            Self::MemberOutbound => "member_outbound",
            Self::FeedOutbound => "feed_outbound",
            Self::ShareClick => "share_click",
            Self::ShareOpen => "share_open",
            Self::JoinView => "join_view",
            Self::JoinEdit => "join_edit",
            Self::LocalReturn => "local_return",
        }
    }
}

impl TryFrom<&str> for ProductEventKind {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self> {
        match value {
            "route_start" => Ok(Self::RouteStart),
            "route_complete" => Ok(Self::RouteComplete),
            "random_use" => Ok(Self::RandomUse),
            "member_outbound" => Ok(Self::MemberOutbound),
            "feed_outbound" => Ok(Self::FeedOutbound),
            "share_click" => Ok(Self::ShareClick),
            "share_open" => Ok(Self::ShareOpen),
            "join_view" => Ok(Self::JoinView),
            "join_edit" => Ok(Self::JoinEdit),
            "local_return" => Ok(Self::LocalReturn),
            _ => Err(anyhow!("unknown product event")),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventInput {
    pub kind: String,
    pub member_id: Option<i64>,
    #[serde(default)]
    pub channel: Option<String>,
}

#[derive(Clone)]
pub struct ProductEventService {
    db_pool: DbPool,
}

impl ProductEventService {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }

    pub fn increment(
        &self,
        day: NaiveDate,
        kind: ProductEventKind,
        member_id: Option<i64>,
    ) -> Result<()> {
        let member_id = member_id.unwrap_or(0);
        if member_id < 0 {
            return Err(anyhow!("member_id must be positive"));
        }
        diesel::sql_query(
            "INSERT INTO product_events (event_date, event_kind, member_id, count) \
             VALUES (?1, ?2, ?3, 1) \
             ON CONFLICT(event_date, event_kind, member_id) DO UPDATE SET count = count + 1",
        )
        .bind::<Text, _>(day.format("%Y-%m-%d").to_string())
        .bind::<Text, _>(kind.as_str())
        .bind::<BigInt, _>(member_id)
        .execute(&mut self.db_pool.get()?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_event_is_rejected() {
        assert!(ProductEventKind::try_from("arbitrary-user-text").is_err());
    }

    #[test]
    fn event_payload_has_no_identity_fields() {
        let json = serde_json::to_value(EventInput {
            kind: "route_start".to_string(),
            member_id: None,
            channel: None,
        })
        .unwrap();
        let keys = json
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(keys, vec!["channel", "kind", "member_id"]);
    }
}
