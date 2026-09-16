CREATE TABLE traffic_hourly (
  bucket_start TIMESTAMP NOT NULL,
  member_id BIGINT NOT NULL,
  event_kind TEXT NOT NULL,
  dimension_kind TEXT NOT NULL,
  dimension_value TEXT NOT NULL DEFAULT '',
  count BIGINT NOT NULL DEFAULT 0,
  PRIMARY KEY (bucket_start, member_id, event_kind, dimension_kind, dimension_value)
);

CREATE INDEX idx_traffic_hourly_range
  ON traffic_hourly (member_id, bucket_start, event_kind, dimension_kind);

CREATE TABLE traffic_daily (
  bucket_date DATE NOT NULL,
  member_id BIGINT NOT NULL,
  event_kind TEXT NOT NULL,
  dimension_kind TEXT NOT NULL,
  dimension_value TEXT NOT NULL DEFAULT '',
  count BIGINT NOT NULL DEFAULT 0,
  PRIMARY KEY (bucket_date, member_id, event_kind, dimension_kind, dimension_value)
);

CREATE INDEX idx_traffic_daily_range
  ON traffic_daily (member_id, bucket_date, event_kind, dimension_kind);

INSERT INTO traffic_daily
  (bucket_date, member_id, event_kind, dimension_kind, dimension_value, count)
SELECT date(created_at), membership_id, 'badge_view', 'total', '', unique_visitor
FROM statistics
WHERE unique_visitor > 0
ON CONFLICT(bucket_date, member_id, event_kind, dimension_kind, dimension_value)
DO UPDATE SET count = excluded.count;

INSERT INTO traffic_daily
  (bucket_date, member_id, event_kind, dimension_kind, dimension_value, count)
SELECT date(created_at), membership_id, 'inbound_referral', 'total', '', referrer
FROM statistics
WHERE referrer > 0
ON CONFLICT(bucket_date, member_id, event_kind, dimension_kind, dimension_value)
DO UPDATE SET count = excluded.count;

INSERT INTO traffic_daily
  (bucket_date, member_id, event_kind, dimension_kind, dimension_value, count)
SELECT event_date, member_id, 'outbound_click', 'total', '', SUM(count)
FROM product_events
WHERE member_id > 0 AND event_kind IN ('member_outbound', 'feed_outbound')
GROUP BY event_date, member_id
ON CONFLICT(bucket_date, member_id, event_kind, dimension_kind, dimension_value)
DO UPDATE SET count = excluded.count;
