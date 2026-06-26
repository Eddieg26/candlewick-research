use thiserror::Error;

#[derive(Debug, Error)]
pub enum TiingoError {
    #[error("No API key provided. Set TIINGO_API_KEY or use --token.")]
    NoApiKey,

    #[error("API key rejected as unauthorized (HTTP 401).")]
    Unauthorized,

    #[error("Rate limit exceeded (HTTP 429). Try again later.")]
    RateLimited,

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("WebSocket connection failed: {0}")]
    WebSocketConnect(String),

    #[error("WebSocket connection lost: {0}")]
    WebSocketDisconnect(String),

    #[error("Invalid frequency '{0}'. Valid values: 1min, 5min, 15min, 30min, 1hour, daily")]
    InvalidFrequency(String),

    #[error("Invalid date range: from-date must be before to-date.")]
    InvalidDateRange,

    #[error("Invalid date format '{0}': expected YYYY-MM-DD")]
    InvalidDate(String),

    #[error("{0}")]
    Other(String),
}
