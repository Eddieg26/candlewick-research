use crate::{
    error::AiError,
    ext::ResponseExt,
    providers::provider::{
        AiProvider, ChatMessage, ChatRequest, ChatRequestInput, ChatResponse, UsageMetrics,
    },
};
use serde::{Deserialize, Serialize};

pub struct DigitalOceanProvider {
    http: reqwest::Client,
    api_key: String,
}

impl DigitalOceanProvider {
    pub const BASE_URL: &'static str = "https://inference.do-ai.run/v1";

    pub fn new(http: reqwest::Client, api_key: String) -> Self {
        Self { http, api_key }
    }
}

impl AiProvider for DigitalOceanProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        let url = format!("{}/chat/completions", Self::BASE_URL);
        let body = serde_json::to_string(&DOChatRequest::from(request))?;

        let response = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .body(body)
            .send()
            .await?
            .check()
            .await?;

        let response = response
            .json::<DOChatResponse>()
            .await
            .map(ChatResponse::from)?;

        Ok(response)
    }

    async fn models(&self) -> Result<Vec<String>, AiError> {
        let url = format!("{}/models", Self::BASE_URL);

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?
            .check()
            .await?;

        let response = response.json::<DOModelResponse>().await?;

        Ok(response.data.into_iter().map(|m| m.id).collect())
    }
}

#[derive(Serialize)]
pub struct DOChatRequest {
    pub model: String,
    pub messages: Vec<DOChatRequestInput>,
}

impl From<ChatRequest> for DOChatRequest {
    fn from(value: ChatRequest) -> Self {
        Self {
            model: value.model,
            messages: value
                .input
                .into_iter()
                .map(DOChatRequestInput::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct DOChatRequestInput {
    pub role: String,
    pub content: String,
}

impl From<ChatRequestInput> for DOChatRequestInput {
    fn from(value: ChatRequestInput) -> Self {
        Self {
            role: value.role,
            content: value.content,
        }
    }
}

#[derive(Deserialize)]
pub struct DOChatResponse {
    pub choices: Vec<DOChatChoice>,
    pub usage: DOUsageMetrics,
}

impl From<DOChatResponse> for ChatResponse {
    fn from(value: DOChatResponse) -> Self {
        let messages = value
            .choices
            .into_iter()
            .map(|choice| ChatMessage {
                role: choice.message.role,
                content: choice.message.content.unwrap_or_default(),
            })
            .collect();

        Self {
            messages,
            usage: UsageMetrics::from(value.usage),
        }
    }
}

#[derive(Deserialize)]
pub struct DOChatChoice {
    pub message: DOChatMessage,
}

#[derive(Deserialize)]
pub struct DOChatMessage {
    pub role: String,
    pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct DOUsageMetrics {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl From<DOUsageMetrics> for UsageMetrics {
    fn from(value: DOUsageMetrics) -> Self {
        Self {
            input_tokens: value.prompt_tokens,
            output_tokens: value.completion_tokens,
        }
    }
}

#[derive(Deserialize)]
pub struct DOModelResponse {
    pub data: Vec<DOModelOutput>,
}

#[derive(Deserialize)]
pub struct DOModelOutput {
    pub id: String,
}
