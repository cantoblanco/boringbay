use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, Result};
use chrono::{Duration, NaiveDate, NaiveDateTime, Timelike};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Date, Nullable, Text, Timestamp},
};

use crate::DbPool;

const TOTAL: &str = "total";
const COUNTRY: &str = "country";
const REFERRER_DOMAIN: &str = "referrer_domain";
const CHANNEL: &str = "channel";
const PUBLIC_BUCKET_MIN: i64 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalyticsEventKind {
    BadgeView,
    InboundReferral,
    OutboundClick,
}

impl AnalyticsEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BadgeView => "badge_view",
            Self::InboundReferral => "inbound_referral",
            Self::OutboundClick => "outbound_click",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalyticsChannel {
    Home,
    Rank,
    Route,
    Random,
    Feed,
    Share,
    Unknown,
}

impl AnalyticsChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Rank => "rank",
            Self::Route => "route",
            Self::Random => "random",
            Self::Feed => "feed",
            Self::Share => "share",
            Self::Unknown => "unknown",
        }
    }
}

impl TryFrom<&str> for AnalyticsChannel {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        match value {
            "home" => Ok(Self::Home),
            "rank" => Ok(Self::Rank),
            "route" => Ok(Self::Route),
            "random" => Ok(Self::Random),
            "feed" => Ok(Self::Feed),
            "share" => Ok(Self::Share),
            "unknown" | "" => Ok(Self::Unknown),
            _ => Err(anyhow!("unknown analytics channel")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnalyticsEvent {
    pub at: NaiveDateTime,
    pub member_id: i64,
    pub kind: AnalyticsEventKind,
    pub country: Option<String>,
    pub referrer_domain: Option<String>,
    pub channel: Option<AnalyticsChannel>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MetricSet {
    pub badge_views: i64,
    pub inbound_referrals: i64,
    pub outbound_clicks: i64,
}

impl MetricSet {
    pub fn total(self) -> i64 {
        self.badge_views + self.inbound_referrals + self.outbound_clicks
    }

    fn add(&mut self, kind: &str, count: i64) {
        match kind {
            "badge_view" => self.badge_views += count,
            "inbound_referral" => self.inbound_referrals += count,
            "outbound_click" => self.outbound_clicks += count,
            _ => {}
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DailyPoint {
    pub date: NaiveDate,
    pub metrics: MetricSet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HourPoint {
    pub hour: u32,
    pub count: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionCount {
    pub label: String,
    pub count: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberAggregate {
    pub member_id: i64,
    pub metrics: MetricSet,
    pub previous_total: i64,
}

impl MemberAggregate {
    pub fn growth_percent(&self) -> i64 {
        let current = self.metrics.total();
        if self.previous_total == 0 {
            return if current > 0 { 100 } else { 0 };
        }
        ((current - self.previous_total) * 100) / self.previous_total
    }
}

#[derive(Clone, Debug)]
pub struct AnalyticsReport {
    pub days: i64,
    pub current: MetricSet,
    pub previous: MetricSet,
    pub daily: Vec<DailyPoint>,
    pub hourly: Vec<HourPoint>,
    pub countries: Vec<DimensionCount>,
    pub referrers: Vec<DimensionCount>,
    pub channels: Vec<DimensionCount>,
    pub members: Vec<MemberAggregate>,
}

#[derive(Clone)]
pub struct AnalyticsService {
    db_pool: DbPool,
}

impl AnalyticsService {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }

    pub fn record(&self, event: AnalyticsEvent) -> Result<()> {
        if event.member_id <= 0 {
            return Err(anyhow!("analytics member_id must be positive"));
        }
        let hour = event
            .at
            .date()
            .and_hms_opt(event.at.hour(), 0, 0)
            .expect("valid hour bucket");
        let day = event.at.date();
        let mut dimensions = vec![(TOTAL.to_string(), String::new())];
        if let Some(country) = event.country.as_deref().and_then(normalize_country) {
            dimensions.push((COUNTRY.to_string(), country));
        }
        if let Some(domain) = event
            .referrer_domain
            .as_deref()
            .and_then(normalize_domain)
        {
            dimensions.push((REFERRER_DOMAIN.to_string(), domain));
        }
        if let Some(channel) = event.channel {
            dimensions.push((CHANNEL.to_string(), channel.as_str().to_string()));
        }

        self.db_pool.get()?.transaction(|connection| {
            for (dimension_kind, dimension_value) in &dimensions {
                diesel::sql_query(
                    "INSERT INTO traffic_hourly \
                     (bucket_start, member_id, event_kind, dimension_kind, dimension_value, count) \
                     VALUES (?1, ?2, ?3, ?4, ?5, 1) \
                     ON CONFLICT(bucket_start, member_id, event_kind, dimension_kind, dimension_value) \
                     DO UPDATE SET count = count + 1",
                )
                .bind::<Timestamp, _>(hour)
                .bind::<BigInt, _>(event.member_id)
                .bind::<Text, _>(event.kind.as_str())
                .bind::<Text, _>(dimension_kind)
                .bind::<Text, _>(dimension_value)
                .execute(connection)?;
                diesel::sql_query(
                    "INSERT INTO traffic_daily \
                     (bucket_date, member_id, event_kind, dimension_kind, dimension_value, count) \
                     VALUES (?1, ?2, ?3, ?4, ?5, 1) \
                     ON CONFLICT(bucket_date, member_id, event_kind, dimension_kind, dimension_value) \
                     DO UPDATE SET count = count + 1",
                )
                .bind::<Date, _>(day)
                .bind::<BigInt, _>(event.member_id)
                .bind::<Text, _>(event.kind.as_str())
                .bind::<Text, _>(dimension_kind)
                .bind::<Text, _>(dimension_value)
                .execute(connection)?;
            }
            Ok::<_, diesel::result::Error>(())
        })?;
        Ok(())
    }

    pub fn overview(&self, days: i64, now: NaiveDateTime) -> Result<AnalyticsReport> {
        self.report(None, days, now)
    }

    pub fn member(&self, member_id: i64, days: i64, now: NaiveDateTime) -> Result<AnalyticsReport> {
        if member_id <= 0 {
            return Err(anyhow!("member_id must be positive"));
        }
        self.report(Some(member_id), days, now)
    }

    pub fn prune_hourly(&self, now: NaiveDateTime) -> Result<usize> {
        let cutoff = now - Duration::days(90);
        Ok(diesel::sql_query("DELETE FROM traffic_hourly WHERE bucket_start < ?1")
            .bind::<Timestamp, _>(cutoff)
            .execute(&mut self.db_pool.get()?)?)
    }

    fn report(
        &self,
        member_id: Option<i64>,
        days: i64,
        now: NaiveDateTime,
    ) -> Result<AnalyticsReport> {
        let days = match days {
            7 | 30 | 90 => days,
            _ => 30,
        };
        let end = now.date() + Duration::days(1);
        let start = end - Duration::days(days);
        let previous_start = start - Duration::days(days);
        let mut connection = self.db_pool.get()?;
        let current = load_totals(&mut connection, start, end, member_id)?;
        let previous = load_totals(&mut connection, previous_start, start, member_id)?;
        let daily = load_daily(&mut connection, start, end, member_id)?;
        let hourly_start = now - Duration::days(days.min(90));
        let hourly = load_hourly(&mut connection, hourly_start, now, member_id)?;
        let countries = load_dimension(&mut connection, start, end, member_id, COUNTRY)?;
        let referrers =
            load_dimension(&mut connection, start, end, member_id, REFERRER_DOMAIN)?;
        let channels = load_dimension(&mut connection, start, end, member_id, CHANNEL)?;
        let members = if member_id.is_none() {
            load_members(&mut connection, start, end, previous_start)?
        } else {
            Vec::new()
        };
        Ok(AnalyticsReport {
            days,
            current,
            previous,
            daily,
            hourly,
            countries,
            referrers,
            channels,
            members,
        })
    }
}

#[derive(QueryableByName)]
struct KindCount {
    #[diesel(sql_type = Text)]
    event_kind: String,
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct DayKindCount {
    #[diesel(sql_type = Date)]
    bucket_date: NaiveDate,
    #[diesel(sql_type = Text)]
    event_kind: String,
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct LabelCount {
    #[diesel(sql_type = Text)]
    label: String,
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct MemberKindCount {
    #[diesel(sql_type = BigInt)]
    member_id: i64,
    #[diesel(sql_type = Text)]
    event_kind: String,
    #[diesel(sql_type = BigInt)]
    count: i64,
}

fn load_totals(
    connection: &mut SqliteConnection,
    start: NaiveDate,
    end: NaiveDate,
    member_id: Option<i64>,
) -> Result<MetricSet> {
    let rows = diesel::sql_query(
        "SELECT event_kind, SUM(count) AS count FROM traffic_daily \
         WHERE bucket_date >= ?1 AND bucket_date < ?2 AND dimension_kind = 'total' \
         AND (?3 IS NULL OR member_id = ?3) GROUP BY event_kind",
    )
    .bind::<Date, _>(start)
    .bind::<Date, _>(end)
    .bind::<Nullable<BigInt>, _>(member_id)
    .load::<KindCount>(connection)?;
    let mut metrics = MetricSet::default();
    for row in rows {
        metrics.add(&row.event_kind, row.count);
    }
    Ok(metrics)
}

fn load_daily(
    connection: &mut SqliteConnection,
    start: NaiveDate,
    end: NaiveDate,
    member_id: Option<i64>,
) -> Result<Vec<DailyPoint>> {
    let rows = diesel::sql_query(
        "SELECT bucket_date, event_kind, SUM(count) AS count FROM traffic_daily \
         WHERE bucket_date >= ?1 AND bucket_date < ?2 AND dimension_kind = 'total' \
         AND (?3 IS NULL OR member_id = ?3) GROUP BY bucket_date, event_kind ORDER BY bucket_date",
    )
    .bind::<Date, _>(start)
    .bind::<Date, _>(end)
    .bind::<Nullable<BigInt>, _>(member_id)
    .load::<DayKindCount>(connection)?;
    let mut by_day = BTreeMap::<NaiveDate, MetricSet>::new();
    for offset in 0..(end - start).num_days() {
        by_day.insert(start + Duration::days(offset), MetricSet::default());
    }
    for row in rows {
        by_day
            .entry(row.bucket_date)
            .or_default()
            .add(&row.event_kind, row.count);
    }
    Ok(by_day
        .into_iter()
        .map(|(date, metrics)| DailyPoint { date, metrics })
        .collect())
}

fn load_hourly(
    connection: &mut SqliteConnection,
    start: NaiveDateTime,
    end: NaiveDateTime,
    member_id: Option<i64>,
) -> Result<Vec<HourPoint>> {
    let rows = diesel::sql_query(
        "SELECT strftime('%H', bucket_start) AS label, SUM(count) AS count FROM traffic_hourly \
         WHERE bucket_start >= ?1 AND bucket_start <= ?2 AND dimension_kind = 'total' \
         AND (?3 IS NULL OR member_id = ?3) GROUP BY label ORDER BY label",
    )
    .bind::<Timestamp, _>(start)
    .bind::<Timestamp, _>(end)
    .bind::<Nullable<BigInt>, _>(member_id)
    .load::<LabelCount>(connection)?;
    let counts = rows
        .into_iter()
        .filter_map(|row| row.label.parse::<u32>().ok().map(|hour| (hour, row.count)))
        .collect::<HashMap<_, _>>();
    Ok((0..24)
        .map(|hour| HourPoint {
            hour,
            count: counts.get(&hour).copied().unwrap_or(0),
        })
        .collect())
}

fn load_dimension(
    connection: &mut SqliteConnection,
    start: NaiveDate,
    end: NaiveDate,
    member_id: Option<i64>,
    dimension_kind: &str,
) -> Result<Vec<DimensionCount>> {
    let rows = diesel::sql_query(
        "SELECT dimension_value AS label, SUM(count) AS count FROM traffic_daily \
         WHERE bucket_date >= ?1 AND bucket_date < ?2 AND dimension_kind = ?3 \
         AND (?4 IS NULL OR member_id = ?4) GROUP BY dimension_value ORDER BY count DESC, label",
    )
    .bind::<Date, _>(start)
    .bind::<Date, _>(end)
    .bind::<Text, _>(dimension_kind)
    .bind::<Nullable<BigInt>, _>(member_id)
    .load::<LabelCount>(connection)?;
    Ok(public_dimensions(
        rows.into_iter()
            .map(|row| DimensionCount {
                label: row.label,
                count: row.count,
            })
            .collect(),
    ))
}

fn load_members(
    connection: &mut SqliteConnection,
    start: NaiveDate,
    end: NaiveDate,
    previous_start: NaiveDate,
) -> Result<Vec<MemberAggregate>> {
    let current = load_member_rows(connection, start, end)?;
    let previous = load_member_rows(connection, previous_start, start)?;
    let mut by_member = HashMap::<i64, MetricSet>::new();
    for row in current {
        by_member
            .entry(row.member_id)
            .or_default()
            .add(&row.event_kind, row.count);
    }
    let mut previous_by_member = HashMap::<i64, MetricSet>::new();
    for row in previous {
        previous_by_member
            .entry(row.member_id)
            .or_default()
            .add(&row.event_kind, row.count);
    }
    let mut result = by_member
        .into_iter()
        .map(|(member_id, metrics)| MemberAggregate {
            member_id,
            metrics,
            previous_total: previous_by_member
                .get(&member_id)
                .copied()
                .unwrap_or_default()
                .total(),
        })
        .collect::<Vec<_>>();
    result.sort_by(|a, b| {
        b.metrics
            .total()
            .cmp(&a.metrics.total())
            .then_with(|| a.member_id.cmp(&b.member_id))
    });
    result.truncate(50);
    Ok(result)
}

fn load_member_rows(
    connection: &mut SqliteConnection,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Vec<MemberKindCount>> {
    Ok(diesel::sql_query(
        "SELECT member_id, event_kind, SUM(count) AS count FROM traffic_daily \
         WHERE bucket_date >= ?1 AND bucket_date < ?2 AND dimension_kind = 'total' \
         GROUP BY member_id, event_kind",
    )
    .bind::<Date, _>(start)
    .bind::<Date, _>(end)
    .load(connection)?)
}

fn public_dimensions(rows: Vec<DimensionCount>) -> Vec<DimensionCount> {
    let mut visible = Vec::new();
    let mut other = 0;
    for row in rows {
        if row.count < PUBLIC_BUCKET_MIN {
            other += row.count;
        } else {
            visible.push(row);
        }
    }
    if other > 0 {
        visible.push(DimensionCount {
            label: "其他".to_string(),
            count: other,
        });
    }
    visible
}

fn normalize_country(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_uppercase();
    (value.len() == 2 && value.chars().all(|character| character.is_ascii_alphanumeric()))
        .then_some(value)
}

fn normalize_domain(value: &str) -> Option<String> {
    let value = value.trim().trim_end_matches('.').to_ascii_lowercase();
    if value.is_empty()
        || value.len() > 253
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    {
        return None;
    }
    Some(value.strip_prefix("www.").unwrap_or(&value).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{establish_connection, run_migrations};

    fn service() -> (tempfile::TempDir, AnalyticsService) {
        let temp = tempfile::tempdir().unwrap();
        let url = temp.path().join("analytics.db").display().to_string();
        let pool = establish_connection(&url);
        run_migrations(&mut pool.get().unwrap()).unwrap();
        (temp, AnalyticsService::new(pool))
    }

    fn event(at: NaiveDateTime) -> AnalyticsEvent {
        AnalyticsEvent {
            at,
            member_id: 1,
            kind: AnalyticsEventKind::BadgeView,
            country: Some("es".to_string()),
            referrer_domain: Some("WWW.Example.COM.".to_string()),
            channel: Some(AnalyticsChannel::Home),
        }
    }

    #[test]
    fn records_totals_and_independent_dimensions_without_double_counting() {
        let (_temp, service) = service();
        let now = NaiveDate::from_ymd_opt(2026, 9, 16)
            .unwrap()
            .and_hms_opt(12, 30, 0)
            .unwrap();
        for _ in 0..3 {
            service.record(event(now)).unwrap();
        }
        let report = service.member(1, 7, now).unwrap();
        assert_eq!(report.current.badge_views, 3);
        assert_eq!(report.countries[0], DimensionCount { label: "ES".into(), count: 3 });
        assert_eq!(report.referrers[0].label, "example.com");
        assert_eq!(report.channels[0].label, "home");
    }

    #[test]
    fn hides_small_public_buckets_but_keeps_exact_total() {
        let (_temp, service) = service();
        let now = NaiveDate::from_ymd_opt(2026, 9, 16)
            .unwrap()
            .and_hms_opt(8, 0, 0)
            .unwrap();
        service.record(event(now)).unwrap();
        service.record(event(now)).unwrap();
        let report = service.member(1, 7, now).unwrap();
        assert_eq!(report.current.badge_views, 2);
        assert_eq!(report.countries, vec![DimensionCount { label: "其他".into(), count: 2 }]);
    }

    #[test]
    fn prunes_only_rows_older_than_ninety_days() {
        let (_temp, service) = service();
        let now = NaiveDate::from_ymd_opt(2026, 9, 16)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        service.record(event(now - Duration::days(90))).unwrap();
        service
            .record(event(now - Duration::days(90) - Duration::seconds(1)))
            .unwrap();
        assert_eq!(service.prune_hourly(now).unwrap(), 4);
        let report = service.member(1, 90, now).unwrap();
        assert_eq!(report.hourly.iter().map(|point| point.count).sum::<i64>(), 1);
    }

    #[test]
    fn normalizers_reject_unbounded_values() {
        assert_eq!(normalize_country("es").as_deref(), Some("ES"));
        assert_eq!(normalize_country("Europe"), None);
        assert_eq!(normalize_domain("WWW.Example.COM.").as_deref(), Some("example.com"));
        assert_eq!(normalize_domain("https://example.com/path"), None);
    }
}
