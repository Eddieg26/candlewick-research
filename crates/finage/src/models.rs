use chrono::NaiveDate;
use serde::Deserialize;

use crate::error::FinageError;

// --- Symbol Search ---

#[derive(Debug, Deserialize)]
pub struct SymbolListResponse {
    #[allow(dead_code)]
    pub page: Option<u32>,
    pub symbols: Vec<SymbolEntry>,
}

#[derive(Debug, Deserialize)]
pub struct SymbolEntry {
    pub symbol: String,
    pub name: String,
}

// --- Aggregates ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregatesResponse {
    #[allow(dead_code)]
    pub symbol: Option<String>,
    #[allow(dead_code)]
    pub total_results: Option<u32>,
    pub results: Vec<AggBar>,
}

#[derive(Debug, Deserialize)]
pub struct AggBar {
    pub o: f64, // open
    pub h: f64, // high
    pub l: f64, // low
    pub c: f64, // close
    pub v: f64, // volume
    pub t: i64, // timestamp (ms epoch)
}

// --- WebSocket Trade ---

#[derive(Debug, Deserialize)]
pub struct WsTrade {
    pub s: String,      // symbol
    pub p: f64,         // price
    pub t: i64,         // timestamp (ms epoch)
    #[allow(dead_code)]
    pub v: Option<f64>, // volume (optional)
}

// --- Validation ---

pub const VALID_TIMESPANS: &[&str] =
    &["minute", "hour", "day", "week", "month", "quarter", "year"];
pub const VALID_SORTS: &[&str] = &["asc", "desc"];
pub const VALID_MARKETS: &[&str] = &["us-stock", "forex", "crypto"];

pub fn validate_timespan(ts: &str) -> Result<(), FinageError> {
    if VALID_TIMESPANS.contains(&ts) {
        Ok(())
    } else {
        Err(FinageError::InvalidTimespan(ts.to_string()))
    }
}

pub fn validate_sort(sort: &str) -> Result<(), FinageError> {
    if VALID_SORTS.contains(&sort) {
        Ok(())
    } else {
        Err(FinageError::InvalidSort(sort.to_string()))
    }
}

pub fn validate_multiplier(m: u32) -> Result<(), FinageError> {
    if (1..=1440).contains(&m) {
        Ok(())
    } else {
        Err(FinageError::InvalidMultiplier(m))
    }
}

pub fn validate_limit(limit: u32) -> Result<(), FinageError> {
    if (1..=50000).contains(&limit) {
        Ok(())
    } else {
        Err(FinageError::InvalidLimit(limit))
    }
}

pub fn validate_market(market: &str) -> Result<(), FinageError> {
    if VALID_MARKETS.contains(&market) {
        Ok(())
    } else {
        Err(FinageError::Other(format!(
            "Invalid market '{}'. Valid values: us-stock, forex, crypto",
            market
        )))
    }
}

/// Parse a "YYYY-MM-DD" date string into a Unix timestamp (seconds since epoch, start of day UTC).
/// Returns `InvalidDate` on bad format or invalid calendar date (e.g., 2024-02-30).
pub fn parse_date_to_unix(date_str: &str) -> Result<i64, FinageError> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| FinageError::InvalidDate(date_str.to_string()))?;

    let datetime = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| FinageError::InvalidDate(date_str.to_string()))?;

    Ok(datetime.and_utc().timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_timespan_valid() {
        for ts in VALID_TIMESPANS {
            assert!(validate_timespan(ts).is_ok());
        }
    }

    #[test]
    fn test_validate_timespan_invalid() {
        assert!(validate_timespan("daily").is_err());
        assert!(validate_timespan("").is_err());
        assert!(validate_timespan("MINUTE").is_err());
    }

    #[test]
    fn test_validate_sort_valid() {
        assert!(validate_sort("asc").is_ok());
        assert!(validate_sort("desc").is_ok());
    }

    #[test]
    fn test_validate_sort_invalid() {
        assert!(validate_sort("ascending").is_err());
        assert!(validate_sort("").is_err());
    }

    #[test]
    fn test_validate_multiplier_valid() {
        assert!(validate_multiplier(1).is_ok());
        assert!(validate_multiplier(720).is_ok());
        assert!(validate_multiplier(1440).is_ok());
    }

    #[test]
    fn test_validate_multiplier_invalid() {
        assert!(validate_multiplier(0).is_err());
        assert!(validate_multiplier(1441).is_err());
    }

    #[test]
    fn test_validate_limit_valid() {
        assert!(validate_limit(1).is_ok());
        assert!(validate_limit(100).is_ok());
        assert!(validate_limit(50000).is_ok());
    }

    #[test]
    fn test_validate_limit_invalid() {
        assert!(validate_limit(0).is_err());
        assert!(validate_limit(50001).is_err());
    }

    #[test]
    fn test_validate_market_valid() {
        for m in VALID_MARKETS {
            assert!(validate_market(m).is_ok());
        }
    }

    #[test]
    fn test_validate_market_invalid() {
        assert!(validate_market("stocks").is_err());
        assert!(validate_market("").is_err());
    }

    #[test]
    fn test_parse_date_to_unix_valid() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        let ts = parse_date_to_unix("2024-01-01").unwrap();
        assert_eq!(ts, 1704067200);
    }

    #[test]
    fn test_parse_date_to_unix_invalid_format() {
        assert!(parse_date_to_unix("01-01-2024").is_err());
        assert!(parse_date_to_unix("2024/01/01").is_err());
        assert!(parse_date_to_unix("not-a-date").is_err());
    }

    #[test]
    fn test_parse_date_to_unix_invalid_calendar() {
        // Feb 30 doesn't exist
        assert!(parse_date_to_unix("2024-02-30").is_err());
        // Month 13 doesn't exist
        assert!(parse_date_to_unix("2024-13-01").is_err());
    }
}
