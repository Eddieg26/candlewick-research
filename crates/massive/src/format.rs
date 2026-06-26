// Output formatting functions for Massive Forex CLI

use chrono::DateTime;

use crate::models::{
    AggBar, ConversionResponse, ForexTicker, LastQuoteResponse, SnapshotTicker, WsMinuteAgg,
};

/// Convert a millisecond epoch timestamp to ISO 8601 string (YYYY-MM-DDTHH:MM:SS).
/// Returns "INVALID" if the timestamp cannot be converted.
fn format_timestamp_ms(ms: u64) -> String {
    DateTime::from_timestamp_millis(ms as i64)
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S").to_string())
        .unwrap_or_else(|| "INVALID".to_string())
}

/// Format search results as an aligned table string.
/// Returns the full table including header row.
pub(crate) fn format_search_results(results: &[ForexTicker]) -> String {
    if results.is_empty() {
        return "No symbols matched the query.".to_string();
    }

    let header_ticker = "TICKER";
    let header_name = "NAME";
    let header_active = "ACTIVE";

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

    let active_width = results
        .iter()
        .map(|r| if r.active { 4 } else { 5 })
        .max()
        .unwrap_or(0)
        .max(header_active.len());

    let mut output = format!(
        "{:<ticker_width$}    {:<name_width$}    {:<active_width$}",
        header_ticker, header_name, header_active
    );

    for result in results {
        output.push('\n');
        output.push_str(&format!(
            "{:<ticker_width$}    {:<name_width$}    {:<active_width$}",
            result.ticker, result.name, result.active
        ));
    }

    output
}

/// Format aggregate bars as an aligned table string.
pub(crate) fn format_bars(bars: &[AggBar]) -> String {
    if bars.is_empty() {
        return "No data available.".to_string();
    }

    let mut output = format!(
        "{:<22} {:>10} {:>10} {:>10} {:>10} {:>12}",
        "TIMESTAMP", "OPEN", "HIGH", "LOW", "CLOSE", "VOLUME"
    );

    for bar in bars {
        let timestamp = format_timestamp_ms(bar.t);
        output.push('\n');
        output.push_str(&format!(
            "{:<22} {:>10.4} {:>10.4} {:>10.4} {:>10.4} {:>12.0}",
            timestamp, bar.o, bar.h, bar.l, bar.c, bar.v
        ));
    }

    output
}

/// Format last quote as a string.
pub(crate) fn format_quote(resp: &LastQuoteResponse) -> String {
    let timestamp = format_timestamp_ms(resp.last.timestamp);
    format!(
        "{} bid: {:.4} ask: {:.4} @ {}",
        resp.symbol, resp.last.bid, resp.last.ask, timestamp
    )
}

/// Format conversion result as a string.
pub(crate) fn format_conversion(resp: &ConversionResponse) -> String {
    format!(
        "{} {} = {} {} (bid: {:.4}, ask: {:.4})",
        resp.initial_amount, resp.from, resp.converted, resp.to, resp.last.bid, resp.last.ask
    )
}

/// Format snapshot as a multi-line string.
pub(crate) fn format_snapshot(ticker: &SnapshotTicker) -> String {
    let quote_ts = format_timestamp_ms(ticker.last_quote.t);
    format!(
        "Snapshot: {}\n\
         \n\
         Day:\n\
         \x20 Open: {:.4}  High: {:.4}  Low: {:.4}  Close: {:.4}  Volume: {:.0}\n\
         \n\
         Prev Day:\n\
         \x20 Open: {:.4}  High: {:.4}  Low: {:.4}  Close: {:.4}  Volume: {:.0}\n\
         \n\
         Last Quote:\n\
         \x20 Ask: {:.4}  Bid: {:.4}  @ {}\n\
         \n\
         Minute:\n\
         \x20 Open: {:.4}  High: {:.4}  Low: {:.4}  Close: {:.4}  Volume: {:.0}\n\
         \n\
         Today's Change:\n\
         \x20 Change: {:.4}  Percent: {:.4}%",
        ticker.ticker,
        ticker.day.o, ticker.day.h, ticker.day.l, ticker.day.c, ticker.day.v,
        ticker.prev_day.o, ticker.prev_day.h, ticker.prev_day.l, ticker.prev_day.c, ticker.prev_day.v,
        ticker.last_quote.a, ticker.last_quote.b, quote_ts,
        ticker.min.o, ticker.min.h, ticker.min.l, ticker.min.c, ticker.min.v,
        ticker.todays_change, ticker.todays_change_perc
    )
}

/// Format a single WebSocket minute aggregate line.
pub(crate) fn format_ws_minute(agg: &WsMinuteAgg) -> String {
    let dt = DateTime::from_timestamp_millis(agg.s as i64).unwrap_or_default();
    let time_str = dt.format("%H:%M").to_string();
    format!(
        "{} {} {} {} {} {} {}",
        agg.pair, time_str, agg.o, agg.h, agg.l, agg.c, agg.v
    )
}

/// Print search results as an aligned table: TICKER, NAME, ACTIVE columns.
/// Column width = max(header, longest value); 4-space separator between columns.
/// Prints "No symbols matched the query." for empty input.
pub fn print_search_results(results: &[ForexTicker]) {
    println!("{}", format_search_results(results));
}

/// Print aggregate bars as an aligned table.
/// TIMESTAMP: 22-char left-aligned; OPEN/HIGH/LOW/CLOSE: 10-char right-aligned, 4dp;
/// VOLUME: 12-char right-aligned, 0dp. Header row included.
/// Prints "No data available." for empty input.
pub fn print_bars(bars: &[AggBar]) {
    println!("{}", format_bars(bars));
}

/// Print last quote: SYMBOL, BID (4dp), ASK (4dp), TIMESTAMP (ISO 8601).
pub fn print_quote(resp: &LastQuoteResponse) {
    println!("{}", format_quote(resp));
}

/// Print conversion result: FROM, TO, AMOUNT, CONVERTED, BID, ASK.
/// Single formatted line.
pub fn print_conversion(resp: &ConversionResponse) {
    println!("{}", format_conversion(resp));
}

/// Print snapshot with labeled sections: Day, Prev Day, Last Quote, Minute, Today's Change.
/// Prices at 4dp, volume at 0dp.
pub fn print_snapshot(ticker: &SnapshotTicker) {
    println!("{}", format_snapshot(ticker));
}

/// Print a single WebSocket minute aggregate line: PAIR HH:MM OPEN HIGH LOW CLOSE VOLUME.
pub fn print_ws_minute(agg: &WsMinuteAgg) {
    println!("{}", format_ws_minute(agg));
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{OhlcData, QuoteData, SnapshotQuote};
    use proptest::prelude::*;

    // --- Proptest strategies for model types ---

    fn finite_positive_f64() -> impl Strategy<Value = f64> {
        (0.0001f64..10000.0f64).prop_map(|v| (v * 10000.0).round() / 10000.0)
    }

    fn valid_timestamp_ms() -> impl Strategy<Value = u64> {
        1_000_000_000_000u64..=2_000_000_000_000u64
    }

    fn ticker_string() -> impl Strategy<Value = String> {
        "[A-Z]{1,3}:[A-Z]{3,6}".prop_map(|s| s)
    }

    fn name_string() -> impl Strategy<Value = String> {
        "[A-Za-z ]{3,30}".prop_map(|s| s)
    }

    fn currency_code() -> impl Strategy<Value = String> {
        "[A-Z]{3}".prop_map(|s| s)
    }

    fn pair_string() -> impl Strategy<Value = String> {
        "[A-Z]{3}/[A-Z]{3}".prop_map(|s| s)
    }

    fn forex_ticker_strategy() -> impl Strategy<Value = ForexTicker> {
        (ticker_string(), name_string(), any::<bool>()).prop_map(|(ticker, name, active)| {
            ForexTicker {
                ticker,
                name,
                market: "fx".to_string(),
                active,
            }
        })
    }

    fn agg_bar_strategy() -> impl Strategy<Value = AggBar> {
        (
            finite_positive_f64(),
            finite_positive_f64(),
            finite_positive_f64(),
            finite_positive_f64(),
            finite_positive_f64(),
            valid_timestamp_ms(),
        )
            .prop_map(|(o, h, l, c, v, t)| AggBar {
                o,
                h,
                l,
                c,
                v,
                t,
                n: None,
                vw: None,
            })
    }

    fn quote_data_strategy() -> impl Strategy<Value = QuoteData> {
        (finite_positive_f64(), finite_positive_f64(), valid_timestamp_ms()).prop_map(
            |(ask, bid, timestamp)| QuoteData {
                ask,
                bid,
                exchange: 1,
                timestamp,
            },
        )
    }

    fn last_quote_response_strategy() -> impl Strategy<Value = LastQuoteResponse> {
        (ticker_string(), quote_data_strategy()).prop_map(|(symbol, last)| LastQuoteResponse {
            status: "OK".to_string(),
            symbol,
            last,
        })
    }

    fn conversion_response_strategy() -> impl Strategy<Value = ConversionResponse> {
        (
            currency_code(),
            currency_code(),
            finite_positive_f64(),
            finite_positive_f64(),
            quote_data_strategy(),
        )
            .prop_map(|(from, to, initial_amount, converted, last)| ConversionResponse {
                status: "OK".to_string(),
                from,
                to,
                initial_amount,
                converted,
                last,
            })
    }

    fn ohlc_data_strategy() -> impl Strategy<Value = OhlcData> {
        (
            finite_positive_f64(),
            finite_positive_f64(),
            finite_positive_f64(),
            finite_positive_f64(),
            finite_positive_f64(),
        )
            .prop_map(|(o, h, l, c, v)| OhlcData { o, h, l, c, v })
    }

    fn snapshot_quote_strategy() -> impl Strategy<Value = SnapshotQuote> {
        (finite_positive_f64(), finite_positive_f64(), valid_timestamp_ms())
            .prop_map(|(a, b, t)| SnapshotQuote { a, b, t })
    }

    fn snapshot_ticker_strategy() -> impl Strategy<Value = SnapshotTicker> {
        (
            ticker_string(),
            ohlc_data_strategy(),
            ohlc_data_strategy(),
            ohlc_data_strategy(),
            snapshot_quote_strategy(),
            finite_positive_f64(),
            finite_positive_f64(),
            valid_timestamp_ms(),
        )
            .prop_map(
                |(ticker, day, prev_day, min, last_quote, todays_change, todays_change_perc, updated)| {
                    SnapshotTicker {
                        ticker,
                        day,
                        prev_day,
                        min,
                        last_quote,
                        todays_change,
                        todays_change_perc,
                        updated,
                    }
                },
            )
    }

    fn ws_minute_agg_strategy() -> impl Strategy<Value = WsMinuteAgg> {
        (
            pair_string(),
            finite_positive_f64(),
            finite_positive_f64(),
            finite_positive_f64(),
            finite_positive_f64(),
            1u64..=1_000_000u64,
            valid_timestamp_ms(),
        )
            .prop_map(|(pair, o, h, l, c, v, s)| WsMinuteAgg {
                ev: "CA".to_string(),
                pair,
                o,
                c,
                h,
                l,
                v,
                s,
            })
    }

    proptest! {
        /// **Validates: Requirements 2.2, 9.1**
        ///
        /// Property 9: Formatter search output completeness — For any valid ForexTicker,
        /// the formatted search output contains the ticker symbol, name, and active status.
        #[test]
        fn prop_formatter_search_output_completeness(ticker in forex_ticker_strategy()) {
            let output = format_search_results(std::slice::from_ref(&ticker));
            prop_assert!(
                output.contains(&ticker.ticker),
                "Output missing ticker '{}' in:\n{}",
                ticker.ticker, output
            );
            prop_assert!(
                output.contains(&ticker.name),
                "Output missing name '{}' in:\n{}",
                ticker.name, output
            );
            let active_str = ticker.active.to_string();
            prop_assert!(
                output.contains(&active_str),
                "Output missing active '{}' in:\n{}",
                active_str, output
            );
        }

        /// **Validates: Requirements 3.2, 9.2, 9.7**
        ///
        /// Property 10: Formatter bars output completeness — For any valid AggBar,
        /// the formatted output contains timestamp, open, high, low, close, volume.
        #[test]
        fn prop_formatter_bars_output_completeness(bar in agg_bar_strategy()) {
            let output = format_bars(std::slice::from_ref(&bar));

            // Timestamp should be present as formatted ISO string
            let expected_ts = format_timestamp_ms(bar.t);
            prop_assert!(
                output.contains(&expected_ts),
                "Output missing timestamp '{}' in:\n{}",
                expected_ts, output
            );

            // Open, high, low, close formatted to 4dp
            let open_str = format!("{:.4}", bar.o);
            prop_assert!(
                output.contains(&open_str),
                "Output missing open '{}' in:\n{}",
                open_str, output
            );

            let high_str = format!("{:.4}", bar.h);
            prop_assert!(
                output.contains(&high_str),
                "Output missing high '{}' in:\n{}",
                high_str, output
            );

            let low_str = format!("{:.4}", bar.l);
            prop_assert!(
                output.contains(&low_str),
                "Output missing low '{}' in:\n{}",
                low_str, output
            );

            let close_str = format!("{:.4}", bar.c);
            prop_assert!(
                output.contains(&close_str),
                "Output missing close '{}' in:\n{}",
                close_str, output
            );

            // Volume formatted to 0dp
            let vol_str = format!("{:.0}", bar.v);
            prop_assert!(
                output.contains(&vol_str),
                "Output missing volume '{}' in:\n{}",
                vol_str, output
            );
        }

        /// **Validates: Requirements 4.2, 9.3**
        ///
        /// Property 11: Formatter quote output completeness — For any valid LastQuoteResponse,
        /// the formatted output contains symbol, bid, ask, and timestamp.
        #[test]
        fn prop_formatter_quote_output_completeness(resp in last_quote_response_strategy()) {
            let output = format_quote(&resp);

            prop_assert!(
                output.contains(&resp.symbol),
                "Output missing symbol '{}' in:\n{}",
                resp.symbol, output
            );

            let bid_str = format!("{:.4}", resp.last.bid);
            prop_assert!(
                output.contains(&bid_str),
                "Output missing bid '{}' in:\n{}",
                bid_str, output
            );

            let ask_str = format!("{:.4}", resp.last.ask);
            prop_assert!(
                output.contains(&ask_str),
                "Output missing ask '{}' in:\n{}",
                ask_str, output
            );

            let ts_str = format_timestamp_ms(resp.last.timestamp);
            prop_assert!(
                output.contains(&ts_str),
                "Output missing timestamp '{}' in:\n{}",
                ts_str, output
            );
        }

        /// **Validates: Requirements 5.3, 9.4**
        ///
        /// Property 12: Formatter conversion output completeness — For any valid ConversionResponse,
        /// the formatted output contains from, to, amount, converted, bid, and ask.
        #[test]
        fn prop_formatter_conversion_output_completeness(resp in conversion_response_strategy()) {
            let output = format_conversion(&resp);

            prop_assert!(
                output.contains(&resp.from),
                "Output missing from '{}' in:\n{}",
                resp.from, output
            );
            prop_assert!(
                output.contains(&resp.to),
                "Output missing to '{}' in:\n{}",
                resp.to, output
            );

            // initial_amount is printed with default f64 display
            let amount_str = resp.initial_amount.to_string();
            prop_assert!(
                output.contains(&amount_str),
                "Output missing amount '{}' in:\n{}",
                amount_str, output
            );

            // converted is printed with default f64 display
            let converted_str = resp.converted.to_string();
            prop_assert!(
                output.contains(&converted_str),
                "Output missing converted '{}' in:\n{}",
                converted_str, output
            );

            let bid_str = format!("{:.4}", resp.last.bid);
            prop_assert!(
                output.contains(&bid_str),
                "Output missing bid '{}' in:\n{}",
                bid_str, output
            );

            let ask_str = format!("{:.4}", resp.last.ask);
            prop_assert!(
                output.contains(&ask_str),
                "Output missing ask '{}' in:\n{}",
                ask_str, output
            );
        }

        /// **Validates: Requirements 6.2, 9.5**
        ///
        /// Property 13: Formatter snapshot output completeness — For any valid SnapshotTicker,
        /// the formatted output contains all expected section values: day OHLCV, prev day OHLCV,
        /// last quote (ask, bid, timestamp), minute OHLCV, and change values.
        #[test]
        fn prop_formatter_snapshot_output_completeness(ticker in snapshot_ticker_strategy()) {
            let output = format_snapshot(&ticker);

            // Ticker name
            prop_assert!(
                output.contains(&ticker.ticker),
                "Output missing ticker '{}' in:\n{}",
                ticker.ticker, output
            );

            // Day OHLCV
            prop_assert!(output.contains(&format!("{:.4}", ticker.day.o)), "Missing day open");
            prop_assert!(output.contains(&format!("{:.4}", ticker.day.h)), "Missing day high");
            prop_assert!(output.contains(&format!("{:.4}", ticker.day.l)), "Missing day low");
            prop_assert!(output.contains(&format!("{:.4}", ticker.day.c)), "Missing day close");
            prop_assert!(output.contains(&format!("{:.0}", ticker.day.v)), "Missing day volume");

            // Prev Day OHLCV
            prop_assert!(output.contains(&format!("{:.4}", ticker.prev_day.o)), "Missing prev_day open");
            prop_assert!(output.contains(&format!("{:.4}", ticker.prev_day.h)), "Missing prev_day high");
            prop_assert!(output.contains(&format!("{:.4}", ticker.prev_day.l)), "Missing prev_day low");
            prop_assert!(output.contains(&format!("{:.4}", ticker.prev_day.c)), "Missing prev_day close");
            prop_assert!(output.contains(&format!("{:.0}", ticker.prev_day.v)), "Missing prev_day volume");

            // Last Quote
            prop_assert!(output.contains(&format!("{:.4}", ticker.last_quote.a)), "Missing last_quote ask");
            prop_assert!(output.contains(&format!("{:.4}", ticker.last_quote.b)), "Missing last_quote bid");
            let quote_ts = format_timestamp_ms(ticker.last_quote.t);
            prop_assert!(output.contains(&quote_ts), "Missing last_quote timestamp");

            // Minute OHLCV
            prop_assert!(output.contains(&format!("{:.4}", ticker.min.o)), "Missing min open");
            prop_assert!(output.contains(&format!("{:.4}", ticker.min.h)), "Missing min high");
            prop_assert!(output.contains(&format!("{:.4}", ticker.min.l)), "Missing min low");
            prop_assert!(output.contains(&format!("{:.4}", ticker.min.c)), "Missing min close");
            prop_assert!(output.contains(&format!("{:.0}", ticker.min.v)), "Missing min volume");

            // Today's Change
            prop_assert!(output.contains(&format!("{:.4}", ticker.todays_change)), "Missing todays_change");
            prop_assert!(output.contains(&format!("{:.4}", ticker.todays_change_perc)), "Missing todays_change_perc");
        }

        /// **Validates: Requirements 9.6, 9.7**
        ///
        /// Property 14: Formatter WebSocket minute line completeness — For any valid WsMinuteAgg,
        /// the formatted line contains the pair, HH:MM time, open, high, low, close, and volume.
        #[test]
        fn prop_formatter_ws_minute_output_completeness(agg in ws_minute_agg_strategy()) {
            let output = format_ws_minute(&agg);

            // Pair
            prop_assert!(
                output.contains(&agg.pair),
                "Output missing pair '{}' in:\n{}",
                agg.pair, output
            );

            // HH:MM time derived from start timestamp
            let dt = DateTime::from_timestamp_millis(agg.s as i64).unwrap_or_default();
            let time_str = dt.format("%H:%M").to_string();
            prop_assert!(
                output.contains(&time_str),
                "Output missing time '{}' in:\n{}",
                time_str, output
            );

            // OHLCV values (default f64 display for prices, u64 for volume)
            let o_str = agg.o.to_string();
            prop_assert!(
                output.contains(&o_str),
                "Output missing open '{}' in:\n{}",
                o_str, output
            );

            let h_str = agg.h.to_string();
            prop_assert!(
                output.contains(&h_str),
                "Output missing high '{}' in:\n{}",
                h_str, output
            );

            let l_str = agg.l.to_string();
            prop_assert!(
                output.contains(&l_str),
                "Output missing low '{}' in:\n{}",
                l_str, output
            );

            let c_str = agg.c.to_string();
            prop_assert!(
                output.contains(&c_str),
                "Output missing close '{}' in:\n{}",
                c_str, output
            );

            let v_str = agg.v.to_string();
            prop_assert!(
                output.contains(&v_str),
                "Output missing volume '{}' in:\n{}",
                v_str, output
            );
        }
    }
}
