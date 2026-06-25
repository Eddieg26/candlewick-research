use clap::{Parser, Subcommand};

use crate::error::FinnhubError;

#[derive(Parser)]
#[command(name = "finnhub", about = "Finnhub API CLI client")]
pub struct Cli {
    /// API token (overrides FINNHUB_API_KEY env var)
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
        /// Filter by exchange
        #[arg(long)]
        exchange: Option<String>,
    },
    /// Fetch historical candlestick data
    Candles {
        /// Stock symbol (e.g., AAPL)
        symbol: String,
        /// Resolution: 1, 5, 15, 30, 60, D, W, M
        resolution: String,
        /// Start date (YYYY-MM-DD)
        from: String,
        /// End date (YYYY-MM-DD)
        to: String,
    },
    /// Stream real-time trades with per-minute aggregation
    Live {
        /// One or more symbols to subscribe to
        #[arg(required = true)]
        symbols: Vec<String>,
    },
}

/// Resolve the API token from the --token flag or FINNHUB_API_KEY env var.
/// Returns an error if the resolved token is empty or whitespace-only.
pub fn resolve_token(flag: Option<String>) -> Result<String, FinnhubError> {
    let token = if let Some(t) = flag {
        t
    } else {
        std::env::var("FINNHUB_API_KEY").unwrap_or_default()
    };

    if token.trim().is_empty() {
        return Err(FinnhubError::NoApiKey);
    }

    Ok(token)
}
