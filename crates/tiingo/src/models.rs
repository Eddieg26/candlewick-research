use chrono::NaiveDate;
use serde::Deserialize;

use crate::error::TiingoError;

// --- Symbol Search ---

/// Search result from Tiingo search endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub ticker: String,
    pub name: String,
    pub asset_type: String,
}

// --- Price Data ---

/// A single price bar (used for both daily and intraday responses).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceBar {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

// --- WebSocket Trade ---

/// Tiingo IEX WebSocket trade update (parsed manually from arrays).
#[derive(Debug)]
pub struct IexTrade {
    pub symbol: String,
    pub timestamp: u64,
    pub price: f64,
    pub size: f64,
}

// --- Frequency Validation ---

/// Valid frequency values for the prices command.
pub const VALID_FREQUENCIES: &[&str] = &["1min", "5min", "15min", "30min", "1hour", "daily"];

pub fn validate_frequency(freq: &str) -> Result<(), TiingoError> {
    if VALID_FREQUENCIES.contains(&freq) {
        Ok(())
    } else {
        Err(TiingoError::InvalidFrequency(freq.to_string()))
    }
}

// --- Date Validation ---

pub fn validate_date_format(date_str: &str) -> Result<(), TiingoError> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| TiingoError::InvalidDate(date_str.to_string()))?;
    Ok(())
}

// --- Frequency Classification ---

pub fn is_intraday(freq: &str) -> bool {
    matches!(freq, "1min" | "5min" | "15min" | "30min" | "1hour")
}
