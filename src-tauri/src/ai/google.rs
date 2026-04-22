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

    async fn list_models(&self, api_key: &str) -> Result<Vec<String>, String> {
        if api_key.is_empty() {
            return Err("missing API key".into());
        }
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            api_key
        );
        let res = super::models_client()?
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("google request: {e}"))?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(format!("google http {status}: {body}"));
        }
        let v: Value = res.json().await.map_err(|e| format!("google parse: {e}"))?;
        // Keep only models that support generateContent (chat-capable).
        let models = v
            .get("models")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|m| {
                        m.get("supportedGenerationMethods")
                            .and_then(|g| g.as_array())
                            .map(|a| a.iter().any(|v| v.as_str() == Some("generateContent")))
                            .unwrap_or(true)
                    })
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                    // API returns "models/gemini-..."; strip the "models/" prefix for display.
                    .map(|n| n.strip_prefix("models/").unwrap_or(n).to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(models)
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
