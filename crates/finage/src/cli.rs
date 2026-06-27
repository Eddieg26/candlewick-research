use clap::{Parser, Subcommand};
use crate::error::FinageError;

#[derive(Parser)]
#[command(name = "finage", about = "Finage API CLI client")]
pub struct Cli {
    /// API token (overrides FINAGE_API_KEY env var)
    #[arg(long, global = true)]
    pub token: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Search for symbols by keyword
    Search {
        /// Search query string
        query: String,
        /// Market type: us-stock, forex, crypto
        #[arg(long, default_value = "us-stock")]
        market: String,
    },
    /// Fetch historical aggregate bars
    Bars {
        /// Stock symbol (e.g., AAPL)
        symbol: String,
        /// Size of each bar (1-1440)
        multiplier: u32,
        /// Time unit: minute, hour, day, week, month, quarter, year
        timespan: String,
        /// Start date (YYYY-MM-DD)
        from: String,
        /// End date (YYYY-MM-DD)
        to: String,
        /// Sort order: asc or desc
        #[arg(long, default_value = "asc")]
        sort: String,
        /// Max results (1-50000)
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    /// Stream real-time trades via WebSocket
    Live {
        /// Symbols to subscribe to (1-20)
        #[arg(required = true)]
        symbols: Vec<String>,
    },
}

/// Resolve API token from --token flag or FINAGE_API_KEY environment variable.
/// Trims whitespace and returns NoApiKey error if result is empty.
pub fn resolve_token(flag: Option<String>) -> Result<String, FinageError> {
    let raw = match flag {
        Some(v) => v,
        None => std::env::var("FINAGE_API_KEY").unwrap_or_default(),
    };
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        Err(FinageError::NoApiKey)
    } else {
        Ok(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mutex to serialize env-var-dependent tests. Since env vars are process-global,
    /// tests that modify FINAGE_API_KEY must not run concurrently.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn clear_env() {
        // SAFETY: caller holds ENV_MUTEX; no concurrent access.
        unsafe { std::env::remove_var("FINAGE_API_KEY"); }
    }

    fn set_env(val: &str) {
        // SAFETY: caller holds ENV_MUTEX; no concurrent access.
        unsafe { std::env::set_var("FINAGE_API_KEY", val); }
    }

    #[test]
    fn resolve_token_uses_flag_when_provided() {
        let result = resolve_token(Some("my-token".to_string()));
        assert_eq!(result.unwrap(), "my-token");
    }

    #[test]
    fn resolve_token_trims_whitespace() {
        let result = resolve_token(Some("  my-token  ".to_string()));
        assert_eq!(result.unwrap(), "my-token");
    }

    #[test]
    fn resolve_token_returns_error_for_empty_string() {
        let result = resolve_token(Some("".to_string()));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FinageError::NoApiKey));
    }

    #[test]
    fn resolve_token_returns_error_for_whitespace_only() {
        let result = resolve_token(Some("   ".to_string()));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FinageError::NoApiKey));
    }

    #[test]
    fn resolve_token_falls_back_to_env_var() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        set_env("env-token");
        let result = resolve_token(None);
        clear_env();
        assert_eq!(result.unwrap(), "env-token");
    }

    #[test]
    fn resolve_token_returns_error_when_env_var_unset() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let result = resolve_token(None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FinageError::NoApiKey));
    }
}
