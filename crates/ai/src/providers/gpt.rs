use crate::{
    error::AiError,
    ext::ResponseExt,
    providers::provider::{
        AiProvider, ChatMessage, ChatRequest, ChatRequestInput, ChatResponse, UsageMetrics,
    },
};
use serde::{Deserialize, Serialize};

pub struct GptProvider {
    http: reqwest::Client,
    api_key: String,
}

impl GptProvider {
    pub const BASE_URL: &'static str = "https://api.openai.com/v1";
}

impl AiProvider for GptProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        let url = format!("{}/responses", Self::BASE_URL);
        let body = serde_json::to_string(&GptChatRequest::from(request))?;

        let response = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization: Bearer", &self.api_key)
            .body(body)
            .send()
            .await?
            .check()
            .await?;

        let response = response
            .json::<GptChatResponse>()
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

        let response = response.json::<GptModelResponse>().await?;

        Ok(response.data.into_iter().map(|o| o.id).collect())
    }
}

#[derive(Serialize)]
pub struct GptChatRequest {
    pub model: String,
    pub input: Vec<GptChatRequestInput>,
}

impl From<ChatRequest> for GptChatRequest {
    fn from(value: ChatRequest) -> Self {
        Self {
            model: value.model,
            input: value
                .input
                .into_iter()
                .map(GptChatRequestInput::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct GptChatRequestInput {
    pub role: String,
    pub content: String,
}

impl From<ChatRequestInput> for GptChatRequestInput {
    fn from(value: ChatRequestInput) -> Self {
        Self {
            role: value.role,
            content: value.content,
        }
    }
}

#[derive(Deserialize)]
pub struct GptChatResponse {
    pub output: Vec<GptChatResponseOutput>,
    pub usage: GptUsageMetrics,
}

impl From<GptChatResponse> for ChatResponse {
    fn from(value: GptChatResponse) -> Self {
        let mut messages = Vec::new();

        for output in value.output {
            for content in output.content {
                messages.push(ChatMessage {
                    role: output.role.clone(),
                    content: content.text,
                });
            }
        }

        Self {
            messages,
            usage: UsageMetrics::from(value.usage),
        }
    }
}

#[derive(Deserialize)]
pub struct GptChatResponseOutput {
    pub role: String,
    pub content: Vec<GptChatContent>,
}

#[derive(Deserialize)]
pub struct GptChatContent {
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct GptUsageMetrics {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl From<GptUsageMetrics> for UsageMetrics {
    fn from(value: GptUsageMetrics) -> Self {
        Self {
            input_tokens: value.input_tokens,
            output_tokens: value.output_tokens,
        }
    }
}

#[derive(Deserialize)]
pub struct GptModelResponse {
    data: Vec<GptModelOutput>,
}

#[derive(Deserialize)]
pub struct GptModelOutput {
    id: String,
}
