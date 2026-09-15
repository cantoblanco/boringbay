use std::collections::HashMap;

use anyhow::Result;
use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;
use reqwest::header::LOCATION;

use crate::membership_model::Membership;
use crate::network_policy::PublicHttpsUrl;
use crate::schema::site_health;
use crate::{now_shanghai, DbPool};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemberStatus {
    Active,
    Quiet,
    Observation,
    RemovalCandidate,
}

impl MemberStatus {
    pub fn classify(
        activity_age: Duration,
        consecutive_failures: u32,
        badge_missing_age: Option<Duration>,
    ) -> Self {
        if activity_age <= Duration::days(30) {
            return Self::Active;
        }
        if activity_age > Duration::days(90)
            && (consecutive_failures >= 7
                || (consecutive_failures >= 3
                    && badge_missing_age
                        .map(|age| age >= Duration::days(30))
                        .unwrap_or(false)))
        {
            return Self::RemovalCandidate;
        }
        if activity_age > Duration::days(60) && consecutive_failures >= 3 {
            return Self::Observation;
        }
        Self::Quiet
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Active => "活跃",
            Self::Quiet => "安静",
            Self::Observation => "观察中",
            Self::RemovalCandidate => "可移除候选",
        }
    }
}

#[derive(Clone, Debug, Queryable, Identifiable)]
#[diesel(table_name = site_health)]
pub struct SiteHealthRecord {
    pub id: i32,
    pub member_id: i64,
    pub checked_at: NaiveDateTime,
    pub reachable: bool,
    pub status_code: Option<i32>,
    pub badge_state: String,
}

#[derive(Insertable)]
#[diesel(table_name = site_health)]
struct NewSiteHealthRecord<'a> {
    member_id: i64,
    checked_at: NaiveDateTime,
    reachable: bool,
    status_code: Option<i32>,
    badge_state: &'a str,
}

#[derive(Clone, Debug)]
pub struct SiteHealthResult {
    pub member_id: i64,
    pub checked_at: NaiveDateTime,
    pub reachable: bool,
    pub status_code: Option<i32>,
    pub badge_state: &'static str,
}

#[derive(Clone, Debug, Default)]
pub struct HealthEvidence {
    pub last_checked_at: Option<NaiveDateTime>,
    pub consecutive_failures: u32,
    pub badge_missing_since: Option<NaiveDateTime>,
}

#[derive(Clone)]
pub struct SiteHealthService {
    db_pool: DbPool,
}

impl SiteHealthService {
    pub fn new(db_pool: DbPool) -> Result<Self> {
        Ok(Self { db_pool })
    }

    pub async fn probe_member(&self, member: &Membership) -> SiteHealthResult {
        let mut current = format!("https://{}/", member.domain);
        let mut final_status = None;
        let mut reachable = false;

        for redirect_count in 0..=2 {
            let policy_url = match PublicHttpsUrl::parse(&current) {
                Ok(url) => url,
                Err(_) => break,
            };
            let client = match policy_url
                .pinned_client("BoringBay-SiteHealth/2.0 (+https://boringbay.com)")
                .await
            {
                Ok(client) => client,
                Err(_) => break,
            };
            let response = match client.get(policy_url.as_url().clone()).send().await {
                Ok(response) => response,
                Err(_) => break,
            };
            final_status = Some(response.status().as_u16() as i32);
            if response.status().is_redirection() {
                if redirect_count == 2 {
                    reachable = true;
                    break;
                }
                let location = match response
                    .headers()
                    .get(LOCATION)
                    .and_then(|v| v.to_str().ok())
                {
                    Some(value) => value,
                    None => break,
                };
                current = match response.url().join(location) {
                    Ok(url) => url.to_string(),
                    Err(_) => break,
                };
                continue;
            }
            reachable = response.status().is_success();
            break;
        }

        SiteHealthResult {
            member_id: member.id,
            checked_at: now_shanghai(),
            reachable,
            status_code: final_status,
            // Badge absence must not be inferred from a homepage body.
            badge_state: "unknown",
        }
    }

    pub fn save(&self, result: &SiteHealthResult) -> Result<()> {
        diesel::insert_into(site_health::table)
            .values(NewSiteHealthRecord {
                member_id: result.member_id,
                checked_at: result.checked_at,
                reachable: result.reachable,
                status_code: result.status_code,
                badge_state: result.badge_state,
            })
            .execute(&mut self.db_pool.get()?)?;
        Ok(())
    }

    pub fn evidence_by_member(&self) -> Result<HashMap<i64, HealthEvidence>> {
        let records = site_health::table
            .order((site_health::member_id.asc(), site_health::checked_at.desc()))
            .load::<SiteHealthRecord>(&mut self.db_pool.get()?)?;
        let mut evidence = HashMap::<i64, HealthEvidence>::new();
        let mut failure_chain_closed = std::collections::HashSet::new();
        for record in records {
            let entry = evidence.entry(record.member_id).or_default();
            if entry.last_checked_at.is_none() {
                entry.last_checked_at = Some(record.checked_at);
            }
            if record.reachable {
                failure_chain_closed.insert(record.member_id);
            } else if !failure_chain_closed.contains(&record.member_id) {
                entry.consecutive_failures += 1;
            }
            if record.badge_state == "missing" {
                entry.badge_missing_since = Some(
                    entry
                        .badge_missing_since
                        .map(|value| value.min(record.checked_at))
                        .unwrap_or(record.checked_at),
                );
            }
        }
        Ok(evidence)
    }

    pub async fn run_once(&self, members: impl Iterator<Item = Membership>) {
        for member in members {
            let result = self.probe_member(&member).await;
            if let Err(error) = self.save(&result) {
                tracing::warn!(member_id = member.id, "site health save failed: {error}");
            }
        }
    }
}

pub fn status_reason(status: MemberStatus, failures: u32) -> String {
    match status {
        MemberStatus::Active => "最近 30 天有访问或链入活动".to_string(),
        MemberStatus::Quiet => "最近活动较少；不会因此自动移除".to_string(),
        MemberStatus::Observation => format!("长期安静且连续 {failures} 次连通性检测失败"),
        MemberStatus::RemovalCandidate => {
            format!("超过 90 天无活动且连续 {failures} 次检测失败；仍需人工确认")
        }
    }
}

pub fn status_from_evidence(
    now: NaiveDateTime,
    last_activity: NaiveDateTime,
    evidence: &HealthEvidence,
) -> MemberStatus {
    let age = if last_activity.and_utc().timestamp() <= 0 {
        Duration::days(10_000)
    } else {
        now.signed_duration_since(last_activity)
            .max(Duration::zero())
    };
    let badge_age = evidence
        .badge_missing_since
        .map(|since| now.signed_duration_since(since).max(Duration::zero()));
    MemberStatus::classify(age, evidence.consecutive_failures, badge_age)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_site_is_not_a_removal_candidate() {
        assert_eq!(
            MemberStatus::classify(Duration::days(45), 0, None),
            MemberStatus::Quiet
        );
    }

    #[test]
    fn observation_needs_sixty_days_and_three_failures() {
        assert_eq!(
            MemberStatus::classify(Duration::days(61), 2, None),
            MemberStatus::Quiet
        );
        assert_eq!(
            MemberStatus::classify(Duration::days(61), 3, None),
            MemberStatus::Observation
        );
    }

    #[test]
    fn candidate_needs_ninety_days_and_strong_evidence() {
        assert_eq!(
            MemberStatus::classify(Duration::days(91), 6, None),
            MemberStatus::Observation
        );
        assert_eq!(
            MemberStatus::classify(Duration::days(91), 7, None),
            MemberStatus::RemovalCandidate
        );
    }
}
