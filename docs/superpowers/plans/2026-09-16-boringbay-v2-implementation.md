# BoringBay V2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a locally tested BoringBay V2 that preserves every existing route, member record, historical statistic and recognizable design element while adding safer analytics, transparent rankings, status observation, fair discovery, a local passport, opt-in feeds and shareable daily routes.

**Architecture:** Keep the Rust/Axum/Askama/Diesel/SQLite monolith, split new behavior into focused modules, and use additive Diesel migrations. Server-render all essential content; use small local JavaScript modules only for discovery progress, the passport and share-card download. Protect every new experience with graceful degradation and a `BORINGBAY_V2_ENABLED` feature flag.

**Tech Stack:** Rust 2021, Axum 0.4, Askama 0.11, Diesel 2.0 RC + SQLite, Tokio, Reqwest 0.11, feed-rs 1.x, HMAC-SHA256, vanilla JavaScript, CSS, GitHub Actions, Docker/OCI.

## Global Constraints

- Work only on local branch `work/boringbay-v2-local`; do not push, open a PR, merge or deploy before the user confirms the fully tested result.
- Preserve `/`, `/rank`, `/join-us`, `/api/badge/:domain`, `/api/icon/:domain`, `/api/favicon/:domain` and `/api/ws`.
- Preserve all existing members and historical `statistics` rows; all database migrations are additive.
- Preserve the red/pink palette, BoringFace SVG identity, light/dark mode, member cards, UV/RV/level display, real-time activity, rankings, status/removal purpose, join instructions and footer.
- `membership.json` remains authoritative; `tags` and `feed_url` are optional.
- No login, account, comments, follow graph, full-text search or hosted article bodies.
- Exploration, passport, feed and sharing events never modify UV or RV.
- No complete IP address may appear in logs, SQLite, WebSocket payloads or rendered HTML.
- Missing JavaScript, Feed failures, health-check failures and WebSocket failures must not prevent core pages and old APIs from working.
- Use TDD, run the focused test after every behavior change, and create local commits only.

---

## File Structure

### Existing files to modify

- `Cargo.toml`: runtime and test dependencies.
- `src/lib.rs`: module exports, shared configuration and application construction entry points.
- `src/main.rs`: configuration loading, migrations, task startup and graceful shutdown.
- `src/app_model.rs`: reduce to the runtime `Context`; delegate visitor, ranking and persistence behavior.
- `src/app_router.rs`: preserve old handlers and add V2 page/API handlers through focused modules.
- `src/membership_model.rs`: optional `tags` and `feed_url`, validation helpers.
- `src/statistics_model.rs`: existing persistence plus windowed aggregates.
- `src/schema.rs`: additive Diesel schema.
- `templates/base.html`: preserved navigation/footer, local assets, quieter live activity.
- `templates/index.html`: preserved member list plus discovery and feed sections.
- `templates/rank.html`: classic, activity and rising views plus status observation.
- `templates/join_us.html`: document optional tags/feed and old/new Badge behavior.
- `Dockerfile`: pinned base image and copied static assets.
- `.github/workflows/*.yml`: pinned actions, explicit permissions and test job.

### New focused files

- `src/config.rs`: `AppConfig` and environment parsing.
- `src/visitor.rs`: trusted-header parsing, HMAC deduplication and privacy-safe activity payload.
- `src/ranking.rs`: leaderboard calculations and status rules.
- `src/network_policy.rs`: public-HTTPS validation shared by Feed and health checks.
- `src/site_health.rs`: health persistence, probes and status evidence.
- `src/discovery.rs`: daily routes and fair random ordering.
- `src/feed.rs`: safe fetching, parsing, sanitizing and caching.
- `src/product_events.rs`: daily aggregate V2 event counters.
- `src/share.rs`: daily-route share page data and SVG card.
- `resources/static/app.css`: local visual layer.
- `resources/static/app.js`: WebSocket UI and shared helpers.
- `resources/static/passport.js`: local passport, favorites and achievements.
- `resources/static/discovery.js`: route/random interactions.
- `templates/route.html`: stable route/share page.
- `templates/components/*.html`: member card, ranking table, status table and feed card partials.
- `tests/compatibility.rs`: old route/API acceptance.
- `tests/common/mod.rs`: shared temporary SQLite setup and test router builder.
- `tests/pages.rs`: server-rendered page acceptance.
- `tests/migrations.rs`: old-to-new SQLite migration acceptance.
- `tests/fixtures/*.xml`: RSS/Atom and malicious parser fixtures.
- `migrations/20260916010000_create_site_health/{up.sql,down.sql}`: health evidence.
- `migrations/20260916020000_create_daily_routes/{up.sql,down.sql}`: persisted daily routes.
- `migrations/20260916030000_create_product_events/{up.sql,down.sql}`: aggregate V2 events.
- `migrations/20260916040000_create_feed_tables/{up.sql,down.sql}`: Feed sources and items.

---

### Task 1: Establish a Reproducible Baseline and Compatibility Harness

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Create: `src/config.rs`
- Create: `tests/compatibility.rs`

**Interfaces:**
- Produces: `AppConfig::from_env() -> anyhow::Result<AppConfig>`.
- Produces: `build_router(ctx: DynContext, config: Arc<AppConfig>) -> axum::Router`.
- Produces: `run_migrations(conn: &mut SqliteConnection) -> anyhow::Result<()>` for runtime and integration-test setup.
- Produces in `tests/common/mod.rs`: `temporary_app() -> (tempfile::TempDir, Router)`.

- [ ] **Step 1: Install and record a user-local Rust toolchain if `cargo` is absent**

Run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/boringbay-rustup-init.sh
sh /tmp/boringbay-rustup-init.sh -y --profile minimal --default-toolchain stable
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

Expected: stable `rustc` and `cargo` print versions; no repository file changes.

- [ ] **Step 2: Run the untouched baseline**

Run:

```bash
cargo test --locked
cargo build --locked
```

Expected: the existing project compiles and the current test set passes. A failure is a baseline blocker: record it verbatim and stop before changing application code.

- [ ] **Step 3: Add test dependencies and configuration types**

Add these development dependencies:

```toml
[dev-dependencies]
http-body = "0.4"
tempfile = "3"
tower = { version = "0.4", features = ["util"] }
```

Define:

```rust
#[derive(Clone, Debug)]
pub struct AppConfig {
    pub system_domain: String,
    pub database_url: String,
    pub v2_enabled: bool,
    pub trusted_proxy_mode: TrustedProxyMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustedProxyMode { Disabled, Cloudflare }
```

`BORINGBAY_V2_ENABLED` defaults to `false`; `TRUSTED_PROXY_MODE` defaults to `disabled` and accepts only `disabled` or `cloudflare`.

- [ ] **Step 4: Write failing old-route compatibility tests**

```rust
mod common;

#[tokio::test]
async fn old_pages_and_api_routes_remain_registered() {
    let (_tmp, app) = common::temporary_app().await;
    for uri in ["/", "/rank", "/join-us", "/api/icon/boringbay.com", "/api/favicon/boringbay.com"] {
        let response = app.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
        assert_ne!(response.status(), StatusCode::NOT_FOUND, "{uri}");
    }
}
```

- [ ] **Step 5: Extract router construction and make the test pass**

Move only router assembly from `main.rs` into `build_router`; `main` still owns signal handling and listener startup.

Run:

```bash
cargo test --test compatibility old_pages_and_api_routes_remain_registered -- --exact
cargo test --locked
```

Expected: PASS.

- [ ] **Step 6: Commit locally**

```bash
git add Cargo.toml Cargo.lock src/lib.rs src/main.rs src/config.rs tests/compatibility.rs
git commit -m "test: establish compatibility harness"
```

---

### Task 2: Remove IP Exposure and Make Visitor Parsing Safe

**Files:**
- Create: `src/visitor.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/lib.rs`
- Modify: `src/app_model.rs`
- Modify: `src/app_router.rs`
- Modify: `templates/base.html`
- Test: unit tests inside `src/visitor.rs`
- Test: `tests/compatibility.rs`

**Interfaces:**
- Produces: `VisitorIdentity::from_headers(headers: &HeaderMap, mode: TrustedProxyMode, hasher: &VisitorHasher) -> Option<VisitorIdentity>`.
- Produces: `VisitorHasher::random() -> VisitorHasher` and `VisitorHasher::from_key([u8; 32]) -> VisitorHasher`.
- Produces: `VistEvent { country: String, member: Membership, vt: Option<VisitorType> }` with no IP field.

- [ ] **Step 1: Add cryptographic dependencies**

```toml
hmac = "0.12"
rand = "0.8"
sha2 = "0.10"
hex = "0.4"
```

- [ ] **Step 2: Write failing visitor privacy tests**

```rust
#[test]
fn missing_headers_do_not_panic_or_count() {
    let headers = HeaderMap::new();
    assert!(VisitorIdentity::from_headers(&headers, TrustedProxyMode::Cloudflare, &fixed_hasher()).is_none());
}

#[test]
fn disabled_proxy_mode_ignores_forged_cloudflare_headers() {
    let headers = cloudflare_headers("203.0.113.9", "ES");
    assert!(VisitorIdentity::from_headers(&headers, TrustedProxyMode::Disabled, &fixed_hasher()).is_none());
}

#[test]
fn dedupe_key_never_contains_plain_ip() {
    let identity = VisitorIdentity::from_headers(&cloudflare_headers("203.0.113.9", "ES"), TrustedProxyMode::Cloudflare, &fixed_hasher()).unwrap();
    assert!(!identity.dedupe_key.contains("203.0.113.9"));
    assert_eq!(identity.country, "ES");
}
```

- [ ] **Step 3: Implement privacy-safe visitor identity**

Use `Hmac<Sha256>` over normalized IP bytes. Keep the HMAC only in the in-memory cache key; never log or persist it. Generate a random process key at startup. Return `None` for missing/invalid headers.

- [ ] **Step 4: Remove raw IP logs and WebSocket IP output**

Delete `info!("ip {}", ip)`. Update the activity payload and client copy to say `来自「ES」的访客` rather than displaying a masked address. Keep `vt`, country and member fields.

- [ ] **Step 5: Verify old endpoints degrade safely**

Add an integration assertion that `/api/badge/boringbay.com` without trusted headers returns a valid SVG but does not increment statistics.

Run:

```bash
cargo test visitor --lib
cargo test --test compatibility
rg -n 'CF-Connecting-IP|info!\("ip|data\.ip' src templates
```

Expected: tests PASS; remaining header references exist only in `visitor.rs`; no raw-IP log or client field remains.

- [ ] **Step 6: Commit locally**

```bash
git add Cargo.toml Cargo.lock src templates tests
git commit -m "fix: protect visitor privacy and header parsing"
```

---

### Task 3: Add Windowed Rankings and Correct Last-Activity Semantics

**Files:**
- Create: `src/ranking.rs`
- Modify: `src/lib.rs`
- Modify: `src/statistics_model.rs`
- Modify: `src/app_model.rs`
- Modify: `src/app_router.rs`
- Modify: `templates/index.html`
- Modify: `templates/rank.html`
- Create: `templates/components/ranking_table.html`

**Interfaces:**
- Produces: `RankingWindow { start: NaiveDateTime, end: NaiveDateTime }`.
- Produces: `RankingEntry { membership_id, uv, rv, last_activity, score, growth_rate }`.
- Produces: `RankingService::activity_30d(&self, now)`, `rising_7d(&self, now)`, and `classic(&self, now)`.
- Produces: `Statistics::last_activity(&self) -> NaiveDateTime` using the later of `updated_at` and `latest_referrer_at`.

- [ ] **Step 1: Write failing pure ranking tests**

```rust
#[test]
fn activity_rank_uses_uv_plus_rv_and_transparent_ties() {
    let entries = rank_activity(vec![sample(1, 5, 10), sample(2, 9, 6)]);
    assert_eq!(entries.iter().map(|e| e.membership_id).collect::<Vec<_>>(), vec![1, 2]);
}

#[test]
fn rising_rank_requires_five_recent_events() {
    assert!(growth_entry(1, 4, 0).is_none());
    assert_eq!(growth_entry(1, 10, 5).unwrap().growth_rate, 1.0);
}

#[test]
fn last_activity_considers_inbound_and_visited_timestamps() {
    let s = statistic_with_times(dt("2026-09-01"), dt("2026-09-12"));
    assert_eq!(s.last_activity(), dt("2026-09-12"));
}
```

- [ ] **Step 2: Implement pure ranking math**

Implement activity score `uv + rv`; tie-break by RV, UV, last activity. Implement growth rate `(current - previous) / max(previous, 5)` and require current total `>= 5`.

- [ ] **Step 3: Add Diesel window queries**

Reuse daily `statistics` rows and parameterized `created_at.between(start, end)`. Do not modify historical rows. Return aggregate values and the maximum of both activity timestamps.

- [ ] **Step 4: Render three transparent tabs**

Keep `/rank` defaulting to classic total. Add query `?view=activity` and `?view=rising`; homepage shows a 30-day activity summary. Every view displays its time window and formula.

- [ ] **Step 5: Run focused and regression tests**

```bash
cargo test ranking --lib
cargo test --test pages rankings
cargo test --locked
```

Expected: all PASS; classic ranking fixture order remains unchanged.

- [ ] **Step 6: Commit locally**

```bash
git add src templates tests
git commit -m "feat: add transparent ranking views"
```

---

### Task 4: Replace Automatic-Looking Removal with Evidence-Based Status Observation

**Files:**
- Create: `src/network_policy.rs`
- Create: `src/site_health.rs`
- Create: `migrations/20260916010000_create_site_health/up.sql`
- Create: `migrations/20260916010000_create_site_health/down.sql`
- Modify: `src/schema.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/app_router.rs`
- Modify: `templates/rank.html`
- Create: `templates/components/status_table.html`
- Create: `tests/migrations.rs`

**Interfaces:**
- Produces: `PublicHttpsUrl::parse(input: &str) -> Result<PublicHttpsUrl, NetworkPolicyError>`.
- Produces: `SiteHealthService::probe_member(&self, member: &Membership) -> SiteHealthResult`.
- Produces: `MemberStatus::classify(activity_age: Duration, consecutive_failures: u32, badge_missing_age: Option<Duration>) -> MemberStatus`.

- [ ] **Step 1: Add the shared HTTP client dependency**

```toml
reqwest = { version = "0.11", default-features = false, features = ["rustls-tls", "gzip"] }
```

- [ ] **Step 2: Create additive migration SQL**

Use exact tables:

```sql
CREATE TABLE site_health (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  member_id BIGINT NOT NULL,
  checked_at TIMESTAMP NOT NULL,
  reachable BOOLEAN NOT NULL,
  status_code INTEGER,
  badge_state TEXT NOT NULL
);
CREATE INDEX site_health_member_checked ON site_health(member_id, checked_at);
```

The down migration drops only the V2 table and index.

- [ ] **Step 3: Write failing status-boundary tests**

```rust
#[test]
fn quiet_site_is_not_a_removal_candidate() {
    assert_eq!(MemberStatus::classify(days(45), 0, None), MemberStatus::Quiet);
}

#[test]
fn observation_needs_sixty_days_and_three_failures() {
    assert_eq!(MemberStatus::classify(days(61), 2, None), MemberStatus::Quiet);
    assert_eq!(MemberStatus::classify(days(61), 3, None), MemberStatus::Observation);
}

#[test]
fn candidate_needs_ninety_days_and_strong_evidence() {
    assert_eq!(MemberStatus::classify(days(91), 6, None), MemberStatus::Observation);
    assert_eq!(MemberStatus::classify(days(91), 7, None), MemberStatus::RemovalCandidate);
}
```

- [ ] **Step 4: Implement public HTTPS policy**

Reject non-HTTPS, userinfo, localhost, IP literals in loopback/private/link-local/reserved ranges and DNS resolutions containing any forbidden address. This is the shared SSRF boundary. Health probes use a fixed User-Agent, 3-second connect timeout, 8-second total timeout and at most two manually validated redirects.

- [ ] **Step 5: Implement low-frequency health probes**

Probe the member homepage once daily. Treat 200–399 as reachable. Store only status code, timestamps and badge-state enum; do not store response bodies. Automated badge detection may record `unknown`; only explicit positive evidence or manual confirmation can set `missing`.

- [ ] **Step 6: Render status observation**

Replace strikethrough “即将移除” presentation with Active/Quiet/Observation/Removal Candidate labels, reason, last activity, last check and update link. Never execute deletion.

- [ ] **Step 7: Verify migration and failure isolation**

```bash
cargo test site_health --lib
cargo test --test migrations
cargo test --test pages status_observation
```

Expected: PASS; a probe timeout still renders `/` and `/rank`.

- [ ] **Step 8: Commit locally**

```bash
git add migrations src templates tests
git commit -m "feat: add evidence-based member status"
```

---

### Task 5: Modernize the Existing UI Without Replacing It

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `templates/base.html`
- Modify: `templates/index.html`
- Modify: `templates/rank.html`
- Modify: `templates/join_us.html`
- Create: `templates/components/member_card.html`
- Create: `resources/static/app.css`
- Create: `resources/static/app.js`
- Test: `tests/pages.rs`

**Interfaces:**
- Produces: `/static/*` via `tower_http::services::ServeDir`.
- Consumes existing page ViewModels without changing old URLs.

- [ ] **Step 1: Add local static-file support**

Add `tower-http = { version = "0.2", features = ["fs"] }` compatible with Axum 0.4. Nest `/static` and ensure missing assets return 404 without affecting app routes.

- [ ] **Step 2: Write failing page structure tests**

```rust
#[tokio::test]
async fn home_preserves_brand_member_metrics_and_join_paths() {
    let html = get_html("/").await;
    for marker in ["无聊湾", "今日无聊", "UV", "RV", "排行榜", "一起无聊？"] {
        assert!(html.contains(marker), "missing {marker}");
    }
}

#[tokio::test]
async fn pages_load_without_inline_remote_framework_dependency() {
    let html = get_html("/").await;
    assert!(html.contains("/static/app.css"));
    assert!(html.contains("/static/app.js"));
}
```

- [ ] **Step 3: Build the preserved visual system**

Define CSS custom properties for `#d0273e`, `#f5acb9`, surfaces, text and focus. Keep BoringFace, navbar, theme button, member metric labels and footer copy. Add responsive grids, visible focus, reduced-motion rules and a collapsible activity rail.

- [ ] **Step 4: Replace blocking alerts with the activity rail**

Use the existing WebSocket route. Keep at most 10 desktop or 3 mobile entries, expose pause/resume, use exponential reconnect delays `3s, 6s, 12s, 30s`, and do not show IP.

- [ ] **Step 5: Verify progressive enhancement**

Run:

```bash
cargo test --test pages
cargo test --test compatibility
```

Then render pages with JavaScript disabled in the local browser and verify member links, rankings, join instructions and theme-appropriate readable colors.

- [ ] **Step 6: Commit locally**

```bash
git add Cargo.toml Cargo.lock src templates resources tests
git commit -m "feat: modernize the preserved boringbay UI"
```

---

### Task 6: Add Daily Routes and Fair Random Discovery

**Files:**
- Create: `src/discovery.rs`
- Modify: `src/schema.rs`
- Create: `migrations/20260916020000_create_daily_routes/up.sql`
- Create: `migrations/20260916020000_create_daily_routes/down.sql`
- Modify: `src/app_router.rs`
- Modify: `src/membership_model.rs`
- Modify: `templates/index.html`
- Create: `templates/route.html`
- Create: `resources/static/discovery.js`
- Test: unit tests in `src/discovery.rs`
- Test: `tests/pages.rs`

**Interfaces:**
- Extends `Membership` with `tags: Option<Vec<String>>` using `#[serde(default)]`.
- Produces: `DiscoveryService::daily_route(date: NaiveDate, members: &[Membership]) -> Result<DailyRoute>`.
- Produces: `DiscoveryService::ordered_candidates(seed: u64, members: &[Membership], exposure: &HashMap<i64, i64>) -> Vec<i64>`.
- Produces pages `/route/:date` and JSON `/api/discovery/today`.

- [ ] **Step 1: Add optional member tags and a new migration**

Add `tags: Option<Vec<String>>` without changing any existing JSON row. Create:

```sql
CREATE TABLE daily_routes (
  route_date DATE PRIMARY KEY NOT NULL,
  member_ids TEXT NOT NULL,
  generated_at TIMESTAMP NOT NULL
);
```

Store member IDs as validated JSON because SQLite lacks an array type.

- [ ] **Step 2: Write failing deterministic discovery tests**

```rust
#[test]
fn daily_route_has_five_unique_eligible_members() {
    let route = generate(date(), fixture_members(), fixture_exposure()).unwrap();
    assert_eq!(route.member_ids.len(), 5);
    assert_eq!(route.member_ids.iter().collect::<HashSet<_>>().len(), 5);
    assert!(route.member_ids.iter().all(|id| eligible(*id)));
}

#[test]
fn same_date_and_state_produces_same_order() {
    assert_eq!(generate(date(), members(), exposure()), generate(date(), members(), exposure()));
}
```

- [ ] **Step 3: Implement persisted daily routes**

On first request, use a transaction to insert the generated route; on uniqueness conflict, load the winning row. Weight members inversely by recent route exposure, exclude hidden/observation/candidate members, and maximize distinct tags without making tags mandatory.

- [ ] **Step 4: Add discovery UI**

Add “随便逛逛 / 今日航线 / 最新漂流瓶” above the existing member list only when V2 is enabled. Random discovery executes from the server-provided eligible order and skips IDs listed in the local passport.

- [ ] **Step 5: Test feature flag and fallback**

```bash
BORINGBAY_V2_ENABLED=false cargo test --test pages v1_home_has_no_v2_controls
BORINGBAY_V2_ENABLED=true cargo test --test pages route_page_has_five_member_links
cargo test discovery --lib
```

Expected: PASS.

- [ ] **Step 6: Commit locally**

```bash
git add migrations src templates resources tests
git commit -m "feat: add fair daily blog discovery"
```

---

### Task 7: Add the Local Passport, Favorites and Achievements

**Files:**
- Create: `resources/static/passport.js`
- Modify: `resources/static/discovery.js`
- Modify: `resources/static/app.css`
- Modify: `templates/index.html`
- Modify: `templates/route.html`
- Create: `tests/passport.test.mjs`

**Interfaces:**
- Produces browser module `BoringBayPassport` with `load()`, `visit(memberId, tags)`, `toggleFavorite(memberId)`, `completeRoute(date)`, `achievements()` and `reset()`.
- Persists only under localStorage key `boringbay.passport.v1`.

- [ ] **Step 1: Write failing Node DOM-independent tests**

```javascript
test('visit is idempotent and unlocks first voyage', () => {
  const store = memoryStorage();
  const passport = createPassport(store, fixedClock('2026-09-16T12:00:00+08:00'));
  passport.visit(12, ['tech']);
  passport.visit(12, ['tech']);
  assert.deepEqual(passport.state().visitedMemberIds, [12]);
  assert.ok(passport.achievements().includes('first-voyage'));
});
```

- [ ] **Step 2: Implement a versioned local schema**

Use:

```javascript
{
  version: 1,
  visitedMemberIds: [],
  favorites: [],
  routeCompletions: {},
  explorationDays: [],
  unlockedAchievements: []
}
```

Invalid JSON or unknown versions reset safely without throwing.

- [ ] **Step 3: Implement exact initial achievements**

- `first-voyage`: one unique member visited.
- `five-islands`: five unique members visited.
- `night-owl`: a visit between local 00:00 and 04:59.
- `hidden-gem`: visit a server-marked low-exposure member.
- `three-day-streak`: visits on three consecutive Shanghai dates.

- [ ] **Step 4: Connect cards and route stamps**

Member links call `visit` before navigation. Cards receive an accessible visited badge and favorite toggle. Route page shows 0–5 stamps and a reset control with confirmation.

- [ ] **Step 5: Test and manually verify storage failure**

```bash
node --test tests/passport.test.mjs
cargo test --test pages passport_assets
```

Also simulate localStorage throwing; controls hide and member links still work.

- [ ] **Step 6: Commit locally**

```bash
git add resources templates tests
git commit -m "feat: add a local no-login passport"
```

---

### Task 8: Add Aggregate Product Events Without User Profiles

**Files:**
- Create: `src/product_events.rs`
- Modify: `src/schema.rs`
- Create: `migrations/20260916030000_create_product_events/up.sql`
- Create: `migrations/20260916030000_create_product_events/down.sql`
- Modify: `src/app_router.rs`
- Modify: `resources/static/discovery.js`
- Modify: `resources/static/passport.js`

**Interfaces:**
- Produces: `ProductEventKind` allowlist.
- Produces: `ProductEventService::increment(day, kind, member_id) -> Result<()>`.
- Produces: `POST /api/events` accepting only `{ kind, member_id? }`.

- [ ] **Step 1: Extend migration**

```sql
CREATE TABLE product_events (
  event_date DATE NOT NULL,
  event_kind TEXT NOT NULL,
  member_id BIGINT NOT NULL DEFAULT 0,
  count BIGINT NOT NULL DEFAULT 0,
  UNIQUE(event_date, event_kind, member_id)
);
```

- [ ] **Step 2: Write failing allowlist tests**

```rust
#[test]
fn unknown_event_is_rejected() {
    assert!(ProductEventKind::try_from("arbitrary-user-text").is_err());
}

#[test]
fn event_payload_has_no_identity_fields() {
    let json = serde_json::to_value(EventInput { kind: RouteStart, member_id: None }).unwrap();
    assert_eq!(json.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["kind", "member_id"]);
}
```

- [ ] **Step 3: Implement atomic daily increments**

Use SQLite `INSERT ... ON CONFLICT ... DO UPDATE SET count = count + 1`. Map a missing member ID to sentinel `0`; real member IDs are positive. Allow only route start/complete, random use, member outbound, feed outbound, share click/open, join view/edit and local-return boolean.

- [ ] **Step 4: Ensure telemetry is optional**

Client requests use `keepalive`, ignore failures and never block navigation. The server does not read or persist IP, user agent, referrer, localStorage contents or a stable identifier for these events.

- [ ] **Step 5: Run tests and commit locally**

```bash
cargo test product_events --lib
cargo test --test compatibility
git add migrations src resources
git commit -m "feat: add privacy-safe aggregate product events"
```

---

### Task 9: Implement Opt-In RSS/Atom Fetching and Drift Bottles

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/membership_model.rs`
- Create: `src/feed.rs`
- Modify: `src/schema.rs`
- Create: `migrations/20260916040000_create_feed_tables/up.sql`
- Create: `migrations/20260916040000_create_feed_tables/down.sql`
- Modify: `src/main.rs`
- Modify: `src/app_router.rs`
- Modify: `templates/index.html`
- Create: `templates/components/feed_card.html`
- Create: `tests/fixtures/rss.xml`
- Create: `tests/fixtures/atom.xml`
- Create: `tests/fixtures/xxe.xml`

**Interfaces:**
- Extends `Membership` with `feed_url: Option<String>` using `#[serde(default)]`; `tags` already exists from Task 6.
- Produces: `FeedFetcher::refresh_member(&self, member: &Membership) -> Result<FeedRefresh>`.
- Produces: `FeedRepository::latest(limit: usize) -> Result<Vec<FeedItem>>`.

- [ ] **Step 1: Add feed dependencies**

```toml
feed-rs = "1"
ammonia = "3"
```

- [ ] **Step 2: Extend migration**

Create `feed_sources` keyed by member ID and `feed_items` keyed by `(member_id, item_key)`. Store title, URL, plain summary, published time and fetched time; never store full body HTML.

- [ ] **Step 3: Write failing fixture tests**

```rust
#[test]
fn parses_rss_and_atom_into_same_model() {
    for fixture in ["rss.xml", "atom.xml"] {
        let items = parse_fixture(fixture).unwrap();
        assert_eq!(items[0].title, "First post");
        assert_eq!(items[0].summary, "Safe summary");
    }
}

#[test]
fn rejects_external_entities_and_strips_script_content() {
    assert!(parse_fixture("xxe.xml").is_err());
    assert!(!sanitize("<script>x()</script><p>Safe</p>").contains("script"));
}
```

- [ ] **Step 4: Implement guarded HTTP fetching**

Reuse `PublicHttpsUrl`; disable automatic redirects, manually allow at most two validated redirects, cap decompressed bytes at 1 MiB, use 3-second connect and 8-second total timeout, and accept only XML-like response content.

- [ ] **Step 5: Implement refresh and staleness behavior**

Refresh opt-in sources no more than every 30 minutes. On failure, retain cache and record error kind. Hide a source from “最新漂流瓶” after seven days without success. Feed state never changes membership status.

- [ ] **Step 6: Render drift bottles**

Show at most 12 items with title, source, date and a 160-character plain-text summary. The main link goes directly to the original HTTPS URL and records only an aggregate feed-outbound event.

- [ ] **Step 7: Test and commit locally**

```bash
cargo test feed --lib
cargo test --test pages drift_bottles
cargo test --locked
git add Cargo.toml Cargo.lock migrations src templates tests
git commit -m "feat: add safe opt-in blog feeds"
```

---

### Task 10: Add Stable Share Pages and Branded Route Cards

**Files:**
- Create: `src/share.rs`
- Modify: `src/lib.rs`
- Modify: `src/app_router.rs`
- Modify: `templates/route.html`
- Modify: `templates/join_us.html`
- Modify: `resources/static/discovery.js`
- Test: unit tests in `src/share.rs`
- Test: `tests/pages.rs`

**Interfaces:**
- Produces: `GET /route/:date` with Open Graph metadata.
- Produces: `GET /api/share/route/:date.svg` returning branded SVG.
- Produces: optional `GET /api/badge-v2/:domain` without changing `/api/badge/:domain`.
- Produces: client download card from the same route data; no user text accepted.

- [ ] **Step 1: Write failing route validation tests**

```rust
#[test]
fn share_svg_escapes_all_member_text() {
    let svg = render_route_svg(&route_with_name("<script>alert(1)</script>")).unwrap();
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("&lt;script&gt;"));
}

#[tokio::test]
async fn invalid_route_date_is_not_found() {
    assert_eq!(get("/route/not-a-date").await.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Implement stable metadata**

Set canonical URL, title, description and `og:image` to the SVG endpoint. Use only persisted route data and server-controlled copy. Add `Cache-Control` for completed dates.

- [ ] **Step 3: Implement browser completion card**

Generate a downloadable image or SVG containing date, five station names and stamp count. Treat stamp count as decorative and never submit it to ranking or rewards.

- [ ] **Step 4: Add an optional V2 Badge**

Add `/api/badge-v2/:domain` with the existing BoringFace identity plus a small route/exploration treatment. Keep `/api/badge/:domain`, `/api/icon/:domain` and `/api/favicon/:domain` byte-for-byte compatible for the same metrics. Document both Badge forms in `join_us.html`.

- [ ] **Step 5: Verify injection and fallback**

```bash
cargo test share --lib
cargo test --test pages share_metadata
cargo test --test compatibility old_badge_contract_is_unchanged
```

Disable SVG rendering in a test fixture and verify the route HTML still returns 200 with a default brand image.

- [ ] **Step 6: Commit locally**

```bash
git add src templates resources tests
git commit -m "feat: add shareable daily routes"
```

---

### Task 11: Harden CI, Container Builds and Runtime Operations

**Files:**
- Modify: `.github/workflows/contributors.yml`
- Modify: `.github/workflows/cross-compile.yml`
- Modify: `.github/workflows/docker-build-push.yml`
- Modify: `Dockerfile`
- Modify: `README.md`
- Create: `.env.example`

**Interfaces:**
- Produces a test-before-build CI job.
- Documents `BORINGBAY_V2_ENABLED`, `TRUSTED_PROXY_MODE`, `DATABASE_URL`, `SYSTEM_DOMAIN`, Feed and health intervals.

- [ ] **Step 1: Pin third-party actions and permissions**

Use these reviewed commits, resolved from the existing major tags on 2026-09-16:

```text
actions/checkout@11d5960a326750d5838078e36cf38b85af677262
actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830
actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02
actions-rs/toolchain@63eb9591781c46a70274cb3ebdf190fce92702e8
actions-rs/cargo@e7f754b8e09f70ad8eb2c5aebf61e58e8403b210
docker/login-action@c94ce9fb468520275223c153574b00df6fe4bcc9
docker/setup-qemu-action@c7c53464625b32c7a7e944ae62b3e17d2b600130
docker/setup-buildx-action@8d2750c68a42422c14e847fe6c8ac0403b4cbd6f
docker/build-push-action@10e90e3645eae34f1e60eeb005ba3a3d33f178e8
dawidd6/action-download-artifact@ac66b43f0e6a346234dd65d4d0c8fbb31cb316e5
jaywcjlove/github-action-contributors@86707f6d4c2469ce6b46bc3367253ebd41ee242c
jaywcjlove/github-action-modify-file-content@0e3b8d492a44de3769f2f24af63a5dca7eb39559
```

Set workflow-level `permissions: { contents: read }`; grant `packages: write` only to the image-publish job and `contents: write` only to the contributor updater.

- [ ] **Step 2: Add CI quality gates**

Run:

```yaml
- run: cargo fmt --all -- --check
- run: cargo test --locked
- run: cargo clippy --all-targets --locked -- -D warnings
```

Cross compilation and Docker publishing depend on the test job.

- [ ] **Step 3: Pin the container base**

Resolve the current linux/amd64 and linux/arm64 manifest-list digest with:

```bash
docker buildx imagetools inspect ubuntu:24.04
```

Replace `ubuntu:latest` with `ubuntu:24.04@sha256:224a1869083a311ef3f13648a154ba79832fbef6364d31493642ca03082da254`. Re-run the same command immediately before the edit and require the Dockerfile digest to equal this reviewed 2026-09-16 value; if the registry value changed, stop for review instead of silently substituting it. Keep `# Ubuntu 24.04 LTS` above the `FROM` line.

- [ ] **Step 4: Document runtime and rollback**

Document SQLite backup, feature-flag rollback, trusted proxy requirement, Feed opt-in fields and health/status rules. Do not include secrets or production credential values.

- [ ] **Step 5: Validate workflow syntax and local container build**

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
docker build --platform linux/amd64 -t boringbay:v2-local .
```

Expected: all commands succeed.

- [ ] **Step 6: Commit locally**

```bash
git add .github Dockerfile README.md .env.example
git commit -m "ci: harden boringbay build and release"
```

---

### Task 12: Full Local Acceptance, Migration Rehearsal and Handoff

**Files:**
- Modify only files required to fix acceptance defects.
- Create: `docs/testing/2026-09-16-boringbay-v2-local-acceptance.md`

**Interfaces:**
- Produces a reproducible test report with command, result, environment and residual risks.
- Produces a local build/image for user confirmation; no push or deployment.

- [ ] **Step 1: Run the complete automated suite**

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
node --test tests/passport.test.mjs
```

Expected: all PASS with zero warnings promoted by Clippy.

- [ ] **Step 2: Rehearse migration against a copy of V1 data**

Create a temporary database containing the current migrations and representative statistics, record row counts and aggregates, start the V2 binary to run migrations, then verify:

```text
statistics row count unchanged
SUM(unique_visitor) unchanged
SUM(referrer) unchanged
all V2 tables present
V1 pages render after migration
```

- [ ] **Step 3: Run local HTTP acceptance with V2 off and on**

Start two local runs sequentially:

```bash
BORINGBAY_V2_ENABLED=false TRUSTED_PROXY_MODE=disabled DATABASE_URL=/tmp/boringbay-v1.db SYSTEM_DOMAIN=localhost:3000 cargo run --release
BORINGBAY_V2_ENABLED=true TRUSTED_PROXY_MODE=cloudflare DATABASE_URL=/tmp/boringbay-v2.db SYSTEM_DOMAIN=localhost:3000 cargo run --release
```

Probe old routes, new routes, static assets, invalid dates, missing headers and simulated Feed/health failures. Record response status and content type.

- [ ] **Step 4: Perform browser acceptance**

Verify desktop and mobile widths, light/dark mode, keyboard-only navigation, reduced motion, JS disabled fallback, WebSocket reconnect, route completion, local passport reset, favorites, stale Feed display, all three rankings and all four member statuses.

- [ ] **Step 5: Build and smoke-test the container**

```bash
docker build --platform linux/amd64 -t boringbay:v2-local .
docker run --rm -p 3300:3000 \
  -e BORINGBAY_V2_ENABLED=true \
  -e TRUSTED_PROXY_MODE=disabled \
  -e DATABASE_URL=/webapp/data/naive.db \
  -e SYSTEM_DOMAIN=localhost:3300 \
  boringbay:v2-local
```

Probe `http://127.0.0.1:3300/`, `/rank`, `/join-us`, old SVG endpoints and `/route/2026-09-16`.

- [ ] **Step 6: Review the diff and secret scan**

```bash
git diff main...HEAD --check
git status --short
git log --oneline main..HEAD
rg -n '(ghp_|github_pat_|BEGIN .*PRIVATE KEY|password\s*=|token\s*=)' . --glob '!Cargo.lock' --glob '!.git/**'
```

Expected: clean diff, only intended local commits, no credential material.

- [ ] **Step 7: Write and commit the local acceptance report**

The report must list exact command versions, pass/fail totals, migration invariants, manual browser checks, container digest, known limitations and the statement `No branch, commit, PR or image was pushed or deployed after implementation began.`

```bash
git add docs/testing/2026-09-16-boringbay-v2-local-acceptance.md
git commit -m "docs: record boringbay v2 local acceptance"
```

- [ ] **Step 8: Stop and request user confirmation**

Provide the local branch, final commit, test summary, preview instructions and residual risks. Do not push, open a PR, merge or deploy. Wait for the user to review and explicitly authorize the next external action.
