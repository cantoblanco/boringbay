CREATE TABLE feed_sources (
  member_id BIGINT PRIMARY KEY NOT NULL,
  feed_url TEXT NOT NULL,
  last_attempt TIMESTAMP,
  last_success TIMESTAMP,
  last_error TEXT
);

CREATE TABLE feed_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  member_id BIGINT NOT NULL,
  item_key TEXT NOT NULL,
  title TEXT NOT NULL,
  url TEXT NOT NULL,
  summary TEXT NOT NULL,
  published_at TIMESTAMP NOT NULL,
  fetched_at TIMESTAMP NOT NULL,
  UNIQUE(member_id, item_key)
);

CREATE INDEX feed_items_published ON feed_items(published_at DESC);
