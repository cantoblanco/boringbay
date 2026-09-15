use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use anyhow::{anyhow, Result};
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

use crate::membership_model::Membership;
use crate::schema::daily_routes;
use crate::{now_shanghai, DbPool};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DailyRoute {
    pub date: NaiveDate,
    pub member_ids: Vec<i64>,
}

#[derive(Queryable, Identifiable)]
#[diesel(table_name = daily_routes)]
#[diesel(primary_key(route_date))]
struct DailyRouteRecord {
    route_date: NaiveDate,
    member_ids: String,
    _generated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = daily_routes)]
struct NewDailyRoute<'a> {
    route_date: NaiveDate,
    member_ids: &'a str,
    generated_at: NaiveDateTime,
}

#[derive(Clone)]
pub struct DiscoveryService {
    db_pool: DbPool,
}

impl DiscoveryService {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }

    pub fn daily_route(&self, date: NaiveDate, members: &[Membership]) -> Result<DailyRoute> {
        let mut connection = self.db_pool.get()?;
        connection.transaction(|connection| {
            if let Some(route) = load_route(connection, date)? {
                return Ok(route);
            }
            let exposure = route_exposure(connection)?;
            let generated = generate(date, members, &exposure)?;
            let json = serde_json::to_string(&generated.member_ids)?;
            diesel::insert_into(daily_routes::table)
                .values(NewDailyRoute {
                    route_date: date,
                    member_ids: &json,
                    generated_at: now_shanghai(),
                })
                .on_conflict(daily_routes::route_date)
                .do_nothing()
                .execute(connection)?;
            load_route(connection, date)?.ok_or_else(|| anyhow!("daily route was not persisted"))
        })
    }

    pub fn ordered_candidates(
        seed: u64,
        members: &[Membership],
        exposure: &HashMap<i64, i64>,
    ) -> Vec<i64> {
        ordered_candidates(seed, members, exposure)
    }
}

pub fn generate(
    date: NaiveDate,
    members: &[Membership],
    exposure: &HashMap<i64, i64>,
) -> Result<DailyRoute> {
    if members.len() < 5 {
        return Err(anyhow!("at least five eligible members are required"));
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    date.hash(&mut hasher);
    members
        .iter()
        .map(|member| member.id)
        .for_each(|id| id.hash(&mut hasher));
    let ordered = ordered_candidates(hasher.finish(), members, exposure);
    let by_id = members
        .iter()
        .map(|member| (member.id, member))
        .collect::<HashMap<_, _>>();
    let mut selected = Vec::with_capacity(5);
    let mut used_tags = HashSet::<String>::new();

    for id in &ordered {
        let member = by_id[id];
        let tags = member.tags.clone().unwrap_or_default();
        if tags.iter().any(|tag| !used_tags.contains(tag)) {
            selected.push(*id);
            used_tags.extend(tags);
        }
        if selected.len() == 5 {
            break;
        }
    }
    for id in ordered {
        if !selected.contains(&id) {
            selected.push(id);
        }
        if selected.len() == 5 {
            break;
        }
    }
    Ok(DailyRoute {
        date,
        member_ids: selected,
    })
}

pub fn ordered_candidates(
    seed: u64,
    members: &[Membership],
    exposure: &HashMap<i64, i64>,
) -> Vec<i64> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut scored = members
        .iter()
        .map(|member| {
            let count = exposure.get(&member.id).copied().unwrap_or(0).max(0) as f64;
            let weight = 1.0 / (1.0 + count);
            let key = rng.gen::<f64>().powf(1.0 / weight);
            (member.id, key)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.into_iter().map(|(id, _)| id).collect()
}

fn load_route(connection: &mut SqliteConnection, date: NaiveDate) -> Result<Option<DailyRoute>> {
    let record = daily_routes::table
        .filter(daily_routes::route_date.eq(date))
        .first::<DailyRouteRecord>(connection)
        .optional()?;
    record
        .map(|record| {
            let member_ids = serde_json::from_str::<Vec<i64>>(&record.member_ids)?;
            if member_ids.len() != 5 || member_ids.iter().collect::<HashSet<_>>().len() != 5 {
                return Err(anyhow!("stored daily route is invalid"));
            }
            Ok(DailyRoute {
                date: record.route_date,
                member_ids,
            })
        })
        .transpose()
}

fn route_exposure(connection: &mut SqliteConnection) -> Result<HashMap<i64, i64>> {
    let routes = daily_routes::table
        .select(daily_routes::member_ids)
        .load::<String>(connection)?;
    let mut exposure = HashMap::new();
    for route in routes {
        for id in serde_json::from_str::<Vec<i64>>(&route).unwrap_or_default() {
            *exposure.entry(id).or_insert(0) += 1;
        }
    }
    Ok(exposure)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members() -> Vec<Membership> {
        (1..=8)
            .map(|id| Membership {
                id,
                domain: format!("{id}.example.com"),
                name: format!("Member {id}"),
                icon: String::new(),
                description: String::new(),
                github_username: String::new(),
                tags: Some(vec![format!("tag-{}", id % 4)]),
                feed_url: None,
                hidden: None,
            })
            .collect()
    }

    #[test]
    fn daily_route_has_five_unique_eligible_members() {
        let route = generate(
            NaiveDate::from_ymd_opt(2026, 9, 16).unwrap(),
            &members(),
            &HashMap::new(),
        )
        .unwrap();
        assert_eq!(route.member_ids.len(), 5);
        assert_eq!(route.member_ids.iter().collect::<HashSet<_>>().len(), 5);
        assert!(route.member_ids.iter().all(|id| (1..=8).contains(id)));
    }

    #[test]
    fn same_date_and_state_produces_same_order() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 16).unwrap();
        assert_eq!(
            generate(date, &members(), &HashMap::new()).unwrap(),
            generate(date, &members(), &HashMap::new()).unwrap()
        );
    }
}
