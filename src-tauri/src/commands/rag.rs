use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, State};

use crate::commands::sync::DbState;
use crate::rag::{self, embed, index, search};

#[derive(Serialize)]
pub struct RagStatus {
    pub enabled: bool,
    pub files_indexed: i64,
    pub chunks: i64,
    pub last_indexed_ms: Option<i64>,
    pub model: String,
    pub ollama_url: String,
}

#[tauri::command]
pub async fn rag_status(
    db: State<'_, DbState>,
    workspace: Option<String>,
) -> Result<RagStatus, String> {
    let enabled = rag::is_enabled();
    let model = rag::DEFAULT_MODEL.to_string();
    let ollama_url = embed::base_url();

    let (files_indexed, chunks, last_indexed_ms) = if let Some(ws) = workspace {
        let (f,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM rag_files WHERE workspace = ?1")
                .bind(&ws)
                .fetch_one(&db.pool)
                .await
                .map_err(|e| e.to_string())?;
        let (c,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM rag_chunks WHERE workspace = ?1")
                .bind(&ws)
                .fetch_one(&db.pool)
                .await
                .map_err(|e| e.to_string())?;
        let last: Option<(i64,)> = sqlx::query_as(
            "SELECT MAX(indexed_ms) FROM rag_files WHERE workspace = ?1",
        )
        .bind(&ws)
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?;
        (f, c, last.map(|(t,)| t))
    } else {
        (0, 0, None)
    };

    Ok(RagStatus {
        enabled,
        files_indexed,
        chunks,
        last_indexed_ms,
        model,
        ollama_url,
    })
}

#[tauri::command]
pub fn rag_set_enabled(enabled: bool) -> Result<(), String> {
    rag::set_enabled(enabled)
}

#[tauri::command]
pub async fn rag_reindex(
    app: AppHandle,
    db: State<'_, DbState>,
    workspace: String,
) -> Result<index::IndexReport, String> {
    if !rag::is_enabled() {
        return Err("RAG is disabled".into());
    }
    let root = PathBuf::from(&workspace);
    if !root.is_dir() {
        return Err(format!("not a directory: {workspace}"));
    }
    Ok(index::reindex(app, db.pool.clone(), root).await)
}

#[tauri::command]
pub async fn rag_search(
    db: State<'_, DbState>,
    workspace: String,
    query: String,
    k: Option<usize>,
) -> Result<Vec<search::SearchHit>, String> {
    if !rag::is_enabled() {
        return Ok(vec![]);
    }
    search::search(&db.pool, &workspace, &query, k.unwrap_or(5)).await
}

#[tauri::command]
pub async fn rag_clear(
    db: State<'_, DbState>,
    workspace: String,
) -> Result<(), String> {
    index::clear_workspace(&db.pool, &workspace).await
}

#[tauri::command]
pub async fn rag_reindex_file(
    db: State<'_, DbState>,
    workspace: String,
    rel_path: String,
) -> Result<(), String> {
    if !rag::is_enabled() {
        return Ok(());
    }
    let mut abs = PathBuf::from(&workspace);
    abs.push(&rel_path);
    if !abs.exists() {
        index::remove_file(&db.pool, &workspace, &rel_path).await?;
        return Ok(());
    }
    index::index_file(&db.pool, &workspace, &rel_path, &abs)
        .await
        .map(|_| ())
}
