use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter};

use super::conflict::write_sidecar;
use super::{hash_bytes, SyncBackend};

const MD_EXTS: &[&str] = &["md", "markdown", "mdx"];

#[derive(Debug, Clone, Serialize)]
pub struct SyncReport {
    pub uploaded: u32,
    pub downloaded: u32,
    pub conflicts: u32,
    pub deleted_remote: u32,
    pub errors: Vec<String>,
}

pub async fn sync_once(
    app: &AppHandle,
    pool: &SqlitePool,
    workspace_root: &Path,
    backend: Arc<dyn SyncBackend>,
) -> Result<SyncReport, String> {
    let _ = app.emit("sync://start", ());
    let mut report = SyncReport {
        uploaded: 0,
        downloaded: 0,
        conflicts: 0,
        deleted_remote: 0,
        errors: Vec::new(),
    };

    let locals = gather_local(workspace_root).await?;
    let mut remotes = HashMap::new();
    let mut cursor: Option<String> = None;
    loop {
        let page = backend.list(cursor.as_deref()).await?;
        for f in page.items {
            if f.trashed {
                continue;
            }
            remotes.insert(f.rel_path.clone(), f);
        }
        cursor = page.next_page_token;
        if cursor.is_none() {
            break;
        }
    }

    let workspace_key = workspace_root.to_string_lossy().to_string();

    let mut keys: Vec<String> = locals.keys().cloned().collect();
    for k in remotes.keys() {
        if !locals.contains_key(k) {
            keys.push(k.clone());
        }
    }

    for rel_path in keys {
        let local = locals.get(&rel_path);
        let remote = remotes.get(&rel_path);
        let row: Option<FileRow> = sqlx::query_as::<_, FileRow>(
            "SELECT rel_path, workspace, mtime_ms, size, content_hash, backend, remote_id, remote_etag, remote_modified_ms, last_synced_ms, state
             FROM files WHERE rel_path = ?1 AND workspace = ?2"
        )
        .bind(&rel_path)
        .bind(&workspace_key)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        match (local, remote) {
            (Some(l), None) => {
                match backend.upload(&rel_path, &l.bytes, None).await {
                    Ok(uploaded) => {
                        upsert_row(pool, &workspace_key, &rel_path, l, Some(&uploaded)).await?;
                        report.uploaded += 1;
                    }
                    Err(e) => report.errors.push(format!("upload {rel_path}: {e}")),
                }
            }
            (None, Some(r)) => {
                match backend.download(&r.id).await {
                    Ok(content) => {
                        let target = workspace_root.join(&rel_path);
                        if let Some(parent) = target.parent() {
                            let _ = tokio::fs::create_dir_all(parent).await;
                        }
                        if let Err(e) = tokio::fs::write(&target, &content.bytes).await {
                            report.errors.push(format!("write {rel_path}: {e}"));
                            continue;
                        }
                        let hash = hash_bytes(&content.bytes);
                        let now = Utc::now().timestamp_millis();
                        let local_mtime = file_mtime_ms(&target).await.unwrap_or(now);
                        let size = content.bytes.len() as i64;
                        sqlx::query(
                            "INSERT INTO files(rel_path, workspace, mtime_ms, size, content_hash, backend, remote_id, remote_etag, remote_modified_ms, last_synced_ms, state)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'clean')
                             ON CONFLICT(rel_path) DO UPDATE SET
                               workspace=excluded.workspace,
                               mtime_ms=excluded.mtime_ms,
                               size=excluded.size,
                               content_hash=excluded.content_hash,
                               backend=excluded.backend,
                               remote_id=excluded.remote_id,
                               remote_etag=excluded.remote_etag,
                               remote_modified_ms=excluded.remote_modified_ms,
                               last_synced_ms=excluded.last_synced_ms,
                               state='clean'"
                        )
                        .bind(&rel_path)
                        .bind(&workspace_key)
                        .bind(local_mtime)
                        .bind(size)
                        .bind(&hash)
                        .bind(backend.id())
                        .bind(&r.id)
                        .bind::<Option<String>>(None)
                        .bind(r.modified_ms)
                        .bind(now)
                        .execute(pool)
                        .await
                        .map_err(|e| e.to_string())?;
                        report.downloaded += 1;
                    }
                    Err(e) => report.errors.push(format!("download {rel_path}: {e}")),
                }
            }
            (Some(l), Some(r)) => {
                let local_hash = &l.hash;
                let (prev_hash, prev_synced) = match &row {
                    Some(r) => (Some(r.content_hash.clone()), r.last_synced_ms.unwrap_or(0)),
                    None => (None, 0),
                };
                let local_changed = prev_hash.as_ref().map(|h| h != local_hash).unwrap_or(true);
                let remote_changed = r.modified_ms > prev_synced;

                if !local_changed && !remote_changed {
                    continue;
                }

                if local_changed && remote_changed {
                    match backend.download(&r.id).await {
                        Ok(content) => {
                            let remote_hash = hash_bytes(&content.bytes);
                            if remote_hash == *local_hash {
                                upsert_row(pool, &workspace_key, &rel_path, l, Some(r)).await?;
                                continue;
                            }
                            let target = workspace_root.join(&rel_path);
                            let sidecar = write_sidecar(&target, &content.bytes).await?;
                            let now = Utc::now().timestamp_millis();
                            sqlx::query(
                                "INSERT INTO conflicts(rel_path, workspace, local_hash, remote_hash, sidecar_path, detected_ms)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                                 ON CONFLICT(rel_path) DO UPDATE SET
                                   local_hash=excluded.local_hash,
                                   remote_hash=excluded.remote_hash,
                                   sidecar_path=excluded.sidecar_path,
                                   detected_ms=excluded.detected_ms"
                            )
                            .bind(&rel_path)
                            .bind(&workspace_key)
                            .bind(local_hash)
                            .bind(&remote_hash)
                            .bind(sidecar.to_string_lossy().to_string())
                            .bind(now)
                            .execute(pool)
                            .await
                            .map_err(|e| e.to_string())?;
                            match backend.upload(&rel_path, &l.bytes, Some(&r.id)).await {
                                Ok(up) => {
                                    upsert_row(pool, &workspace_key, &rel_path, l, Some(&up)).await?;
                                    report.uploaded += 1;
                                }
                                Err(e) => report.errors.push(format!("upload after conflict {rel_path}: {e}")),
                            }
                            report.conflicts += 1;
                        }
                        Err(e) => report.errors.push(format!("conflict download {rel_path}: {e}")),
                    }
                } else if local_changed {
                    match backend.upload(&rel_path, &l.bytes, Some(&r.id)).await {
                        Ok(up) => {
                            upsert_row(pool, &workspace_key, &rel_path, l, Some(&up)).await?;
                            report.uploaded += 1;
                        }
                        Err(e) => report.errors.push(format!("upload {rel_path}: {e}")),
                    }
                } else if remote_changed {
                    match backend.download(&r.id).await {
                        Ok(content) => {
                            let target = workspace_root.join(&rel_path);
                            if let Some(parent) = target.parent() {
                                let _ = tokio::fs::create_dir_all(parent).await;
                            }
                            if let Err(e) = tokio::fs::write(&target, &content.bytes).await {
                                report.errors.push(format!("write {rel_path}: {e}"));
                                continue;
                            }
                            let updated = LocalFile {
                                bytes: content.bytes.clone(),
                                hash: hash_bytes(&content.bytes),
                                mtime_ms: file_mtime_ms(&target)
                                    .await
                                    .unwrap_or_else(|| Utc::now().timestamp_millis()),
                                size: content.bytes.len() as i64,
                            };
                            upsert_row(pool, &workspace_key, &rel_path, &updated, Some(r)).await?;
                            report.downloaded += 1;
                        }
                        Err(e) => report.errors.push(format!("download {rel_path}: {e}")),
                    }
                }
            }
            (None, None) => {}
        }
    }

    let _ = app.emit("sync://done", report.clone());
    Ok(report)
}

async fn upsert_row(
    pool: &SqlitePool,
    workspace_key: &str,
    rel_path: &str,
    l: &LocalFile,
    remote: Option<&super::RemoteFile>,
) -> Result<(), String> {
    let now = Utc::now().timestamp_millis();
    sqlx::query(
        "INSERT INTO files(rel_path, workspace, mtime_ms, size, content_hash, backend, remote_id, remote_etag, remote_modified_ms, last_synced_ms, state)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'clean')
         ON CONFLICT(rel_path) DO UPDATE SET
           workspace=excluded.workspace,
           mtime_ms=excluded.mtime_ms,
           size=excluded.size,
           content_hash=excluded.content_hash,
           backend=excluded.backend,
           remote_id=excluded.remote_id,
           remote_etag=excluded.remote_etag,
           remote_modified_ms=excluded.remote_modified_ms,
           last_synced_ms=excluded.last_synced_ms,
           state='clean'"
    )
    .bind(rel_path)
    .bind(workspace_key)
    .bind(l.mtime_ms)
    .bind(l.size)
    .bind(&l.hash)
    .bind(remote.map(|_| "gdrive"))
    .bind(remote.map(|r| r.id.clone()))
    .bind::<Option<String>>(None)
    .bind(remote.map(|r| r.modified_ms))
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

struct LocalFile {
    bytes: Vec<u8>,
    hash: String,
    mtime_ms: i64,
    size: i64,
}

async fn gather_local(root: &Path) -> Result<HashMap<String, LocalFile>, String> {
    let mut out = HashMap::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut rd = tokio::fs::read_dir(&dir).await.map_err(|e| e.to_string())?;
        while let Some(entry) = rd.next_entry().await.map_err(|e| e.to_string())? {
            let p = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') {
                continue;
            }
            if name_str.contains(".conflict-") {
                continue;
            }
            let ft = entry.file_type().await.map_err(|e| e.to_string())?;
            if ft.is_dir() {
                stack.push(p);
                continue;
            }
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if !MD_EXTS.contains(&ext.as_str()) {
                continue;
            }
            let bytes = tokio::fs::read(&p).await.map_err(|e| e.to_string())?;
            let hash = hash_bytes(&bytes);
            let rel = p
                .strip_prefix(root)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| name_str.to_string());
            let meta = entry.metadata().await.map_err(|e| e.to_string())?;
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(Utc::now().timestamp_millis());
            let size = bytes.len() as i64;
            out.insert(
                rel,
                LocalFile {
                    bytes,
                    hash,
                    mtime_ms,
                    size,
                },
            );
        }
    }
    Ok(out)
}

async fn file_mtime_ms(p: &Path) -> Option<i64> {
    let meta = tokio::fs::metadata(p).await.ok()?;
    let t = meta.modified().ok()?;
    let d = t.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(d.as_millis() as i64)
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct FileRow {
    rel_path: String,
    workspace: String,
    mtime_ms: i64,
    size: i64,
    content_hash: String,
    backend: Option<String>,
    remote_id: Option<String>,
    remote_etag: Option<String>,
    remote_modified_ms: Option<i64>,
    last_synced_ms: Option<i64>,
    state: String,
}
