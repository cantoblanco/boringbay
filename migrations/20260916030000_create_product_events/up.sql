CREATE TABLE product_events (
  event_date DATE NOT NULL,
  event_kind TEXT NOT NULL,
  member_id BIGINT NOT NULL DEFAULT 0,
  count BIGINT NOT NULL DEFAULT 0,
  UNIQUE(event_date, event_kind, member_id)
);
