CREATE TABLE IF NOT EXISTS files (
    rel_path TEXT PRIMARY KEY,
    workspace TEXT NOT NULL,
    mtime_ms INTEGER NOT NULL,
    size INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    backend TEXT,
    remote_id TEXT,
    remote_etag TEXT,
    remote_modified_ms INTEGER,
    last_synced_ms INTEGER,
    state TEXT NOT NULL DEFAULT 'clean'
);

CREATE INDEX IF NOT EXISTS files_workspace_idx ON files(workspace);

CREATE TABLE IF NOT EXISTS sync_cursors (
    backend TEXT PRIMARY KEY,
    page_token TEXT,
    last_poll_ms INTEGER
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    backend TEXT NOT NULL,
    display_name TEXT,
    remote_root TEXT,
    created_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS conflicts (
    rel_path TEXT PRIMARY KEY,
    workspace TEXT NOT NULL,
    local_hash TEXT NOT NULL,
    remote_hash TEXT NOT NULL,
    sidecar_path TEXT NOT NULL,
    detected_ms INTEGER NOT NULL
);
