use thiserror::Error;

#[derive(Debug, Error)]
pub enum MassiveError {
    #[error("No API key provided. Set MASSIVE_API_KEY or use --token.")]
    NoApiKey,

    #[error("API key rejected as unauthorized (HTTP 401).")]
    Unauthorized,

    #[error("Endpoint requires premium subscription (HTTP 403).")]
    PremiumRequired,

    #[error("Rate limit exceeded (HTTP 429). Wait before retrying.")]
    RateLimited,

    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("WebSocket connection failed: {0}")]
    WebSocketConnect(String),

    #[error("WebSocket connection lost: {0}")]
    WebSocketDisconnect(String),

    #[error("Invalid timespan '{0}'. Valid: minute, hour, day, week, month, quarter, year")]
    InvalidTimespan(String),

    #[error("Invalid sort order '{0}'. Valid: asc, desc")]
    InvalidSort(String),

    #[error("Invalid date format '{0}': expected YYYY-MM-DD")]
    InvalidDate(String),

    #[error("Invalid date range: from-date must be before to-date.")]
    InvalidDateRange,

    #[error("Invalid limit {value}: must be 1..={max}")]
    InvalidLimit { value: u32, max: u32 },

    #[error("{0}")]
    Other(String),
}
