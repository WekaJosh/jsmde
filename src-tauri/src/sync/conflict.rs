use std::path::{Path, PathBuf};

pub fn sidecar_path(original: &Path) -> PathBuf {
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let stem = original.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = original.extension().and_then(|s| s.to_str()).unwrap_or("md");
    let name = format!("{stem}.conflict-{ts}.{ext}");
    original
        .parent()
        .map(|p| p.join(&name))
        .unwrap_or_else(|| PathBuf::from(&name))
}

pub async fn write_sidecar(original: &Path, body: &[u8]) -> Result<PathBuf, String> {
    let p = sidecar_path(original);
    tokio::fs::write(&p, body).await.map_err(|e| e.to_string())?;
    Ok(p)
}
