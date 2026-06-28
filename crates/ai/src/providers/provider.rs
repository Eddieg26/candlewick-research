use serde::{Deserialize, Serialize};
use crate::error::AiError;

pub trait AiProvider {
    fn chat(&self, request: ChatRequest) -> impl Future<Output = Result<ChatResponse, AiError>>;

    fn models(&self) -> impl Future<Output = Result<Vec<String>, AiError>>;
}

#[derive(Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub input: Vec<ChatRequestInput>,
}

#[derive(Serialize, Deserialize)]
pub struct ChatRequestInput {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct ChatResponse {
    pub messages: Vec<ChatMessage>,
    pub usage: UsageMetrics,
}

#[derive(Serialize, Deserialize)]
pub struct UsageMetrics {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
