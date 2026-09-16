use std::collections::HashSet;

use anyhow::{anyhow, Context, Result};
use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;
use feed_rs::parser;
use reqwest::header::{CONTENT_TYPE, LOCATION};
use sha2::{Digest, Sha256};

use crate::membership_model::Membership;
use crate::network_policy::PublicHttpsUrl;
use crate::schema::{feed_items, feed_sources};
use crate::{now_shanghai, DbPool};

const MAX_FEED_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Queryable)]
struct FeedThrottle {
    feed_url: String,
    last_attempt: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, Queryable, Identifiable)]
#[diesel(table_name = feed_items)]
pub struct FeedItem {
    pub id: i32,
    pub member_id: i64,
    pub item_key: String,
    pub title: String,
    pub url: String,
    pub summary: String,
    pub published_at: NaiveDateTime,
    pub fetched_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = feed_items)]
struct NewFeedItem<'a> {
    member_id: i64,
    item_key: &'a str,
    title: &'a str,
    url: &'a str,
    summary: &'a str,
    published_at: NaiveDateTime,
    fetched_at: NaiveDateTime,
}

#[derive(Clone, Debug)]
pub struct ParsedFeedItem {
    pub item_key: String,
    pub title: String,
    pub url: String,
    pub summary: String,
    pub published_at: NaiveDateTime,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FeedRefresh {
    pub fetched: usize,
    pub stored: usize,
    pub skipped: bool,
}

#[derive(Clone)]
pub struct FeedRepository {
    db_pool: DbPool,
}

impl FeedRepository {
    pub fn new(db_pool: DbPool) -> Self {
        Self { db_pool }
    }

    pub fn latest(&self, limit: usize) -> Result<Vec<FeedItem>> {
        Ok(feed_items::table
            .filter(feed_items::fetched_at.gt(now_shanghai() - Duration::days(7)))
            .order(feed_items::published_at.desc())
            .limit(limit.min(50) as i64)
            .load::<FeedItem>(&mut self.db_pool.get()?)?)
    }

    fn throttle(&self, member_id: i64) -> Result<Option<FeedThrottle>> {
        Ok(feed_sources::table
            .find(member_id)
            .select((feed_sources::feed_url, feed_sources::last_attempt))
            .first::<FeedThrottle>(&mut self.db_pool.get()?)
            .optional()?)
    }

    fn mark_attempt(&self, member_id: i64, feed_url: &str, now: NaiveDateTime) -> Result<()> {
        diesel::sql_query(
            "INSERT INTO feed_sources (member_id, feed_url, last_attempt) VALUES (?1, ?2, ?3) \
             ON CONFLICT(member_id) DO UPDATE SET feed_url = excluded.feed_url, last_attempt = excluded.last_attempt",
        )
        .bind::<diesel::sql_types::BigInt, _>(member_id)
        .bind::<diesel::sql_types::Text, _>(feed_url)
        .bind::<diesel::sql_types::Timestamp, _>(now)
        .execute(&mut self.db_pool.get()?)?;
        Ok(())
    }

    fn mark_failure(&self, member_id: i64, error: &str) -> Result<()> {
        diesel::update(feed_sources::table.find(member_id))
            .set(feed_sources::last_error.eq(Some(truncate(error, 160))))
            .execute(&mut self.db_pool.get()?)?;
        Ok(())
    }

    fn save_success(
        &self,
        member_id: i64,
        items: &[ParsedFeedItem],
        now: NaiveDateTime,
    ) -> Result<usize> {
        Ok(self
            .db_pool
            .get()?
            .transaction::<usize, diesel::result::Error, _>(|connection| {
                let mut stored = 0;
                for item in items {
                    stored += diesel::insert_into(feed_items::table)
                        .values(NewFeedItem {
                            member_id,
                            item_key: &item.item_key,
                            title: &item.title,
                            url: &item.url,
                            summary: &item.summary,
                            published_at: item.published_at,
                            fetched_at: now,
                        })
                        .on_conflict((feed_items::member_id, feed_items::item_key))
                        .do_update()
                        .set((
                            feed_items::title.eq(&item.title),
                            feed_items::url.eq(&item.url),
                            feed_items::summary.eq(&item.summary),
                            feed_items::published_at.eq(item.published_at),
                            feed_items::fetched_at.eq(now),
                        ))
                        .execute(connection)?;
                }
                diesel::update(feed_sources::table.find(member_id))
                    .set((
                        feed_sources::last_success.eq(Some(now)),
                        feed_sources::last_error.eq::<Option<String>>(None),
                    ))
                    .execute(connection)?;
                Ok(stored)
            })?)
    }
}

#[derive(Clone)]
pub struct FeedFetcher {
    repository: FeedRepository,
}

impl FeedFetcher {
    pub fn new(db_pool: DbPool) -> Self {
        Self {
            repository: FeedRepository::new(db_pool),
        }
    }

    pub async fn refresh_member(&self, member: &Membership) -> Result<FeedRefresh> {
        let feed_url = member
            .feed_url
            .as_deref()
            .ok_or_else(|| anyhow!("member has no feed"))?;
        let now = now_shanghai();
        let recently_attempted = self.repository.throttle(member.id)?.map(|source| {
            source.feed_url == feed_url
                && source
                    .last_attempt
                    .map(|attempt| now - attempt < Duration::minutes(30))
                    .unwrap_or(false)
        });
        if recently_attempted.unwrap_or(false) {
            return Ok(FeedRefresh {
                skipped: true,
                ..FeedRefresh::default()
            });
        }
        self.repository.mark_attempt(member.id, feed_url, now)?;
        match fetch_bytes(feed_url)
            .await
            .and_then(|bytes| parse_feed(&bytes))
        {
            Ok(items) => {
                let stored = self.repository.save_success(member.id, &items, now)?;
                Ok(FeedRefresh {
                    fetched: items.len(),
                    stored,
                    skipped: false,
                })
            }
            Err(error) => {
                let _ = self.repository.mark_failure(member.id, &error.to_string());
                Err(error)
            }
        }
    }
}

async fn fetch_bytes(input: &str) -> Result<Vec<u8>> {
    let mut current = input.to_string();
    for redirect_count in 0..=2 {
        let safe_url = PublicHttpsUrl::parse(&current)?;
        let client = safe_url
            .pinned_client("BoringBay-Feed/2.0 (+https://boringbay.com)")
            .await?;
        let mut response = client.get(safe_url.as_url().clone()).send().await?;
        if response.status().is_redirection() {
            if redirect_count == 2 {
                return Err(anyhow!("too many feed redirects"));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| anyhow!("feed redirect has no valid location"))?;
            current = response.url().join(location)?.to_string();
            continue;
        }
        if !response.status().is_success() {
            return Err(anyhow!("feed returned HTTP {}", response.status()));
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !(content_type.contains("xml")
            || content_type.contains("rss")
            || content_type.contains("atom"))
        {
            return Err(anyhow!("feed content type is not XML-like"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len() + chunk.len() > MAX_FEED_BYTES {
                return Err(anyhow!("feed exceeds 1 MiB"));
            }
            bytes.extend_from_slice(&chunk);
        }
        return Ok(bytes);
    }
    Err(anyhow!("feed redirect loop"))
}

pub fn parse_feed(bytes: &[u8]) -> Result<Vec<ParsedFeedItem>> {
    let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(8192)]).to_ascii_lowercase();
    if prefix.contains("<!doctype") || prefix.contains("<!entity") {
        return Err(anyhow!("DTD and external entities are not allowed"));
    }
    let feed = parser::parse(bytes).context("invalid RSS/Atom document")?;
    let mut items = Vec::new();
    for entry in feed.entries.into_iter().take(30) {
        let title = sanitize(
            entry
                .title
                .as_ref()
                .map(|text| text.content.as_str())
                .unwrap_or("Untitled"),
        );
        let link = entry
            .links
            .iter()
            .find(|link| link.rel.as_deref().unwrap_or("alternate") == "alternate")
            .or_else(|| entry.links.first())
            .ok_or_else(|| anyhow!("feed item has no link"))?
            .href
            .clone();
        PublicHttpsUrl::parse(&link).context("feed item URL is not public HTTPS")?;
        let raw_summary = entry
            .summary
            .as_ref()
            .map(|text| text.content.as_str())
            .or_else(|| {
                entry
                    .content
                    .as_ref()
                    .and_then(|content| content.body.as_deref())
            })
            .unwrap_or("");
        let summary = truncate(&sanitize(raw_summary), 160);
        let published_at = entry
            .published
            .or(entry.updated)
            .map(|value| value.naive_utc())
            .unwrap_or_else(now_shanghai);
        let mut hasher = Sha256::new();
        hasher.update(if entry.id.is_empty() {
            link.as_bytes()
        } else {
            entry.id.as_bytes()
        });
        items.push(ParsedFeedItem {
            item_key: hex::encode(hasher.finalize()),
            title: truncate(&title, 200),
            url: link,
            summary,
            published_at,
        });
    }
    Ok(items)
}

pub fn sanitize(input: &str) -> String {
    let allowed = HashSet::new();
    ammonia::Builder::new()
        .tags(allowed)
        .clean(input)
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn truncate(input: &str, limit: usize) -> String {
    let mut value = input.chars().take(limit).collect::<String>();
    if input.chars().count() > limit {
        value.push('…');
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rss_and_atom_into_same_model() {
        for fixture in ["tests/fixtures/rss.xml", "tests/fixtures/atom.xml"] {
            let items = parse_feed(&std::fs::read(fixture).unwrap()).unwrap();
            assert_eq!(items[0].title, "First post");
            assert_eq!(items[0].summary, "Safe summary");
        }
    }

    #[test]
    fn rejects_external_entities_and_strips_script_content() {
        assert!(parse_feed(&std::fs::read("tests/fixtures/xxe.xml").unwrap()).is_err());
        let clean = sanitize("<script>x()</script><p>Safe</p>");
        assert!(!clean.contains("script"));
        assert!(!clean.contains("x()"));
        assert!(clean.contains("Safe"));
    }
}
