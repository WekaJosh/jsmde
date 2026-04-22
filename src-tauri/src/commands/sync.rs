use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use sqlx::SqlitePool;
use tauri::{AppHandle, State};

use crate::sync::backends::gdrive::GoogleDrive;
use crate::sync::engine::{sync_once, SyncReport};

pub struct DbState {
    pub pool: SqlitePool,
}

#[derive(Debug, Serialize, Clone)]
pub struct GDriveStatus {
    pub has_client_id: bool,
    pub signed_in: bool,
    pub root_folder_id: Option<String>,
}

#[tauri::command]
pub async fn gdrive_status() -> Result<GDriveStatus, String> {
    let has_client_id = crate::keychain::get_secret("gdrive:client_id")?.is_some();
    let has_refresh = crate::keychain::get_secret("gdrive:refresh")?.is_some();
    let root_folder_id = crate::keychain::get_secret("gdrive:root_folder")?;
    Ok(GDriveStatus {
        has_client_id,
        signed_in: has_refresh,
        root_folder_id,
    })
}

#[tauri::command]
pub fn gdrive_save_client_id(client_id: String) -> Result<(), String> {
    GoogleDrive::save_client_id(&client_id)
}

#[tauri::command]
pub async fn gdrive_sign_in(app: AppHandle) -> Result<(), String> {
    let drive = GoogleDrive::try_load()?
        .ok_or_else(|| "No Google client ID configured. Add one in Settings → Google Drive.".to_string())?;
    let handle = app.clone();
    drive
        .sign_in(move |url| {
            use tauri_plugin_opener::OpenerExt;
            let _ = handle.opener().open_url(url, None::<&str>);
        })
        .await
}

#[tauri::command]
pub fn gdrive_sign_out() -> Result<(), String> {
    GoogleDrive::forget()
}

#[tauri::command]
pub async fn sync_run(
    app: AppHandle,
    db: State<'_, DbState>,
    workspace_root: String,
) -> Result<SyncReport, String> {
    let drive = GoogleDrive::try_load()?
        .ok_or_else(|| "Google Drive not configured.".to_string())?;
    let root = PathBuf::from(&workspace_root);
    if !root.is_dir() {
        return Err(format!("not a directory: {workspace_root}"));
    }
    let backend: Arc<dyn crate::sync::SyncBackend> = Arc::new(drive);
    sync_once(&app, &db.pool, &root, backend).await
}
