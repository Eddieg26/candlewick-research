use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarketError {
    #[error("No API key provided")]
    NoApiKey,

    #[error("API key rejected as unauthorized (HTTP 401).")]
    Unauthorized,

    #[error("Rate limit exceeded (HTTP 429). Try again later.")]
    RateLimited,

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("Failed to parse response: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("WebSocket connection failed: {0}")]
    WebSocketConnect(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("WebSocket connection lost: {0}")]
    WebSocketDisconnect(String),

    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("{0}")]
    Other(String),
}
