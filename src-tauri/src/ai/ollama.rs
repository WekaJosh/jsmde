use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use serde_json::{json, Value};

use super::{AiProvider, ChatMessage, ChatOptions, StreamChunk};

pub struct Ollama;

impl Ollama {
    pub fn new() -> Self {
        Self
    }
}

fn base_url(api_key: &str) -> String {
    if api_key.is_empty() {
        "http://127.0.0.1:11434".to_string()
    } else {
        api_key.trim_end_matches('/').to_string()
    }
}

#[async_trait]
impl AiProvider for Ollama {
    async fn chat_stream<'a>(
        &'a self,
        api_key: &'a str,
        messages: &'a [ChatMessage],
        opts: &'a ChatOptions,
    ) -> Result<BoxStream<'a, StreamChunk>, String> {
        let mut all: Vec<Value> = Vec::new();
        if let Some(sys) = &opts.system {
            all.push(json!({ "role": "system", "content": sys }));
        }
        for m in messages {
            all.push(json!({ "role": m.role, "content": m.content }));
        }

        let mut options = json!({});
        if let Some(t) = opts.temperature {
            options["temperature"] = json!(t);
        }
        if let Some(mx) = opts.max_tokens {
            options["num_predict"] = json!(mx);
        }

        let body = json!({
            "model": opts.model,
            "messages": all,
            "stream": true,
            "options": options,
        });

        let url = format!("{}/api/chat", base_url(api_key));
        let resp = reqwest::Client::new()
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama error {status}: {text}"));
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut stream = resp.bytes_stream();
            let mut buf = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buf.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(idx) = buf.find('\n') {
                            let line: String = buf.drain(..=idx).collect();
                            for c in parse_ollama(line.trim()) {
                                if tx.send(c).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(StreamChunk::Error {
                            message: e.to_string(),
                        });
                        return;
                    }
                }
            }
            let tail = buf.trim();
            if !tail.is_empty() {
                for c in parse_ollama(tail) {
                    let _ = tx.send(c);
                }
            }
        });

        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        Ok(Box::pin(stream))
    }
}

fn parse_ollama(data: &str) -> Vec<StreamChunk> {
    let mut out = Vec::new();
    if data.is_empty() {
        return out;
    }
    let v: Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(_) => return out,
    };
    if let Some(msg) = v.get("message") {
        if let Some(content) = msg.get("content").and_then(|x| x.as_str()) {
            if !content.is_empty() {
                out.push(StreamChunk::Delta {
                    text: content.to_string(),
                });
            }
        }
    }
    if v.get("done").and_then(|x| x.as_bool()).unwrap_or(false) {
        out.push(StreamChunk::Done);
    }
    out
}
