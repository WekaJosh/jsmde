mod ai;
mod commands;
mod db;
mod keychain;
mod paths;
mod rag;
mod sync;
mod workspace;

use commands::ai as ai_cmds;
use commands::fs as fs_cmds;
use commands::rag as rag_cmds;
use commands::sync as sync_cmds;
use tauri::Manager;
use workspace::watcher as ws;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // Updater is wired as a dependency but not initialized until signing is
        // configured. See RELEASE.md for the steps, then uncomment:
        // .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            ws::register(app);
            app.manage(ai_cmds::AiState::new());

            // Open DevTools automatically in debug builds only. In release,
            // the `devtools` Cargo feature keeps them available but not
            // auto-opened; there's no built-in UI to toggle them since Tauri
            // disables the right-click menu.
            #[cfg(debug_assertions)]
            if let Some(win) = app.get_webview_window("main") {
                win.open_devtools();
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match db::init().await {
                    Ok(pool) => {
                        handle.manage(sync_cmds::DbState { pool });
                    }
                    Err(e) => {
                        log::error!("db init failed: {e}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            fs_cmds::read_file,
            fs_cmds::write_file,
            fs_cmds::list_dir,
            ws::watch_workspace,
            ws::unwatch_workspace,
            ws::current_workspace,
            ai_cmds::ai_save_key,
            ai_cmds::ai_delete_key,
            ai_cmds::ai_has_key,
            ai_cmds::ai_chat_stream,
            ai_cmds::ai_cancel,
            ai_cmds::ai_list_models,
            sync_cmds::gdrive_status,
            sync_cmds::gdrive_save_client_id,
            sync_cmds::gdrive_sign_in,
            sync_cmds::gdrive_sign_out,
            sync_cmds::sync_run,
            rag_cmds::rag_status,
            rag_cmds::rag_set_enabled,
            rag_cmds::rag_reindex,
            rag_cmds::rag_reindex_file,
            rag_cmds::rag_search,
            rag_cmds::rag_clear,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
