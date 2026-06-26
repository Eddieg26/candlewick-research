use clap::{Parser, Subcommand};

use crate::error::TiingoError;

#[derive(Parser)]
#[command(name = "tiingo", about = "Tiingo API CLI client")]
pub struct Cli {
    /// API token (overrides TIINGO_API_KEY env var)
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
    },
    /// Fetch historical price data
    Prices {
        /// Stock symbol (e.g., AAPL)
        symbol: String,
        /// Frequency: 1min, 5min, 15min, 30min, 1hour, daily
        frequency: String,
        /// Start date (YYYY-MM-DD)
        from: String,
        /// End date (YYYY-MM-DD)
        to: String,
    },
    /// Stream real-time IEX trades
    Live {
        /// One or more symbols to subscribe to
        #[arg(required = true)]
        symbols: Vec<String>,
    },
}

/// Resolve the API token from the --token flag or TIINGO_API_KEY env var.
/// Returns an error if the resolved token is empty or whitespace-only.
pub fn resolve_token(flag: Option<String>) -> Result<String, TiingoError> {
    let token = if let Some(t) = flag {
        t
    } else {
        std::env::var("TIINGO_API_KEY").unwrap_or_default()
    };

    if token.trim().is_empty() {
        return Err(TiingoError::NoApiKey);
    }

    Ok(token)
}
