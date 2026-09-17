ALTER TABLE tasks ADD COLUMN schedule_start TEXT;
ALTER TABLE tasks ADD COLUMN schedule_end TEXT;
ALTER TABLE tasks ADD COLUMN recurrence TEXT NOT NULL DEFAULT 'none';
ALTER TABLE tasks ADD COLUMN remind_on_open INTEGER NOT NULL DEFAULT 1;

CREATE TABLE task_daily_items (
  id TEXT PRIMARY KEY, task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  item_date TEXT NOT NULL, content TEXT NOT NULL, is_done INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_task_daily_date ON task_daily_items(item_date);

CREATE TABLE knowledge_categories (
  id TEXT PRIMARY KEY, parent_id TEXT REFERENCES knowledge_categories(id) ON DELETE CASCADE,
  title TEXT NOT NULL, sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  deleted_at TEXT
);
CREATE TABLE knowledge_files (
  id TEXT PRIMARY KEY, category_id TEXT NOT NULL REFERENCES knowledge_categories(id) ON DELETE CASCADE,
  title TEXT NOT NULL, file_name TEXT NOT NULL, relative_path TEXT NOT NULL UNIQUE,
  file_size INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')), deleted_at TEXT
);
CREATE INDEX idx_knowledge_files_category ON knowledge_files(category_id);
INSERT INTO schema_migrations(version, name) VALUES(3, 'task schedules and uploaded knowledge');
