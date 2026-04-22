CREATE TABLE IF NOT EXISTS rag_chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace TEXT NOT NULL,
    rel_path TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    embedding BLOB NOT NULL,
    model TEXT NOT NULL,
    dim INTEGER NOT NULL,
    indexed_ms INTEGER NOT NULL,
    UNIQUE(workspace, rel_path, chunk_index)
);

CREATE INDEX IF NOT EXISTS rag_chunks_workspace_idx ON rag_chunks(workspace);
CREATE INDEX IF NOT EXISTS rag_chunks_rel_path_idx ON rag_chunks(workspace, rel_path);

CREATE TABLE IF NOT EXISTS rag_files (
    workspace TEXT NOT NULL,
    rel_path TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    indexed_ms INTEGER NOT NULL,
    PRIMARY KEY (workspace, rel_path)
);
