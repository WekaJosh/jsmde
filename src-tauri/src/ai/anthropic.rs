use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::stream::{BoxStream, StreamExt};
use serde_json::{json, Value};

use super::{AiProvider, ChatMessage, ChatOptions, StreamChunk};

pub struct Anthropic;

impl Anthropic {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AiProvider for Anthropic {
    async fn chat_stream<'a>(
        &'a self,
        api_key: &'a str,
        messages: &'a [ChatMessage],
        opts: &'a ChatOptions,
    ) -> Result<BoxStream<'a, StreamChunk>, String> {
        let client = reqwest::Client::new();
        let messages_json: Vec<Value> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": if m.role == "assistant" { "assistant" } else { "user" },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = json!({
            "model": opts.model,
            "max_tokens": opts.max_tokens.unwrap_or(4096),
            "messages": messages_json,
            "stream": true,
        });
        if let Some(sys) = &opts.system {
            body["system"] = json!(sys);
        }
        if let Some(temp) = opts.temperature {
            body["temperature"] = json!(temp);
        }

        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Anthropic error {status}: {text}"));
        }

        let stream = resp.bytes_stream().eventsource().flat_map(|ev| {
            let chunks: Vec<StreamChunk> = match ev {
                Ok(event) => match event.event.as_str() {
                    "content_block_delta" => parse_content_delta(&event.data)
                        .map(|t| vec![StreamChunk::Delta { text: t }])
                        .unwrap_or_default(),
                    "message_stop" => vec![StreamChunk::Done],
                    "error" => vec![StreamChunk::Error {
                        message: event.data,
                    }],
                    _ => vec![],
                },
                Err(e) => vec![StreamChunk::Error {
                    message: e.to_string(),
                }],
            };
            futures_util::stream::iter(chunks)
        });

        Ok(Box::pin(stream))
    }

    async fn list_models(&self, api_key: &str) -> Result<Vec<String>, String> {
        if api_key.is_empty() {
            return Err("missing API key".into());
        }
        let res = super::models_client()?
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .send()
            .await
            .map_err(|e| format!("anthropic request: {e}"))?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(format!("anthropic http {status}: {body}"));
        }
        let v: Value = res
            .json()
            .await
            .map_err(|e| format!("anthropic parse: {e}"))?;
        let models = v
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(models)
    }
}

fn parse_content_delta(data: &str) -> Option<String> {
    let v: Value = serde_json::from_str(data).ok()?;
    let delta = v.get("delta")?;
    let t = delta.get("type")?.as_str()?;
    if t == "text_delta" {
        Some(delta.get("text")?.as_str()?.to_string())
    } else {
        None
    }
}
