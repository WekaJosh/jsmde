use async_trait::async_trait;
use futures_util::stream::{BoxStream, StreamExt};
use serde_json::{json, Value};

use super::{AiProvider, ChatMessage, ChatOptions, StreamChunk};

pub struct Google;

impl Google {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AiProvider for Google {
    async fn chat_stream<'a>(
        &'a self,
        api_key: &'a str,
        messages: &'a [ChatMessage],
        opts: &'a ChatOptions,
    ) -> Result<BoxStream<'a, StreamChunk>, String> {
        let contents: Vec<Value> = messages
            .iter()
            .map(|m| {
                json!({
                    "role": if m.role == "assistant" { "model" } else { "user" },
                    "parts": [{ "text": m.content }],
                })
            })
            .collect();

        let mut body = json!({ "contents": contents });
        if let Some(sys) = &opts.system {
            body["systemInstruction"] = json!({ "parts": [{ "text": sys }] });
        }
        let mut config = json!({});
        if let Some(t) = opts.temperature {
            config["temperature"] = json!(t);
        }
        if let Some(mx) = opts.max_tokens {
            config["maxOutputTokens"] = json!(mx);
        }
        if config.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
            body["generationConfig"] = config;
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            opts.model, api_key
        );

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
            return Err(format!("Google error {status}: {text}"));
        }

        use eventsource_stream::Eventsource;
        let stream = resp.bytes_stream().eventsource().flat_map(|ev| {
            let chunks: Vec<StreamChunk> = match ev {
                Ok(event) => parse_google(&event.data)
                    .map(|t| vec![StreamChunk::Delta { text: t }])
                    .unwrap_or_default(),
                Err(e) => vec![StreamChunk::Error {
                    message: e.to_string(),
                }],
            };
            futures_util::stream::iter(chunks)
        });

        Ok(Box::pin(
            stream.chain(futures_util::stream::iter(vec![StreamChunk::Done])),
        ))
    }
}

fn parse_google(data: &str) -> Option<String> {
    let v: Value = serde_json::from_str(data).ok()?;
    let candidates = v.get("candidates")?.as_array()?;
    let first = candidates.first()?;
    let parts = first.get("content")?.get("parts")?.as_array()?;
    let mut out = String::new();
    for p in parts {
        if let Some(t) = p.get("text").and_then(|x| x.as_str()) {
            out.push_str(t);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}
