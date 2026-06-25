use thiserror::Error;

#[derive(Debug, Error)]
pub enum FinnhubError {
    #[error("No API key provided. Set FINNHUB_API_KEY or use --token.")]
    NoApiKey,

    #[error("API key rejected as unauthorized (HTTP 401).")]
    Unauthorized,

    #[error("Endpoint requires premium subscription (HTTP 403).")]
    PremiumRequired,

    #[error("Rate limit exceeded (HTTP 429). Wait at least 1 second before retrying.")]
    RateLimited,

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("WebSocket connection failed: {0}")]
    WebSocketConnect(String),

    #[error("WebSocket connection lost: {0}")]
    WebSocketDisconnect(String),

    #[error("Invalid resolution '{0}'. Valid values: 1, 5, 15, 30, 60, D, W, M")]
    InvalidResolution(String),

    #[error("Invalid date range: from-date must be before to-date.")]
    InvalidDateRange,

    #[error("Invalid date format '{0}': expected YYYY-MM-DD")]
    InvalidDate(String),

    #[error("{0}")]
    Other(String),
}
