# BoringBay Public Analytics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the regression fixes, privacy-safe realtime masked IP notifications, and public aggregate traffic analytics for BoringBay without adding member-side tracking scripts or storing visitor-level history.

**Architecture:** A focused `analytics` module atomically increments hourly and daily SQLite aggregates for total, country, referrer-domain, and internal-channel dimensions. Askama renders a public overview and member pages from privacy-filtered ViewModels; realtime WebSocket payloads receive only a server-generated masked IP. Existing UV/RV tables and ranking formulas remain authoritative and compatible.

**Tech Stack:** Rust 1.98.1, Axum 0.8, Askama 0.16, Diesel 2.3, SQLite WAL, native JavaScript/CSS, Node test runner.

## Global Constraints

- Do not persist or log complete or masked IP addresses.
- Do not store raw visit events or cross-join country and source dimensions.
- Keep `/`, `/rank`, `/join-us`, old Badge/Icon/Favicon endpoints, `/api/ws`, and old `/api/events` payloads compatible.
- Keep UV/RV ranking semantics unchanged; outbound clicks never affect formal ranking.
- Hourly aggregates retain 90 days; daily aggregates retain indefinitely.
- Public dimension buckets below 3 are merged into `其他`; exact event totals remain public.
- Analytics routes return 404 when `BORINGBAY_V2_ENABLED=false`.
- Complete all local verification before pushing; create a PR only after local acceptance is green.

---

### Task 1: Realtime Masked IP Contract

**Files:**
- Modify: `src/visitor.rs`
- Modify: `src/app_model.rs`
- Modify: `resources/static/activity.js`
- Modify: `tests/activity.test.mjs`

**Interfaces:**
- Produces: `VisitorIdentity { dedupe_key, country, masked_ip }`
- Produces: WebSocket JSON field `ip` containing a masked value only
- Consumes: trusted `CF-Connecting-IP` and `CF-IPCountry` headers

- [ ] **Step 1: Add failing Rust tests for IPv4, IPv6, invalid IP, and absence of full IP**

```rust
assert_eq!(mask_ip("203.0.113.9"), Some("203.****.9".into()));
assert_eq!(mask_ip("2001:db8:1:2:3:4:5:6"), Some("2001:db8:****:5:6".into()));
assert_eq!(mask_ip("not-an-ip"), None);
```

- [ ] **Step 2: Run `cargo test visitor --locked` and verify the new tests fail**

- [ ] **Step 3: Parse `IpAddr` server-side and add the masked field to `VisitorIdentity` and `VistEvent`**

```rust
pub fn mask_ip(value: &str) -> Option<String>;
```

- [ ] **Step 4: Update the browser activity renderer and Node test to show country plus masked IP while rejecting the complete IP**

```javascript
assert.match(flattenText(toast), /ES · 203\.\*\*\*\*\.9/);
assert.doesNotMatch(flattenText(toast), /203\.0\.113\.9/);
```

- [ ] **Step 5: Run `cargo test visitor --locked` and `node --test tests/activity.test.mjs`**

- [ ] **Step 6: Commit `feat: restore privacy-safe realtime visitor location`**

### Task 2: Aggregate Storage and Privacy Queries

**Files:**
- Create: `migrations/20260916050000_create_traffic_analytics/up.sql`
- Create: `migrations/20260916050000_create_traffic_analytics/down.sql`
- Create: `src/analytics.rs`
- Modify: `src/schema.rs`
- Modify: `src/lib.rs`
- Modify: `tests/migrations.rs`

**Interfaces:**
- Produces: `AnalyticsService::record(AnalyticsEvent) -> anyhow::Result<()>`
- Produces: `AnalyticsService::overview(range, now) -> anyhow::Result<AnalyticsReport>`
- Produces: `AnalyticsService::member(member_id, range, now) -> anyhow::Result<AnalyticsReport>`
- Produces: `AnalyticsService::prune_hourly(now) -> anyhow::Result<usize>`

- [ ] **Step 1: Add migration tests asserting both new tables, indexes, preserved statistics, and idempotent historical total backfill**

```rust
assert_eq!(traffic_hourly::table.count().get_result::<i64>(&mut conn)?, 0);
assert!(traffic_daily::table.count().get_result::<i64>(&mut conn)? >= seeded_days);
```

- [ ] **Step 2: Add failing module tests for atomic increments, separated dimensions, threshold merging, range comparison, and 90-day pruning**

- [ ] **Step 3: Run the focused migration and analytics tests and verify failure before implementation**

- [ ] **Step 4: Add additive tables with composite primary keys and query indexes; backfill only `total` daily rows from `statistics` and mappable `product_events`**

- [ ] **Step 5: Implement bounded enums and value normalization**

```rust
pub enum AnalyticsEventKind { BadgeView, InboundReferral, OutboundClick }
pub enum AnalyticsChannel { Home, Rank, Route, Random, Feed, Share, Unknown }
pub struct AnalyticsEvent { pub at: NaiveDateTime, pub member_id: i64, pub kind: AnalyticsEventKind, pub country: Option<String>, pub referrer_domain: Option<String>, pub channel: Option<AnalyticsChannel> }
```

- [ ] **Step 6: Implement one transaction that upserts total plus independent optional dimensions into hourly and daily tables**

- [ ] **Step 7: Implement report queries that read totals only from `dimension_kind='total'` and merge displayed dimension buckets below 3 into `其他`**

- [ ] **Step 8: Run `cargo test analytics --locked` and `cargo test --test migrations --locked`**

- [ ] **Step 9: Commit `feat: add privacy-preserving traffic aggregates`**

### Task 3: Connect Existing Traffic Paths

**Files:**
- Modify: `src/app_model.rs`
- Modify: `src/app_router.rs`
- Modify: `src/product_events.rs`
- Modify: `resources/static/discovery.js`
- Modify: `templates/index.html`
- Modify: `templates/rank.html`
- Modify: `templates/route.html`
- Test: `tests/compatibility.rs`
- Test: `tests/pages.rs`

**Interfaces:**
- Consumes: `AnalyticsService::record`
- Extends: `EventInput { kind, member_id, channel?: String }` while preserving old JSON
- Produces: only successfully incremented UV/RV events and accepted outbound events enter analytics

- [ ] **Step 1: Add failing integration tests proving duplicate Badge/RV requests do not duplicate analytics and old event JSON remains accepted**

- [ ] **Step 2: Run those integration tests and verify the missing aggregate behavior**

- [ ] **Step 3: Record Badge and inbound aggregates only inside the existing `visitor_cache.is_none()` branches**

- [ ] **Step 4: Validate optional event channels against the enum and map member/feed outbound events to `outbound_click`**

- [ ] **Step 5: Add stable `data-channel` values to home, rank, route, random, feed, and share interactions**

- [ ] **Step 6: Treat analytics write failures as non-fatal and log only member, kind, and error**

- [ ] **Step 7: Run `cargo test --test compatibility --locked`, `cargo test --test pages --locked`, and all Node tests**

- [ ] **Step 8: Commit `feat: aggregate observable BoringBay traffic paths`**

### Task 4: Public Analytics Pages

**Files:**
- Create: `templates/analytics.html`
- Create: `templates/member_analytics.html`
- Modify: `src/app_router.rs`
- Modify: `src/lib.rs`
- Modify: `templates/base.html`
- Modify: `templates/index.html`
- Modify: `templates/rank.html`
- Modify: `resources/static/app.css`
- Test: `tests/pages.rs`

**Interfaces:**
- Produces: `GET /analytics?range=7|30|90`
- Produces: `GET /analytics/{domain}?range=7|30|90`
- Consumes: privacy-filtered `AnalyticsReport`; templates never receive unfiltered dimension rows

- [ ] **Step 1: Add failing page tests for overview, member page, unknown member 404, invalid range fallback, empty state, and V2-disabled 404**

- [ ] **Step 2: Run the focused page tests and verify routes are absent**

- [ ] **Step 3: Add route handlers that validate member domains, select 7/30/90-day ranges, and map internal errors to generic HTTP responses**

- [ ] **Step 4: Build server-rendered metric cards, trend SVG, hourly bars, member tables, and privacy-filtered dimension lists**

- [ ] **Step 5: Add analytics navigation and per-member links without changing the main member outbound target or ranking formulas**

- [ ] **Step 6: Add responsive, keyboard-visible, reduced-motion-friendly styles with readable no-JavaScript output**

- [ ] **Step 7: Run page, compatibility, and template tests**

- [ ] **Step 8: Commit `feat: add public traffic analytics pages`**

### Task 5: Retention Job, Documentation, and Regression Coverage

**Files:**
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `tests/pages.rs`
- Modify: `tests/migrations.rs`
- Modify: `docs/superpowers/specs/2026-09-16-boringbay-v2-design.md`

**Interfaces:**
- Consumes: `AnalyticsService::prune_hourly`
- Produces: one daily best-effort retention task after startup

- [ ] **Step 1: Add a deterministic pruning boundary test for exactly 90 days and 90 days plus one second**

- [ ] **Step 2: Start a daily retention task only when V2 is enabled; delay first run to avoid startup write contention**

- [ ] **Step 3: Update README with observable-data limits, public routes, retention, privacy threshold, masked realtime IP, and rollback behavior**

- [ ] **Step 4: Amend the V2 design privacy section so it no longer incorrectly claims WebSocket contains no masked IP**

- [ ] **Step 5: Run placeholder, contradiction, and broken-link scans on documentation**

- [ ] **Step 6: Commit `docs: document public analytics and retention`**

### Task 6: Full Local Acceptance and Pull Request

**Files:**
- Verify all changed files
- Store local screenshots outside the repository

**Interfaces:**
- Produces: tested branch and GitHub PR based on current `origin/main`

- [ ] **Step 1: Run `cargo fmt --all -- --check`**

- [ ] **Step 2: Run `cargo clippy --all-targets --all-features --locked -- -D warnings`**

- [ ] **Step 3: Run `cargo test --all-targets --all-features --locked` and all Node tests**

- [ ] **Step 4: Run `cargo build --release --locked` and record the binary SHA-256**

- [ ] **Step 5: Start the release binary with a fresh database and exercise homepage, all rank views, analytics ranges, valid/invalid member pages, Badge/Icon/Favicon, events, WebSocket, routes, Feed fallback, and V2-off behavior**

- [ ] **Step 6: Use Chromium to verify desktop and mobile layouts, realtime masked-IP toast, navigation, empty states, and no-JavaScript readability; save screenshots locally**

- [ ] **Step 7: Verify no complete test IP appears in source output, logs, database, rendered HTML, or WebSocket frames**

- [ ] **Step 8: Rebase or merge current `origin/main` if needed, rerun the full gate, push the branch, and create one PR containing the regression fixes and analytics**

- [ ] **Step 9: Wait for GitHub test, amd64, arm64, and multi-architecture container checks; fix the branch if any check fails**
