CREATE TABLE site_health (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  member_id BIGINT NOT NULL,
  checked_at TIMESTAMP NOT NULL,
  reachable BOOLEAN NOT NULL,
  status_code INTEGER,
  badge_state TEXT NOT NULL
);

CREATE INDEX site_health_member_checked ON site_health(member_id, checked_at);
