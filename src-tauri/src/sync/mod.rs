pub mod backends;
pub mod conflict;
pub mod engine;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFile {
    pub id: String,
    pub rel_path: String,
    pub etag: Option<String>,
    pub modified_ms: i64,
    pub size: Option<u64>,
    pub trashed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoteContent {
    pub bytes: Vec<u8>,
    pub etag: Option<String>,
    pub modified_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ChangeBatch {
    pub changes: Vec<RemoteFile>,
    pub next_page_token: Option<String>,
    pub new_start_page_token: Option<String>,
}

#[async_trait]
#[allow(dead_code)]
pub trait SyncBackend: Send + Sync {
    fn id(&self) -> &'static str;
    async fn list(&self, cursor: Option<&str>) -> Result<Page<RemoteFile>, String>;
    async fn download(&self, remote_id: &str) -> Result<RemoteContent, String>;
    async fn upload(
        &self,
        rel_path: &str,
        body: &[u8],
        prev_remote_id: Option<&str>,
    ) -> Result<RemoteFile, String>;
    async fn delete(&self, remote_id: &str) -> Result<(), String>;
    async fn poll_changes(
        &self,
        page_token: Option<&str>,
    ) -> Result<ChangeBatch, String>;
    async fn start_page_token(&self) -> Result<String, String>;
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
