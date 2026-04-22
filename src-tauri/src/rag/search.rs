use serde::Serialize;
use sqlx::SqlitePool;

use super::{embed, DEFAULT_MODEL};

#[derive(Serialize)]
pub struct SearchHit {
    pub rel_path: String,
    pub chunk_index: i64,
    pub content: String,
    pub score: f32,
}

pub async fn search(
    pool: &SqlitePool,
    workspace: &str,
    query: &str,
    k: usize,
) -> Result<Vec<SearchHit>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let q = embed::embed(DEFAULT_MODEL, query).await?;
    let q_norm = l2_norm(&q);
    if q_norm == 0.0 {
        return Ok(vec![]);
    }

    let rows: Vec<(String, i64, String, Vec<u8>)> = sqlx::query_as(
        "SELECT rel_path, chunk_index, content, embedding FROM rag_chunks WHERE workspace = ?1",
    )
    .bind(workspace)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut scored: Vec<SearchHit> = rows
        .into_iter()
        .filter_map(|(rel_path, chunk_index, content, blob)| {
            let v = embed::blob_to_vec(&blob);
            if v.len() != q.len() {
                return None;
            }
            let score = cosine(&q, &v, q_norm);
            Some(SearchHit {
                rel_path,
                chunk_index,
                content,
                score,
            })
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    Ok(scored)
}

fn l2_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

fn cosine(q: &[f32], d: &[f32], q_norm: f32) -> f32 {
    let mut dot = 0.0f32;
    let mut dn = 0.0f32;
    for (a, b) in q.iter().zip(d.iter()) {
        dot += a * b;
        dn += b * b;
    }
    let dn = dn.sqrt();
    if dn == 0.0 {
        0.0
    } else {
        dot / (q_norm * dn)
    }
}
