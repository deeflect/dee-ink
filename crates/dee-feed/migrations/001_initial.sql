CREATE TABLE items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  feed_id INTEGER NOT NULL,
  ext_id TEXT NOT NULL,
  title TEXT NOT NULL,
  url TEXT NOT NULL DEFAULT '',
  summary TEXT NOT NULL DEFAULT '',
  published TEXT NOT NULL,
  read INTEGER NOT NULL DEFAULT 0,
  UNIQUE(feed_id, ext_id)
);

CREATE INDEX idx_items_feed_id ON items(feed_id);
CREATE INDEX idx_items_published ON items(published);

CREATE TABLE feeds_cache (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  url TEXT NOT NULL
);
