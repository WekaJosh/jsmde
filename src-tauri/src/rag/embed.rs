use serde::Deserialize;
use serde_json::json;

pub fn base_url() -> String {
    crate::keychain::get_secret("ai:ollama")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string())
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

pub async fn embed(model: &str, text: &str) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/embeddings", base_url());
    let res = client
        .post(&url)
        .json(&json!({
            "model": model,
            "prompt": text,
        }))
        .send()
        .await
        .map_err(|e| format!("embed request failed: {e}"))?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("embed http {status}: {body}"));
    }
    let parsed: EmbedResponse = res
        .json()
        .await
        .map_err(|e| format!("embed parse failed: {e}"))?;
    if parsed.embedding.is_empty() {
        return Err("embed returned empty vector".into());
    }
    Ok(parsed.embedding)
}

pub fn vec_to_blob(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

pub fn blob_to_vec(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}
