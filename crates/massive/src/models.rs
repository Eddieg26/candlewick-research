use serde::Deserialize;

use crate::error::MassiveError;

// --- Search ---

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub status: String,
    pub count: u32,
    pub results: Vec<ForexTicker>,
}

#[derive(Debug, Deserialize)]
pub struct ForexTicker {
    pub ticker: String,
    pub name: String,
    pub market: String,
    pub active: bool,
}

// --- Aggregate Bars ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggResponse {
    pub status: String,
    pub ticker: String,
    pub results_count: Option<u32>,
    #[serde(default)]
    pub results: Vec<AggBar>,
}

#[derive(Debug, Deserialize)]
pub struct AggBar {
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
    pub t: u64,
    pub n: Option<u32>,
    pub vw: Option<f64>,
}

// --- Last Quote ---

#[derive(Debug, Deserialize)]
pub struct LastQuoteResponse {
    pub status: String,
    pub symbol: String,
    pub last: QuoteData,
}

#[derive(Debug, Deserialize)]
pub struct QuoteData {
    pub ask: f64,
    pub bid: f64,
    pub exchange: u32,
    pub timestamp: u64,
}

// --- Conversion ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionResponse {
    pub status: String,
    pub from: String,
    pub to: String,
    pub initial_amount: f64,
    pub converted: f64,
    pub last: QuoteData,
}

// --- Snapshot ---

#[derive(Debug, Deserialize)]
pub struct SnapshotResponse {
    pub status: String,
    pub ticker: SnapshotTicker,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotTicker {
    pub ticker: String,
    pub day: OhlcData,
    pub last_quote: SnapshotQuote,
    pub min: OhlcData,
    pub prev_day: OhlcData,
    pub todays_change: f64,
    pub todays_change_perc: f64,
    pub updated: u64,
}

#[derive(Debug, Deserialize)]
pub struct OhlcData {
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub v: f64,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotQuote {
    pub a: f64,
    pub b: f64,
    pub t: u64,
}

// --- WebSocket Minute Aggregate ---

#[derive(Debug, Deserialize)]
pub struct WsMinuteAgg {
    pub ev: String,
    pub pair: String,
    pub o: f64,
    pub c: f64,
    pub h: f64,
    pub l: f64,
    pub v: u64,
    pub s: u64,
}

// --- Validation ---

pub const VALID_TIMESPANS: &[&str] = &[
    "minute", "hour", "day", "week", "month", "quarter", "year",
];

pub const VALID_SORT_ORDERS: &[&str] = &["asc", "desc"];

pub fn validate_timespan(ts: &str) -> Result<(), MassiveError> {
    if VALID_TIMESPANS.contains(&ts) {
        Ok(())
    } else {
        Err(MassiveError::InvalidTimespan(ts.to_string()))
    }
}

pub fn validate_sort(sort: &str) -> Result<(), MassiveError> {
    if VALID_SORT_ORDERS.contains(&sort) {
        Ok(())
    } else {
        Err(MassiveError::InvalidSort(sort.to_string()))
    }
}

pub fn validate_date_format(date_str: &str) -> Result<(), MassiveError> {
    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| MassiveError::InvalidDate(date_str.to_string()))?;
    Ok(())
}

pub fn validate_date_range(from: &str, to: &str) -> Result<(), MassiveError> {
    let from_date = chrono::NaiveDate::parse_from_str(from, "%Y-%m-%d")
        .map_err(|_| MassiveError::InvalidDate(from.to_string()))?;
    let to_date = chrono::NaiveDate::parse_from_str(to, "%Y-%m-%d")
        .map_err(|_| MassiveError::InvalidDate(to.to_string()))?;
    if from_date >= to_date {
        Err(MassiveError::InvalidDateRange)
    } else {
        Ok(())
    }
}

pub fn validate_limit(limit: u32, max: u32) -> Result<(), MassiveError> {
    if limit == 0 || limit > max {
        Err(MassiveError::InvalidLimit { value: limit, max })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy that generates valid YYYY-MM-DD date strings.
    fn valid_date_strategy() -> impl Strategy<Value = String> {
        // Generate year 0001..=9999, month 1..=12, then constrain day to valid range
        (1i32..=9999i32, 1u32..=12u32).prop_flat_map(|(year, month)| {
            let max_day = days_in_month(year, month);
            (Just(year), Just(month), 1u32..=max_day)
        })
        .prop_map(|(year, month, day)| {
            format!("{:04}-{:02}-{:02}", year, month, day)
        })
    }

    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if is_leap_year(year) { 29 } else { 28 }
            }
            _ => unreachable!(),
        }
    }

    fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    proptest! {
        /// **Validates: Requirements 3.4, 10.1**
        ///
        /// Property 2: Timespan validation completeness — For any string input,
        /// validate_timespan returns Ok iff the input is one of the seven valid values.
        #[test]
        fn prop_timespan_validation_completeness(input in "\\PC*") {
            let result = validate_timespan(&input);
            let is_valid = VALID_TIMESPANS.contains(&input.as_str());
            prop_assert_eq!(
                result.is_ok(),
                is_valid,
                "validate_timespan({:?}) returned {:?}, expected is_ok={}",
                input, result, is_valid
            );
        }

        /// **Validates: Requirements 3.5, 10.2**
        ///
        /// Property 6: Sort validation completeness — For any string input,
        /// validate_sort returns Ok iff the input is "asc" or "desc".
        #[test]
        fn prop_sort_validation_completeness(input in "\\PC*") {
            let result = validate_sort(&input);
            let is_valid = VALID_SORT_ORDERS.contains(&input.as_str());
            prop_assert_eq!(
                result.is_ok(),
                is_valid,
                "validate_sort({:?}) returned {:?}, expected is_ok={}",
                input, result, is_valid
            );
        }

        /// **Validates: Requirements 3.8, 10.4**
        ///
        /// Property 5: Limit validation boundary correctness — For any limit and max,
        /// validate_limit returns Ok iff 1 <= limit <= max.
        #[test]
        fn prop_limit_validation_boundary(limit in any::<u32>(), max in any::<u32>()) {
            let result = validate_limit(limit, max);
            let expected_ok = limit >= 1 && limit <= max;
            prop_assert_eq!(
                result.is_ok(),
                expected_ok,
                "validate_limit({}, {}) returned {:?}, expected is_ok={}",
                limit, max, result, expected_ok
            );
        }

        /// **Validates: Requirements 3.6, 3.7, 10.3, 10.5**
        ///
        /// Property 3: Date validation correctness — For any string,
        /// validate_date_format returns Ok iff it's a valid YYYY-MM-DD calendar date.
        #[test]
        fn prop_date_validation_correctness(s in "\\PC{0,20}") {
            let result = validate_date_format(&s);
            let chrono_result = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d");
            prop_assert_eq!(
                result.is_ok(),
                chrono_result.is_ok(),
                "Mismatch for input {:?}: validate_date_format={:?}, chrono={:?}",
                s, result.is_ok(), chrono_result.is_ok()
            );
        }

        /// **Validates: Requirements 3.6, 3.7, 10.3, 10.5**
        ///
        /// Property 3 (positive case): All valid dates are accepted.
        #[test]
        fn prop_valid_dates_accepted(date in valid_date_strategy()) {
            let result = validate_date_format(&date);
            prop_assert!(
                result.is_ok(),
                "Valid date {:?} was rejected", date
            );
        }

        /// **Validates: Requirements 3.6, 3.7, 10.3, 10.5**
        ///
        /// Property 4: Date range ordering — For any pair of valid dates,
        /// range validation fails iff from >= to.
        #[test]
        fn prop_date_range_ordering(
            from in valid_date_strategy(),
            to in valid_date_strategy()
        ) {
            let from_date = chrono::NaiveDate::parse_from_str(&from, "%Y-%m-%d").unwrap();
            let to_date = chrono::NaiveDate::parse_from_str(&to, "%Y-%m-%d").unwrap();

            let result = validate_date_range(&from, &to);

            if from_date >= to_date {
                prop_assert!(
                    result.is_err(),
                    "Expected error for from={:?} >= to={:?}, got Ok", from, to
                );
            } else {
                prop_assert!(
                    result.is_ok(),
                    "Expected Ok for from={:?} < to={:?}, got {:?}", from, to, result
                );
            }
        }
    }
}
