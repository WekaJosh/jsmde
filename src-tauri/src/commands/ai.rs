use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use crate::ai::{get_provider, ChatMessage, ChatOptions, StreamChunk};
use crate::keychain;

type Streams = Arc<Mutex<HashMap<String, CancellationToken>>>;

pub struct AiState {
    pub streams: Streams,
}

impl AiState {
    pub fn new() -> Self {
        Self {
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

fn key_account(provider: &str) -> String {
    format!("ai:{provider}")
}

#[tauri::command]
pub fn ai_save_key(provider: String, api_key: String) -> Result<(), String> {
    keychain::set_secret(&key_account(&provider), &api_key)
}

#[tauri::command]
pub fn ai_delete_key(provider: String) -> Result<(), String> {
    keychain::delete_secret(&key_account(&provider))
}

#[tauri::command]
pub fn ai_has_key(provider: String) -> Result<bool, String> {
    Ok(keychain::get_secret(&key_account(&provider))?.is_some())
}

#[tauri::command]
pub async fn ai_chat_stream(
    app: AppHandle,
    state: State<'_, AiState>,
    request_id: String,
    provider: String,
    messages: Vec<ChatMessage>,
    options: ChatOptions,
) -> Result<(), String> {
    let api_key = keychain::get_secret(&key_account(&provider))?.unwrap_or_default();
    let Some(impl_) = get_provider(&provider) else {
        return Err(format!("unknown provider: {provider}"));
    };

    let streams = state.streams.clone();
    let token = CancellationToken::new();
    if let Ok(mut guard) = streams.lock() {
        guard.insert(request_id.clone(), token.clone());
    }

    let event = format!("ai://{request_id}");
    let request_id_clone = request_id.clone();

    tokio::spawn(async move {
        run_stream(&app, &event, impl_, api_key, messages, options, token).await;
        if let Ok(mut guard) = streams.lock() {
            guard.remove(&request_id_clone);
        }
    });

    Ok(())
}

async fn run_stream(
    app: &AppHandle,
    event: &str,
    impl_: Box<dyn crate::ai::AiProvider>,
    api_key: String,
    messages: Vec<ChatMessage>,
    options: ChatOptions,
    token: CancellationToken,
) {
    use futures_util::StreamExt;
    let mut stream = match impl_.chat_stream(&api_key, &messages, &options).await {
        Ok(s) => s,
        Err(e) => {
            let _ = app.emit(event, StreamChunk::Error { message: e });
            let _ = app.emit(event, StreamChunk::Done);
            return;
        }
    };
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                let _ = app.emit(event, StreamChunk::Error { message: "cancelled".into() });
                let _ = app.emit(event, StreamChunk::Done);
                return;
            }
            next = stream.next() => {
                match next {
                    Some(chunk) => {
                        let done = matches!(chunk, StreamChunk::Done);
                        let _ = app.emit(event, chunk);
                        if done { return; }
                    }
                    None => {
                        let _ = app.emit(event, StreamChunk::Done);
                        return;
                    }
                }
            }
        }
    }
}

#[tauri::command]
pub fn ai_cancel(state: State<'_, AiState>, request_id: String) -> Result<(), String> {
    if let Ok(mut guard) = state.streams.lock() {
        if let Some(token) = guard.remove(&request_id) {
            token.cancel();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn ai_list_models(provider: String) -> Result<Vec<String>, String> {
    let api_key = keychain::get_secret(&key_account(&provider))?.unwrap_or_default();
    let Some(impl_) = get_provider(&provider) else {
        return Err(format!("unknown provider: {provider}"));
    };
    impl_.list_models(&api_key).await
}
