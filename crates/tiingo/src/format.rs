// Output formatting (plain text tables)

use chrono::DateTime;

use crate::models::{PriceBar, SearchResult};
use crate::ws::CompletedMinute;

/// Print search results as an aligned table with columns: TICKER, NAME, EXCHANGE, TYPE.
/// If results is empty, prints a "no matches" message to stdout.
pub fn print_search_results(results: &[SearchResult]) {
    if results.is_empty() {
        println!("No symbols matched the query.");
        return;
    }

    let header_ticker = "TICKER";
    let header_name = "NAME";
    let header_type = "TYPE";

    let ticker_width = results
        .iter()
        .map(|r| r.ticker.len())
        .max()
        .unwrap_or(0)
        .max(header_ticker.len());

    let name_width = results
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(0)
        .max(header_name.len());

    let type_width = results
        .iter()
        .map(|r| r.asset_type.len())
        .max()
        .unwrap_or(0)
        .max(header_type.len());

    println!(
        "{:<ticker_width$}    {:<name_width$}    {:<type_width$}",
        header_ticker, header_name, header_type
    );

    for result in results {
        println!(
            "{:<ticker_width$}    {:<name_width$}    {:<type_width$}",
            result.ticker, result.name, result.asset_type
        );
    }
}

/// Print price bars as an aligned table with columns: DATE, OPEN, HIGH, LOW, CLOSE, VOLUME.
/// DATE is left-aligned; numeric columns are right-aligned (4 decimal places for prices, 0 for volume).
pub fn print_prices(bars: &[PriceBar]) {
    if bars.is_empty() {
        println!("No data available for the given parameters.");
        return;
    }

    println!(
        "{:<22} {:>10} {:>10} {:>10} {:>10} {:>12}",
        "DATE", "OPEN", "HIGH", "LOW", "CLOSE", "VOLUME"
    );

    for bar in bars {
        println!(
            "{:<22} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>12.0}",
            bar.date, bar.open, bar.high, bar.low, bar.close, bar.volume
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
