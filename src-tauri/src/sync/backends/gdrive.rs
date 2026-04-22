use async_trait::async_trait;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::keychain;
use crate::sync::{ChangeBatch, Page, RemoteContent, RemoteFile, SyncBackend};

const CLIENT_ID_KEY: &str = "gdrive:client_id";
const REFRESH_TOKEN_KEY: &str = "gdrive:refresh";
const ROOT_FOLDER_KEY: &str = "gdrive:root_folder";

#[derive(Clone)]
pub struct GoogleDrive {
    client_id: String,
    tokens: Arc<RwLock<Option<TokenSet>>>,
    root_folder_id: Arc<RwLock<Option<String>>>,
    http: reqwest::Client,
}

#[derive(Clone, Debug)]
struct TokenSet {
    access: String,
    expires_at_ms: i64,
    refresh: Option<String>,
}

impl GoogleDrive {
    pub fn try_load() -> Result<Option<Self>, String> {
        let Some(client_id) = keychain::get_secret(CLIENT_ID_KEY)? else {
            return Ok(None);
        };
        let refresh = keychain::get_secret(REFRESH_TOKEN_KEY)?;
        let root = keychain::get_secret(ROOT_FOLDER_KEY)?;
        let tokens = refresh.map(|r| TokenSet {
            access: String::new(),
            expires_at_ms: 0,
            refresh: Some(r),
        });
        Ok(Some(Self {
            client_id,
            tokens: Arc::new(RwLock::new(tokens)),
            root_folder_id: Arc::new(RwLock::new(root)),
            http: reqwest::Client::new(),
        }))
    }

    pub fn save_client_id(client_id: &str) -> Result<(), String> {
        keychain::set_secret(CLIENT_ID_KEY, client_id)
    }

    pub fn forget() -> Result<(), String> {
        keychain::delete_secret(REFRESH_TOKEN_KEY)?;
        keychain::delete_secret(ROOT_FOLDER_KEY)?;
        Ok(())
    }

    pub async fn sign_in(&self, on_open_url: impl FnOnce(&str)) -> Result<(), String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let redirect_uri = format!("http://127.0.0.1:{port}/callback");

        let client = oauth_client(&self.client_id, &redirect_uri)?;
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (auth_url, csrf) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("https://www.googleapis.com/auth/drive.file".into()))
            .add_extra_param("access_type", "offline")
            .add_extra_param("prompt", "consent")
            .set_pkce_challenge(pkce_challenge)
            .url();

        on_open_url(auth_url.as_str());

        let (code, returned_state) = listen_for_code(listener).await?;
        if returned_state != *csrf.secret() {
            return Err("OAuth state mismatch".into());
        }

        let token = client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.secret().clone()))
            .request_async(async_http_client)
            .await
            .map_err(|e| e.to_string())?;

        let refresh_secret = token
            .refresh_token()
            .ok_or_else(|| "Google did not return a refresh token (consent may have been remembered)".to_string())?
            .secret()
            .clone();
        keychain::set_secret(REFRESH_TOKEN_KEY, &refresh_secret)?;

        let expires_at_ms = chrono::Utc::now().timestamp_millis()
            + token
                .expires_in()
                .map(|d| d.as_millis() as i64)
                .unwrap_or(3_500_000);
        *self.tokens.write().await = Some(TokenSet {
            access: token.access_token().secret().clone(),
            expires_at_ms,
            refresh: Some(refresh_secret),
        });

        self.ensure_root_folder().await?;
        Ok(())
    }

    async fn access_token(&self) -> Result<String, String> {
        let now = chrono::Utc::now().timestamp_millis();
        {
            let guard = self.tokens.read().await;
            if let Some(t) = guard.as_ref() {
                if !t.access.is_empty() && t.expires_at_ms - 60_000 > now {
                    return Ok(t.access.clone());
                }
            }
        }
        let refresh = {
            let guard = self.tokens.read().await;
            guard
                .as_ref()
                .and_then(|t| t.refresh.clone())
                .ok_or_else(|| "not signed in".to_string())?
        };
        let client = oauth_client(&self.client_id, "http://127.0.0.1/")?;
        let token = client
            .exchange_refresh_token(&RefreshToken::new(refresh.clone()))
            .request_async(async_http_client)
            .await
            .map_err(|e| e.to_string())?;
        let expires_at_ms = chrono::Utc::now().timestamp_millis()
            + token
                .expires_in()
                .map(|d| d.as_millis() as i64)
                .unwrap_or(3_500_000);
        let access = token.access_token().secret().clone();
        *self.tokens.write().await = Some(TokenSet {
            access: access.clone(),
            expires_at_ms,
            refresh: Some(refresh),
        });
        Ok(access)
    }

    async fn ensure_root_folder(&self) -> Result<String, String> {
        {
            let guard = self.root_folder_id.read().await;
            if let Some(id) = guard.as_ref() {
                return Ok(id.clone());
            }
        }
        let name = "jsmde";
        let token = self.access_token().await?;
        let query = format!(
            "name='{name}' and mimeType='application/vnd.google-apps.folder' and trashed=false"
        );
        let resp = self
            .http
            .get("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(&token)
            .query(&[("q", query.as_str()), ("fields", "files(id,name)")])
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let v: Value = resp.json().await.map_err(|e| e.to_string())?;
        if let Some(files) = v.get("files").and_then(|f| f.as_array()) {
            if let Some(first) = files.first() {
                if let Some(id) = first.get("id").and_then(|x| x.as_str()) {
                    let id = id.to_string();
                    *self.root_folder_id.write().await = Some(id.clone());
                    keychain::set_secret(ROOT_FOLDER_KEY, &id)?;
                    return Ok(id);
                }
            }
        }
        let created = self
            .http
            .post("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(&token)
            .json(&json!({
                "name": name,
                "mimeType": "application/vnd.google-apps.folder"
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let created_v: Value = created.json().await.map_err(|e| e.to_string())?;
        let id = created_v
            .get("id")
            .and_then(|x| x.as_str())
            .ok_or_else(|| "missing folder id".to_string())?
            .to_string();
        *self.root_folder_id.write().await = Some(id.clone());
        keychain::set_secret(ROOT_FOLDER_KEY, &id)?;
        Ok(id)
    }
}

fn oauth_client(client_id: &str, redirect_uri: &str) -> Result<BasicClient, String> {
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".into())
        .map_err(|e| e.to_string())?;
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".into())
        .map_err(|e| e.to_string())?;
    Ok(BasicClient::new(
        ClientId::new(client_id.to_string()),
        None,
        auth_url,
        Some(token_url),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_uri.to_string()).map_err(|e| e.to_string())?))
}

async fn listen_for_code(listener: TcpListener) -> Result<(String, String), String> {
    let server = tiny_http::Server::from_listener(listener, None).map_err(|e| e.to_string())?;
    loop {
        let req = match server.recv_timeout(std::time::Duration::from_secs(300)) {
            Ok(Some(r)) => r,
            Ok(None) => return Err("OAuth timed out".into()),
            Err(e) => return Err(e.to_string()),
        };
        let url_str = format!("http://127.0.0.1{}", req.url());
        if let Ok(parsed) = url::Url::parse(&url_str) {
            let mut code = None;
            let mut state = None;
            for (k, v) in parsed.query_pairs() {
                match k.as_ref() {
                    "code" => code = Some(v.into_owned()),
                    "state" => state = Some(v.into_owned()),
                    _ => {}
                }
            }
            let body = "<html><body style='font-family:system-ui;padding:40px;text-align:center'><h2>jsmde</h2><p>You can close this tab and return to the app.</p></body></html>";
            let resp = tiny_http::Response::from_string(body)
                .with_header("Content-Type: text/html".parse::<tiny_http::Header>().unwrap());
            let _ = req.respond(resp);
            match (code, state) {
                (Some(c), Some(s)) => return Ok((c, s)),
                _ => continue,
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct DriveFile {
    id: String,
    name: String,
    #[serde(rename = "modifiedTime")]
    modified_time: Option<String>,
    size: Option<String>,
    #[serde(default)]
    trashed: bool,
    #[serde(rename = "md5Checksum")]
    _md5: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListResp {
    files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChangesResp {
    changes: Vec<ChangeItem>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "newStartPageToken")]
    new_start_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChangeItem {
    #[serde(rename = "fileId")]
    file_id: Option<String>,
    #[serde(default)]
    removed: bool,
    file: Option<DriveFile>,
}

fn ms_from_rfc3339(s: &Option<String>) -> i64 {
    s.as_ref()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .map(|d| d.timestamp_millis())
        .unwrap_or(0)
}

fn to_remote_file(f: DriveFile) -> RemoteFile {
    RemoteFile {
        id: f.id,
        rel_path: f.name,
        etag: None,
        modified_ms: ms_from_rfc3339(&f.modified_time),
        size: f.size.as_ref().and_then(|s| s.parse::<u64>().ok()),
        trashed: f.trashed,
    }
}

#[async_trait]
impl SyncBackend for GoogleDrive {
    fn id(&self) -> &'static str {
        "gdrive"
    }

    async fn list(&self, cursor: Option<&str>) -> Result<Page<RemoteFile>, String> {
        let token = self.access_token().await?;
        let folder_id = self.ensure_root_folder().await?;
        let q = format!("'{folder_id}' in parents and trashed=false");
        let mut query: Vec<(&str, String)> = vec![
            ("q", q),
            (
                "fields",
                "nextPageToken,files(id,name,modifiedTime,size,trashed)".into(),
            ),
            ("pageSize", "100".into()),
        ];
        if let Some(c) = cursor {
            query.push(("pageToken", c.to_string()));
        }
        let resp: ListResp = self
            .http
            .get("https://www.googleapis.com/drive/v3/files")
            .bearer_auth(&token)
            .query(&query)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        Ok(Page {
            items: resp.files.into_iter().map(to_remote_file).collect(),
            next_page_token: resp.next_page_token,
        })
    }

    async fn download(&self, remote_id: &str) -> Result<RemoteContent, String> {
        let token = self.access_token().await?;
        let url = format!("https://www.googleapis.com/drive/v3/files/{remote_id}?alt=media");
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();

        let meta: Value = self
            .http
            .get(&format!(
                "https://www.googleapis.com/drive/v3/files/{remote_id}?fields=modifiedTime"
            ))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        let modified_ms = meta
            .get("modifiedTime")
            .and_then(|v| v.as_str())
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|d| d.timestamp_millis())
            .unwrap_or(0);
        Ok(RemoteContent {
            bytes,
            etag: None,
            modified_ms,
        })
    }

    async fn upload(
        &self,
        rel_path: &str,
        body: &[u8],
        prev_remote_id: Option<&str>,
    ) -> Result<RemoteFile, String> {
        let token = self.access_token().await?;
        if let Some(id) = prev_remote_id {
            let url =
                format!("https://www.googleapis.com/upload/drive/v3/files/{id}?uploadType=media&fields=id,name,modifiedTime,size,trashed");
            let resp: DriveFile = self
                .http
                .patch(&url)
                .bearer_auth(&token)
                .header("Content-Type", "text/markdown")
                .body(body.to_vec())
                .send()
                .await
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?
                .json()
                .await
                .map_err(|e| e.to_string())?;
            return Ok(to_remote_file(resp));
        }

        let folder_id = self.ensure_root_folder().await?;
        let metadata = json!({
            "name": rel_path,
            "parents": [folder_id],
            "mimeType": "text/markdown"
        });
        let meta_str = metadata.to_string();

        let boundary = format!("mde{}", chrono::Utc::now().timestamp_millis());
        let mut payload = Vec::new();
        payload.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        payload.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
        payload.extend_from_slice(meta_str.as_bytes());
        payload.extend_from_slice(b"\r\n");
        payload.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        payload.extend_from_slice(b"Content-Type: text/markdown\r\n\r\n");
        payload.extend_from_slice(body);
        payload.extend_from_slice(b"\r\n");
        payload.extend_from_slice(format!("--{boundary}--").as_bytes());

        let resp: DriveFile = self
            .http
            .post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id,name,modifiedTime,size,trashed")
            .bearer_auth(&token)
            .header(
                "Content-Type",
                format!("multipart/related; boundary={boundary}"),
            )
            .body(payload)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        Ok(to_remote_file(resp))
    }

    async fn delete(&self, remote_id: &str) -> Result<(), String> {
        let token = self.access_token().await?;
        let url = format!("https://www.googleapis.com/drive/v3/files/{remote_id}");
        self.http
            .patch(&url)
            .bearer_auth(&token)
            .json(&json!({ "trashed": true }))
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn poll_changes(
        &self,
        page_token: Option<&str>,
    ) -> Result<ChangeBatch, String> {
        let Some(token) = page_token else {
            return Ok(ChangeBatch {
                changes: vec![],
                next_page_token: None,
                new_start_page_token: None,
            });
        };
        let access = self.access_token().await?;
        let folder_id = self.ensure_root_folder().await?;
        let resp: ChangesResp = self
            .http
            .get("https://www.googleapis.com/drive/v3/changes")
            .bearer_auth(&access)
            .query(&[
                ("pageToken", token),
                (
                    "fields",
                    "nextPageToken,newStartPageToken,changes(removed,fileId,file(id,name,modifiedTime,size,trashed,parents))",
                ),
                ("pageSize", "100"),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        let mut changes: Vec<RemoteFile> = Vec::new();
        for c in resp.changes {
            if let Some(file) = c.file {
                let rf = to_remote_file(file);
                changes.push(rf);
            } else if c.removed {
                if let Some(id) = c.file_id {
                    changes.push(RemoteFile {
                        id,
                        rel_path: String::new(),
                        etag: None,
                        modified_ms: 0,
                        size: None,
                        trashed: true,
                    });
                }
            }
        }
        let _ = folder_id;
        Ok(ChangeBatch {
            changes,
            next_page_token: resp.next_page_token,
            new_start_page_token: resp.new_start_page_token,
        })
    }

    async fn start_page_token(&self) -> Result<String, String> {
        let token = self.access_token().await?;
        let v: Value = self
            .http
            .get("https://www.googleapis.com/drive/v3/changes/startPageToken")
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;
        v.get("startPageToken")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "missing startPageToken".to_string())
    }
}
