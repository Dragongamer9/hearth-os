CREATE TABLE IF NOT EXISTS pings (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  version TEXT,
  first INTEGER NOT NULL,
  last INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS pings_last ON pings (last);

-- Bug-report logs people send from Settings › System (kept 14 days, read with the code shown on their screen).
CREATE TABLE IF NOT EXISTS logs (
  code TEXT PRIMARY KEY,
  created INTEGER NOT NULL,
  version TEXT,
  kind TEXT,
  body TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS logs_created ON logs (created);

-- Upvotes for the experimental home layouts: one per device and layout.
CREATE TABLE IF NOT EXISTS votes (
  device TEXT NOT NULL,
  layout TEXT NOT NULL,
  at INTEGER NOT NULL,
  PRIMARY KEY (device, layout)
);
