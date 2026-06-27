use thiserror::Error;

#[derive(Debug, Error)]
pub enum FinageError {
    #[error("No API key provided. Set FINAGE_API_KEY or use --token.")]
    NoApiKey,

    #[error("API key rejected as unauthorized (HTTP 401).")]
    Unauthorized,

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

    #[error("Invalid timespan '{0}'. Valid values: minute, hour, day, week, month, quarter, year")]
    InvalidTimespan(String),

    #[error("Invalid sort order '{0}'. Valid values: asc, desc")]
    InvalidSort(String),

    #[error("Invalid date format '{0}': expected YYYY-MM-DD")]
    InvalidDate(String),

    #[error("Invalid date range: from-date must be before to-date.")]
    InvalidDateRange,

    #[error("Invalid limit '{0}': must be between 1 and 50000")]
    InvalidLimit(u32),

    #[error("Invalid multiplier '{0}': must be between 1 and 1440")]
    InvalidMultiplier(u32),

    #[error("{0}")]
    Other(String),
}
