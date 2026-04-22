use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures_util::stream::{BoxStream, StreamExt};
use serde_json::{json, Value};

use super::{AiProvider, ChatMessage, ChatOptions, StreamChunk};

pub struct OpenAi;

impl OpenAi {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AiProvider for OpenAi {
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

        let mut body = json!({
            "model": opts.model,
            "messages": all,
            "stream": true,
        });
        if let Some(t) = opts.temperature {
            body["temperature"] = json!(t);
        }
        if let Some(mx) = opts.max_tokens {
            body["max_tokens"] = json!(mx);
        }

        let resp = reqwest::Client::new()
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAI error {status}: {text}"));
        }

        let stream = resp.bytes_stream().eventsource().flat_map(|ev| {
            let chunks: Vec<StreamChunk> = match ev {
                Ok(event) => {
                    if event.data.trim() == "[DONE]" {
                        vec![StreamChunk::Done]
                    } else if let Some(text) = parse_delta(&event.data) {
                        vec![StreamChunk::Delta { text }]
                    } else {
                        vec![]
                    }
                }
                Err(e) => vec![StreamChunk::Error {
                    message: e.to_string(),
                }],
            };
            futures_util::stream::iter(chunks)
        });

        Ok(Box::pin(stream))
    }
}

fn parse_delta(data: &str) -> Option<String> {
    let v: Value = serde_json::from_str(data).ok()?;
    let choices = v.get("choices")?.as_array()?;
    let first = choices.first()?;
    let delta = first.get("delta")?;
    let content = delta.get("content")?.as_str()?;
    Some(content.to_string())
}
