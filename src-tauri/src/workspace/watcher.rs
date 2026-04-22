use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Debug, Serialize, Clone)]
pub struct WorkspaceEvent {
    pub kind: &'static str,
    pub paths: Vec<String>,
}

pub struct WatcherState {
    inner: Mutex<Option<ActiveWatcher>>,
}

struct ActiveWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher, FileIdMap>,
    root: PathBuf,
}

impl WatcherState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn current_root(&self) -> Option<PathBuf> {
        self.inner.lock().ok()?.as_ref().map(|w| w.root.clone())
    }
}

#[tauri::command]
pub fn watch_workspace(
    app: AppHandle,
    state: State<'_, WatcherState>,
    path: String,
) -> Result<(), String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let app_for_events = app.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(250),
        None,
        move |res: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| match res {
            Ok(events) => {
                let paths: Vec<String> = events
                    .into_iter()
                    .flat_map(|e| e.event.paths.into_iter())
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                if !paths.is_empty() {
                    let _ = app_for_events.emit(
                        "workspace://change",
                        WorkspaceEvent {
                            kind: "change",
                            paths,
                        },
                    );
                }
            }
            Err(errors) => {
                log::warn!("watcher errors: {:?}", errors);
            }
        },
    )
    .map_err(|e| e.to_string())?;

    debouncer
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    let mut guard = state.inner.lock().map_err(|e| e.to_string())?;
    *guard = Some(ActiveWatcher {
        _debouncer: debouncer,
        root,
    });
    Ok(())
}

#[tauri::command]
pub fn unwatch_workspace(state: State<'_, WatcherState>) -> Result<(), String> {
    let mut guard = state.inner.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn current_workspace(state: State<'_, WatcherState>) -> Option<String> {
    state
        .current_root()
        .map(|p| p.to_string_lossy().into_owned())
}

pub fn register(app: &mut tauri::App) {
    app.manage(WatcherState::new());
}
