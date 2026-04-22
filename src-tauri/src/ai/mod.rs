pub mod anthropic;
pub mod google;
pub mod ollama;
pub mod openai;

use async_trait::async_trait;
use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOptions {
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub system: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum StreamChunk {
    Delta { text: String },
    Done,
    Error { message: String },
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn chat_stream<'a>(
        &'a self,
        api_key: &'a str,
        messages: &'a [ChatMessage],
        opts: &'a ChatOptions,
    ) -> Result<BoxStream<'a, StreamChunk>, String>;
}

pub fn get_provider(id: &str) -> Option<Box<dyn AiProvider>> {
    match id {
        "anthropic" => Some(Box::new(anthropic::Anthropic::new())),
        "openai" => Some(Box::new(openai::OpenAi::new())),
        "google" => Some(Box::new(google::Google::new())),
        "ollama" => Some(Box::new(ollama::Ollama::new())),
        _ => None,
    }
}
