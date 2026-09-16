CREATE TABLE daily_routes (
  route_date DATE PRIMARY KEY NOT NULL,
  member_ids TEXT NOT NULL,
  generated_at TIMESTAMP NOT NULL
);
