use chrono::DateTime;

use crate::models::{AggBar, SymbolEntry};

/// Convert a millisecond epoch timestamp to ISO 8601 UTC string (YYYY-MM-DDTHH:MM:SS).
/// Returns "INVALID" if the timestamp is out of representable range.
fn format_timestamp_ms(timestamp_ms: i64) -> String {
    let secs = timestamp_ms / 1000;
    let nanos = ((timestamp_ms % 1000) * 1_000_000) as u32;
    DateTime::from_timestamp(secs, nanos)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_else(|| "INVALID".to_string())
}

/// Print search results as a dynamically-sized aligned table.
/// Columns: SYMBOL, NAME — left-aligned, 4-space separator.
pub fn print_search_results(symbols: &[SymbolEntry]) {
    if symbols.is_empty() {
        println!("No symbols matched the query.");
        return;
    }

    let header_symbol = "SYMBOL";
    let header_name = "NAME";

    let symbol_width = symbols
        .iter()
        .map(|s| s.symbol.len())
        .max()
        .unwrap_or(0)
        .max(header_symbol.len());

    let name_width = symbols
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(0)
        .max(header_name.len());

    println!(
        "{:<symbol_width$}    {:<name_width$}",
        header_symbol, header_name
    );

    for entry in symbols {
        println!(
            "{:<symbol_width$}    {:<name_width$}",
            entry.symbol, entry.name
        );
    }
}

/// Print aggregate bars as a fixed-width aligned table.
/// Columns: TIMESTAMP(22), OPEN(10), HIGH(10), LOW(10), CLOSE(10), VOLUME(12)
pub fn print_bars(bars: &[AggBar]) {
    if bars.is_empty() {
        println!("No data available.");
        return;
    }

    println!(
        "{:<22}  {:>10}  {:>10}  {:>10}  {:>10}  {:>12}",
        "TIMESTAMP", "OPEN", "HIGH", "LOW", "CLOSE", "VOLUME"
    );

    for bar in bars {
        let timestamp = format_timestamp_ms(bar.t);

        println!(
            "{:<22}  {:>10.4}  {:>10.4}  {:>10.4}  {:>10.4}  {:>12.0}",
            timestamp, bar.o, bar.h, bar.l, bar.c, bar.v
        );
    }
}

/// Print a single live trade line.
/// Format: symbol (left), price (right, 4dp), timestamp (ISO 8601), 2+ space separators.
pub fn print_trade(symbol: &str, price: f64, timestamp_ms: i64) {
    let timestamp = format_timestamp_ms(timestamp_ms);
    println!("{}  {:>10.4}  {}", symbol, price, timestamp);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AggBar, SymbolEntry};

    #[test]
    fn test_format_timestamp_ms_valid() {
        // 2024-06-15T00:00:00 UTC = 1718409600000 ms
        let result = format_timestamp_ms(1718409600000);
        assert_eq!(result, "2024-06-15T00:00:00");
    }

    #[test]
    fn test_format_timestamp_ms_invalid() {
        // An extremely large timestamp that can't be represented
        let result = format_timestamp_ms(i64::MAX);
        assert_eq!(result, "INVALID");
    }

    #[test]
    fn test_format_timestamp_ms_zero() {
        let result = format_timestamp_ms(0);
        assert_eq!(result, "1970-01-01T00:00:00");
    }

    #[test]
    fn test_print_search_results_empty() {
        // Should not panic, prints message
        print_search_results(&[]);
    }

    #[test]
    fn test_print_search_results_nonempty() {
        let symbols = vec![
            SymbolEntry {
                symbol: "AAPL".to_string(),
                name: "Apple Inc".to_string(),
            },
            SymbolEntry {
                symbol: "AAPD".to_string(),
                name: "Direxion Daily AAPL Bear 1X Shares".to_string(),
            },
        ];
        // Should not panic
        print_search_results(&symbols);
    }

    #[test]
    fn test_print_bars_empty() {
        print_bars(&[]);
    }

    #[test]
    fn test_print_bars_nonempty() {
        let bars = vec![AggBar {
            o: 182.63,
            h: 183.12,
            l: 181.90,
            c: 182.95,
            v: 1234567.0,
            t: 1718400000000,
        }];
        print_bars(&bars);
    }

    #[test]
    fn test_print_bars_invalid_timestamp() {
        let bars = vec![AggBar {
            o: 100.0,
            h: 101.0,
            l: 99.0,
            c: 100.5,
            v: 500.0,
            t: i64::MAX,
        }];
        // Should print "INVALID" for the timestamp without panicking
        print_bars(&bars);
    }

    #[test]
    fn test_print_trade() {
        // Should not panic
        print_trade("AAPL", 182.54, 1718461822000);
    }

    #[test]
    fn test_print_trade_invalid_timestamp() {
        print_trade("MSFT", 400.1234, i64::MAX);
    }
}
