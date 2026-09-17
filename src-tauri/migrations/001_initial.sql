CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS plans (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL CHECK (kind IN ('study','research','life','project')),
  start_date TEXT,
  end_date TEXT,
  color TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS stages (
  id TEXT PRIMARY KEY,
  plan_id TEXT NOT NULL REFERENCES plans(id),
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS documents (
  id TEXT PRIMARY KEY,
  parent_id TEXT REFERENCES documents(id),
  kind TEXT NOT NULL CHECK (kind IN ('folder','note','daily')),
  title TEXT NOT NULL,
  relative_path TEXT NOT NULL UNIQUE,
  sort_order INTEGER NOT NULL DEFAULT 0,
  content_hash TEXT,
  file_size INTEGER NOT NULL DEFAULT 0,
  file_modified_at TEXT,
  sync_state TEXT NOT NULL DEFAULT 'synced' CHECK (sync_state IN ('synced','needs_reindex','missing','conflict')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'todo' CHECK (status IN ('todo','doing','done')),
  priority TEXT NOT NULL DEFAULT 'medium' CHECK (priority IN ('low','medium','high','urgent')),
  planned_date TEXT,
  due_at TEXT,
  estimated_minutes INTEGER CHECK (estimated_minutes IS NULL OR estimated_minutes >= 0),
  actual_minutes INTEGER NOT NULL DEFAULT 0 CHECK (actual_minutes >= 0),
  plan_id TEXT REFERENCES plans(id),
  stage_id TEXT REFERENCES stages(id),
  document_id TEXT REFERENCES documents(id),
  sort_order INTEGER NOT NULL DEFAULT 0,
  completed_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL COLLATE NOCASE UNIQUE,
  color TEXT,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_tags (
  task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (task_id, tag_id)
);

CREATE TABLE IF NOT EXISTS document_tags (
  document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (document_id, tag_id)
);

CREATE TABLE IF NOT EXISTS study_sessions (
  id TEXT PRIMARY KEY,
  task_id TEXT REFERENCES tasks(id),
  plan_id TEXT REFERENCES plans(id),
  stage_id TEXT REFERENCES stages(id),
  document_id TEXT REFERENCES documents(id),
  started_at TEXT NOT NULL,
  ended_at TEXT,
  duration_minutes INTEGER NOT NULL CHECK (duration_minutes >= 0),
  content TEXT NOT NULL DEFAULT '',
  note TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS checkins (
  id TEXT PRIMARY KEY,
  checkin_date TEXT NOT NULL UNIQUE,
  mood INTEGER CHECK (mood BETWEEN 1 AND 5),
  summary_markdown TEXT NOT NULL DEFAULT '',
  completed_tasks INTEGER NOT NULL DEFAULT 0,
  total_tasks INTEGER NOT NULL DEFAULT 0,
  study_minutes INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS file_operations (
  id TEXT PRIMARY KEY,
  operation TEXT NOT NULL CHECK (operation IN ('write','move','delete','restore')),
  source_path TEXT,
  target_path TEXT,
  entity_id TEXT,
  state TEXT NOT NULL CHECK (state IN ('pending','files_done','completed','failed')),
  error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tasks_planned_date ON tasks(planned_date) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_plan_stage ON tasks(plan_id, stage_id) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_documents_parent ON documents(parent_id, sort_order) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON study_sessions(started_at) WHERE deleted_at IS NULL;

CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
  entity_type UNINDEXED,
  entity_id UNINDEXED,
  title,
  body,
  tags,
  tokenize = 'unicode61'
);

INSERT OR IGNORE INTO schema_migrations(version, name) VALUES (1, 'initial');
