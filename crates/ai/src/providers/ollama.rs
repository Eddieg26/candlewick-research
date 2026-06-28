use crate::{
    error::AiError,
    ext::ResponseExt,
    providers::provider::{
        AiProvider, ChatMessage, ChatRequest, ChatRequestInput, ChatResponse, UsageMetrics,
    },
};
use serde::{Deserialize, Serialize};

pub struct OllamaProvider {
    http: reqwest::Client,
    base_url: String,
}

impl OllamaProvider {
    pub const DEFAULT_BASE_URL: &'static str = "http://localhost:11434";

    pub fn new(base_url: Option<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| Self::DEFAULT_BASE_URL.to_string()),
        }
    }
}

impl AiProvider for OllamaProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        let url = format!("{}/api/chat", self.base_url);
        let body = serde_json::to_string(&OllamaChatRequest::from(request))?;

        let response = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?
            .check()
            .await?;

        let response = response
            .json::<OllamaChatResponse>()
            .await
            .map(ChatResponse::from)?;

        Ok(response)
    }

    async fn models(&self) -> Result<Vec<String>, AiError> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .http
            .get(&url)
            .send()
            .await?
            .check()
            .await?;

        let response = response.json::<OllamaModelResponse>().await?;

        Ok(response.models.into_iter().map(|m| m.name).collect())
    }
}

#[derive(Serialize)]
pub struct OllamaChatRequest {
    pub model: String,
    pub messages: Vec<OllamaChatRequestInput>,
    pub stream: bool,
}

impl From<ChatRequest> for OllamaChatRequest {
    fn from(value: ChatRequest) -> Self {
        Self {
            model: value.model,
            messages: value
                .input
                .into_iter()
                .map(OllamaChatRequestInput::from)
                .collect(),
            stream: false,
        }
    }
}

#[derive(Serialize)]
pub struct OllamaChatRequestInput {
    pub role: String,
    pub content: String,
}

impl From<ChatRequestInput> for OllamaChatRequestInput {
    fn from(value: ChatRequestInput) -> Self {
        Self {
            role: value.role,
            content: value.content,
        }
    }
}

#[derive(Deserialize)]
pub struct OllamaChatResponse {
    pub message: OllamaChatMessage,
    #[serde(default)]
    pub prompt_eval_count: u32,
    #[serde(default)]
    pub eval_count: u32,
}

impl From<OllamaChatResponse> for ChatResponse {
    fn from(value: OllamaChatResponse) -> Self {
        Self {
            messages: vec![ChatMessage {
                role: value.message.role,
                content: value.message.content,
            }],
            usage: UsageMetrics {
                input_tokens: value.prompt_eval_count,
                output_tokens: value.eval_count,
            },
        }
    }
}

#[derive(Deserialize)]
pub struct OllamaChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct OllamaModelResponse {
    pub models: Vec<OllamaModelOutput>,
}

#[derive(Deserialize)]
pub struct OllamaModelOutput {
    pub name: String,
}
