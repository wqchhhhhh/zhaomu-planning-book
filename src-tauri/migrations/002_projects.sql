ALTER TABLE tasks ADD COLUMN is_important INTEGER NOT NULL DEFAULT 0;
ALTER TABLE tasks ADD COLUMN project_id TEXT REFERENCES projects(id);

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','paused','done')),
  start_date TEXT,
  end_date TEXT,
  color TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tasks_project ON tasks(project_id) WHERE deleted_at IS NULL;
INSERT OR IGNORE INTO schema_migrations(version,name) VALUES(2,'projects_and_importance');
