use chrono::NaiveDate;
use serde::Deserialize;

use crate::error::FinnhubError;

// --- Symbol Search ---

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub count: u32,
    pub result: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub description: String,
    #[serde(rename = "displaySymbol")]
    pub display_symbol: String,
    pub symbol: String,
    #[serde(rename = "type")]
    pub security_type: String,
}

// --- Stock Candles ---

#[derive(Debug, Deserialize)]
pub struct CandleResponse {
    pub s: String, // "ok" or "no_data"
    #[serde(default)]
    pub c: Vec<f64>, // close prices
    #[serde(default)]
    pub h: Vec<f64>, // high prices
    #[serde(default)]
    pub l: Vec<f64>, // low prices
    #[serde(default)]
    pub o: Vec<f64>, // open prices
    #[serde(default)]
    pub t: Vec<i64>, // timestamps (Unix)
    #[serde(default)]
    pub v: Vec<f64>, // volumes
}

/// A single candle derived from the parallel arrays in CandleResponse.
#[derive(Debug)]
pub struct Candle {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl CandleResponse {
    /// Convert parallel arrays into a Vec<Candle>.
    pub fn into_candles(self) -> Vec<Candle> {
        (0..self.t.len())
            .map(|i| Candle {
                timestamp: self.t[i],
                open: self.o[i],
                high: self.h[i],
                low: self.l[i],
                close: self.c[i],
                volume: self.v[i],
            })
            .collect()
    }
}

// --- WebSocket Trade ---

#[derive(Debug, Deserialize)]
pub struct WsMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(default)]
    pub data: Vec<Trade>,
}

#[derive(Debug, Deserialize)]
pub struct Trade {
    pub s: String,     // symbol
    pub p: f64,        // price
    pub t: u64,        // timestamp in milliseconds
    pub v: f64,        // volume
    #[serde(default)]
    pub c: Vec<String>, // trade conditions
}

// --- Resolution Validation ---

pub const VALID_RESOLUTIONS: &[&str] = &["1", "5", "15", "30", "60", "D", "W", "M"];

pub fn validate_resolution(res: &str) -> Result<(), FinnhubError> {
    if VALID_RESOLUTIONS.contains(&res) {
        Ok(())
    } else {
        Err(FinnhubError::InvalidResolution(res.to_string()))
    }
}

// --- Date Handling ---

pub fn parse_date_to_unix(date_str: &str) -> Result<i64, FinnhubError> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| FinnhubError::InvalidDate(date_str.to_string()))?;
    Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp())
}
