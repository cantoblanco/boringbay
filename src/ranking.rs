use std::cmp::Ordering;
use std::collections::HashMap;

use anyhow::Result;
use chrono::{Duration, NaiveDateTime};

use crate::{statistics_model::Statistics, DbPool};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RankingWindow {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RankingEntry {
    pub membership_id: i64,
    pub uv: i64,
    pub rv: i64,
    pub joined_at: NaiveDateTime,
    pub last_activity: NaiveDateTime,
    pub score: i64,
    pub growth_rate: Option<f64>,
}

#[derive(Default)]
struct Aggregate {
    uv: i64,
    rv: i64,
    joined_at: Option<NaiveDateTime>,
    last_activity: Option<NaiveDateTime>,
}

pub struct RankingService {
    db_pool: DbPool,
}

impl RankingService {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }

    pub fn classic(&self, now: NaiveDateTime) -> Result<Vec<RankingEntry>> {
        let rows = Statistics::all(self.db_pool.get()?)?;
        Ok(rank_classic(aggregate(&rows, None, now)))
    }

    pub fn activity_30d(&self, now: NaiveDateTime) -> Result<Vec<RankingEntry>> {
        let rows = Statistics::all(self.db_pool.get()?)?;
        let window = RankingWindow {
            start: now - Duration::days(30),
            end: now,
        };
        Ok(rank_activity(aggregate(
            &rows,
            Some(window.start),
            window.end,
        )))
    }

    pub fn rising_7d(&self, now: NaiveDateTime) -> Result<Vec<RankingEntry>> {
        let rows = Statistics::all(self.db_pool.get()?)?;
        let current = aggregate(&rows, Some(now - Duration::days(7)), now);
        let previous = aggregate(
            &rows,
            Some(now - Duration::days(14)),
            now - Duration::days(7),
        );
        Ok(rank_rising(current, previous))
    }
}

fn aggregate(
    rows: &[Statistics],
    start: Option<NaiveDateTime>,
    end: NaiveDateTime,
) -> HashMap<i64, Aggregate> {
    let mut result = HashMap::<i64, Aggregate>::new();
    for row in rows.iter().filter(|row| {
        row.created_at < end && start.map(|start| row.created_at >= start).unwrap_or(true)
    }) {
        let item = result.entry(row.membership_id).or_default();
        item.uv += row.unique_visitor;
        item.rv += row.referrer;
        item.joined_at = Some(
            item.joined_at
                .map(|value| value.min(row.created_at))
                .unwrap_or(row.created_at),
        );
        item.last_activity = Some(
            item.last_activity
                .map(|value| value.max(row.last_activity()))
                .unwrap_or_else(|| row.last_activity()),
        );
    }
    result
}

fn into_entry(membership_id: i64, aggregate: Aggregate) -> RankingEntry {
    RankingEntry {
        membership_id,
        uv: aggregate.uv,
        rv: aggregate.rv,
        joined_at: aggregate.joined_at.unwrap_or_else(epoch),
        last_activity: aggregate.last_activity.unwrap_or_else(epoch),
        score: aggregate.uv + aggregate.rv,
        growth_rate: None,
    }
}

fn rank_activity(entries: HashMap<i64, Aggregate>) -> Vec<RankingEntry> {
    let mut entries = entries
        .into_iter()
        .map(|(id, aggregate)| into_entry(id, aggregate))
        .collect::<Vec<_>>();
    entries.sort_by(activity_order);
    entries
}

fn rank_classic(entries: HashMap<i64, Aggregate>) -> Vec<RankingEntry> {
    let mut entries = entries
        .into_iter()
        .map(|(id, aggregate)| into_entry(id, aggregate))
        .collect::<Vec<_>>();
    // Preserve the historic BoringBay ordering: inbound traffic, then visits.
    entries.sort_by(|a, b| {
        b.rv.cmp(&a.rv)
            .then_with(|| b.uv.cmp(&a.uv))
            .then_with(|| b.last_activity.cmp(&a.last_activity))
            .then_with(|| a.membership_id.cmp(&b.membership_id))
    });
    entries
}

fn rank_rising(
    current: HashMap<i64, Aggregate>,
    previous: HashMap<i64, Aggregate>,
) -> Vec<RankingEntry> {
    let mut entries = current
        .into_iter()
        .filter_map(|(id, aggregate)| {
            let current_total = aggregate.uv + aggregate.rv;
            if current_total < 5 {
                return None;
            }
            let previous_total = previous
                .get(&id)
                .map(|entry| entry.uv + entry.rv)
                .unwrap_or(0);
            let mut entry = into_entry(id, aggregate);
            entry.growth_rate =
                Some((current_total - previous_total) as f64 / previous_total.max(5) as f64);
            Some(entry)
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        b.growth_rate
            .partial_cmp(&a.growth_rate)
            .unwrap_or(Ordering::Equal)
            .then_with(|| activity_order(a, b))
    });
    entries
}

fn activity_order(a: &RankingEntry, b: &RankingEntry) -> Ordering {
    b.score
        .cmp(&a.score)
        .then_with(|| b.rv.cmp(&a.rv))
        .then_with(|| b.uv.cmp(&a.uv))
        .then_with(|| b.last_activity.cmp(&a.last_activity))
        .then_with(|| a.membership_id.cmp(&b.membership_id))
}

fn epoch() -> NaiveDateTime {
    chrono::DateTime::from_timestamp(0, 0)
        .expect("unix epoch must exist")
        .naive_utc()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(day: u32) -> NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 9, day)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn aggregate_entry(uv: i64, rv: i64) -> Aggregate {
        Aggregate {
            uv,
            rv,
            joined_at: Some(dt(1)),
            last_activity: Some(dt(12)),
        }
    }

    #[test]
    fn activity_rank_uses_uv_plus_rv_and_transparent_ties() {
        let entries = rank_activity(HashMap::from([
            (1, aggregate_entry(5, 10)),
            (2, aggregate_entry(9, 6)),
        ]));
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.membership_id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn rising_rank_requires_five_recent_events() {
        let below_threshold =
            rank_rising(HashMap::from([(1, aggregate_entry(4, 0))]), HashMap::new());
        assert!(below_threshold.is_empty());

        let entry = rank_rising(
            HashMap::from([(1, aggregate_entry(10, 0))]),
            HashMap::from([(1, aggregate_entry(5, 0))]),
        );
        assert_eq!(entry[0].growth_rate, Some(1.0));
    }
}
