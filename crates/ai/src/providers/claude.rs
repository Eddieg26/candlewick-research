use crate::{
    error::AiError,
    ext::ResponseExt,
    providers::provider::{
        AiProvider, ChatMessage, ChatRequest, ChatRequestInput, ChatResponse, UsageMetrics,
    },
};
use serde::{Deserialize, Serialize};

pub struct ClaudeProvider {
    http: reqwest::Client,
    api_key: String,
}

impl ClaudeProvider {
    pub const BASE_URL: &'static str = "https://api.anthropic.com/v1";
}

impl AiProvider for ClaudeProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        let url = format!("{}/responses", Self::BASE_URL);
        let body = serde_json::to_string(&ClaudeChatRequest::from(request))?;

        let response = self
            .http
            .post(&url)
            .header("anthropic-version", "2023-06-01")
            .header("X-Api-Key", &self.api_key)
            .body(body)
            .send()
            .await?
            .check()
            .await?;

        let response = response
            .json::<ClaudeChatResponse>()
            .await
            .map(ChatResponse::from)?;

        Ok(response)
    }

    async fn models(&self) -> Result<Vec<String>, AiError> {
        let url = format!("{}/models", Self::BASE_URL);

        let response = self
            .http
            .get(&url)
            .header("Authorization: Bearer", &self.api_key)
            .send()
            .await?
            .check()
            .await?;

        let response = response.json::<ClaudeModelResponse>().await?;

        Ok(response.data.into_iter().map(|o| o.id).collect())
    }
}

#[derive(Serialize)]
pub struct ClaudeChatRequest {
    pub model: String,
    pub messages: Vec<ClaudeChatRequestInput>,
}

impl From<ChatRequest> for ClaudeChatRequest {
    fn from(value: ChatRequest) -> Self {
        Self {
            model: value.model,
            messages: value
                .input
                .into_iter()
                .map(ClaudeChatRequestInput::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct ClaudeChatRequestInput {
    pub role: String,
    pub content: String,
}

impl From<ChatRequestInput> for ClaudeChatRequestInput {
    fn from(value: ChatRequestInput) -> Self {
        Self {
            role: value.role,
            content: value.content,
        }
    }
}

#[derive(Deserialize)]
pub struct ClaudeChatResponse {
    pub role: String,
    pub content: Vec<ClaudeChatResponseOutput>,
    pub usage: ClaudeUsageMetrics,
}

impl From<ClaudeChatResponse> for ChatResponse {
    fn from(value: ClaudeChatResponse) -> Self {
        let mut messages = Vec::new();

        for output in value.content {
            messages.push(ChatMessage {
                role: value.role.clone(),
                content: output.text,
            });
        }

        Self {
            messages,
            usage: UsageMetrics::from(value.usage),
        }
    }
}

#[derive(Deserialize)]
pub struct ClaudeChatResponseOutput {
    pub text: String,
}

#[derive(Deserialize)]
pub struct ClaudeUsageMetrics {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl From<ClaudeUsageMetrics> for UsageMetrics {
    fn from(value: ClaudeUsageMetrics) -> Self {
        Self {
            input_tokens: value.input_tokens,
            output_tokens: value.output_tokens,
        }
    }
}

#[derive(Deserialize)]
pub struct ClaudeModelResponse {
    pub data: Vec<ClaudeModelOutput>,
}

#[derive(Deserialize)]
pub struct ClaudeModelOutput {
    pub id: String,
    pub display_name: String,
}
