// Output formatting (plain text and JSON)

use chrono::DateTime;

use crate::models::{Candle, SearchResult};
use crate::ws::CompletedMinute;

/// Print candle data as an aligned table with ISO 8601 timestamps.
pub fn print_candles(candles: &[Candle]) {
    if candles.is_empty() {
        println!("No data available for the given parameters.");
        return;
    }

    println!(
        "{:<22} {:>10} {:>10} {:>10} {:>10} {:>12}",
        "TIMESTAMP", "OPEN", "HIGH", "LOW", "CLOSE", "VOLUME"
    );

    for candle in candles {
        let timestamp = DateTime::from_timestamp(candle.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
            .unwrap_or_else(|| "INVALID".to_string());

        println!(
            "{:<22} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>12.0}",
            timestamp, candle.open, candle.high, candle.low, candle.close, candle.volume
        );
    }
}

/// Print search results as an aligned table with columns: SYMBOL, DISPLAY, TYPE, DESCRIPTION.
/// If results is empty, prints a "no matches" message to stdout.
pub fn print_search_results(results: &[SearchResult]) {
    if results.is_empty() {
        println!("No symbols matched the query.");
        return;
    }

    // Column headers
    let header_symbol = "SYMBOL";
    let header_display = "DISPLAY";
    let header_type = "TYPE";
    let header_desc = "DESCRIPTION";

    // Calculate maximum width for each column (header vs data)
    let symbol_width = results
        .iter()
        .map(|r| r.symbol.len())
        .max()
        .unwrap_or(0)
        .max(header_symbol.len());

    let display_width = results
        .iter()
        .map(|r| r.display_symbol.len())
        .max()
        .unwrap_or(0)
        .max(header_display.len());

    let type_width = results
        .iter()
        .map(|r| r.security_type.len())
        .max()
        .unwrap_or(0)
        .max(header_type.len());

    // Print header
    println!(
        "{:<symbol_width$}    {:<display_width$}    {:<type_width$}    {}",
        header_symbol, header_display, header_type, header_desc
    );

    // Print each result row
    for result in results {
        println!(
            "{:<symbol_width$}    {:<display_width$}    {:<type_width$}    {}",
            result.symbol, result.display_symbol, result.security_type, result.description
        );
    }
}

/// Print a single completed minute aggregation line.
/// Format: SYMBOL HH:MM OPEN HIGH LOW CLOSE VOLUME
pub fn print_live_minute(completed: &CompletedMinute) {
    let timestamp_ms = completed.minute_key * 60_000;
    let dt = DateTime::from_timestamp_millis(timestamp_ms as i64).unwrap_or_default();
    let time_str = dt.format("%H:%M").to_string();

    println!(
        "{} {} {} {} {} {} {}",
        completed.symbol,
        time_str,
        completed.bucket.open,
        completed.bucket.high,
        completed.bucket.low,
        completed.bucket.close,
        completed.bucket.volume,
    );
}
