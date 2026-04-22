use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

use super::{chunk, embed, DEFAULT_MODEL};

const MD_EXTS: &[&str] = &["md", "markdown", "mdx"];

#[derive(serde::Serialize, Clone)]
pub struct IndexProgress {
    pub phase: String,
    pub done: usize,
    pub total: usize,
    pub current: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct IndexReport {
    pub files_scanned: usize,
    pub files_indexed: usize,
    pub chunks_written: usize,
    pub error: Option<String>,
}

pub async fn reindex(app: AppHandle, pool: SqlitePool, workspace: PathBuf) -> IndexReport {
    let mut report = IndexReport {
        files_scanned: 0,
        files_indexed: 0,
        chunks_written: 0,
        error: None,
    };

    let ws = workspace.to_string_lossy().into_owned();
    let _ = app.emit(
        "rag://progress",
        IndexProgress {
            phase: "scanning".into(),
            done: 0,
            total: 0,
            current: None,
        },
    );

    let files = match collect_md_files(&workspace) {
        Ok(f) => f,
        Err(e) => {
            report.error = Some(e);
            return report;
        }
    };
    report.files_scanned = files.len();

    if let Err(e) = prune_missing(&pool, &ws, &files).await {
        log::warn!("rag prune failed: {e}");
    }

    let total = files.len();
    for (i, abs_path) in files.iter().enumerate() {
        let rel = match abs_path.strip_prefix(&workspace) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        let _ = app.emit(
            "rag://progress",
            IndexProgress {
                phase: "indexing".into(),
                done: i,
                total,
                current: Some(rel.clone()),
            },
        );

        match index_file(&pool, &ws, &rel, abs_path).await {
            Ok(Some(n)) => {
                report.files_indexed += 1;
                report.chunks_written += n;
            }
            Ok(None) => {}
            Err(e) => {
                log::warn!("rag index {rel}: {e}");
                if report.error.is_none() {
                    report.error = Some(e);
                }
            }
        }
    }

    let _ = app.emit(
        "rag://progress",
        IndexProgress {
            phase: "done".into(),
            done: total,
            total,
            current: None,
        },
    );

    report
}

pub async fn index_file(
    pool: &SqlitePool,
    workspace: &str,
    rel_path: &str,
    abs_path: &Path,
) -> Result<Option<usize>, String> {
    let content = tokio::fs::read_to_string(abs_path)
        .await
        .map_err(|e| e.to_string())?;
    let hash = hash_str(&content);

    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT content_hash FROM rag_files WHERE workspace = ?1 AND rel_path = ?2",
    )
    .bind(workspace)
    .bind(rel_path)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    if existing.map(|(h,)| h) == Some(hash.clone()) {
        return Ok(None);
    }

    let chunks = chunk::chunk_markdown(&content);
    if chunks.is_empty() {
        sqlx::query("DELETE FROM rag_chunks WHERE workspace = ?1 AND rel_path = ?2")
            .bind(workspace)
            .bind(rel_path)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        upsert_file(pool, workspace, rel_path, &hash).await?;
        return Ok(Some(0));
    }

    let mut embeddings = Vec::with_capacity(chunks.len());
    for c in &chunks {
        let v = embed::embed(DEFAULT_MODEL, &c.content).await?;
        embeddings.push(v);
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM rag_chunks WHERE workspace = ?1 AND rel_path = ?2")
        .bind(workspace)
        .bind(rel_path)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    let now_ms = now_ms();
    for (c, v) in chunks.iter().zip(embeddings.iter()) {
        sqlx::query(
            "INSERT INTO rag_chunks (workspace, rel_path, chunk_index, content, content_hash, embedding, model, dim, indexed_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(workspace)
        .bind(rel_path)
        .bind(c.index as i64)
        .bind(&c.content)
        .bind(&hash)
        .bind(embed::vec_to_blob(v))
        .bind(DEFAULT_MODEL)
        .bind(v.len() as i64)
        .bind(now_ms)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    sqlx::query(
        "INSERT INTO rag_files (workspace, rel_path, content_hash, indexed_ms) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(workspace, rel_path) DO UPDATE SET content_hash = excluded.content_hash, indexed_ms = excluded.indexed_ms",
    )
    .bind(workspace)
    .bind(rel_path)
    .bind(&hash)
    .bind(now_ms)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(Some(chunks.len()))
}

async fn upsert_file(
    pool: &SqlitePool,
    workspace: &str,
    rel_path: &str,
    hash: &str,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO rag_files (workspace, rel_path, content_hash, indexed_ms) VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(workspace, rel_path) DO UPDATE SET content_hash = excluded.content_hash, indexed_ms = excluded.indexed_ms",
    )
    .bind(workspace)
    .bind(rel_path)
    .bind(hash)
    .bind(now_ms())
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn remove_file(
    pool: &SqlitePool,
    workspace: &str,
    rel_path: &str,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM rag_chunks WHERE workspace = ?1 AND rel_path = ?2")
        .bind(workspace)
        .bind(rel_path)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM rag_files WHERE workspace = ?1 AND rel_path = ?2")
        .bind(workspace)
        .bind(rel_path)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn clear_workspace(pool: &SqlitePool, workspace: &str) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM rag_chunks WHERE workspace = ?1")
        .bind(workspace)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM rag_files WHERE workspace = ?1")
        .bind(workspace)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn prune_missing(
    pool: &SqlitePool,
    workspace: &str,
    current_abs: &[PathBuf],
) -> Result<(), String> {
    let base = PathBuf::from(workspace);
    let keep: std::collections::HashSet<String> = current_abs
        .iter()
        .filter_map(|p| p.strip_prefix(&base).ok())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();

    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT rel_path FROM rag_files WHERE workspace = ?1")
            .bind(workspace)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())?;

    for (rel,) in rows {
        if !keep.contains(&rel) {
            remove_file(pool, workspace, &rel).await?;
        }
    }
    Ok(())
}

fn collect_md_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    walk(root, &mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.') {
            continue;
        }
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk(&path, out)?;
        } else if ft.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if MD_EXTS.contains(&ext.to_ascii_lowercase().as_str()) {
                    out.push(path);
                }
            }
        }
    }
    Ok(())
}

fn hash_str(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
