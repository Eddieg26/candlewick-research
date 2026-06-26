// WebSocket client for Tiingo Crypto real-time data

use std::collections::HashMap;
use std::time::Duration;

use chrono::DateTime;
use futures_util::{SinkExt, StreamExt};
use tokio::time;
use tokio_tungstenite::tungstenite::Message;

use crate::error::TiingoError;
use crate::format::print_live_minute;

/// A single minute's worth of aggregated trade data.
#[derive(Debug, Clone)]
pub struct MinuteBucket {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub trade_count: u32,
}

/// A completed minute aggregation ready for output.
#[derive(Debug, Clone)]
pub struct CompletedMinute {
    pub symbol: String,
    pub minute_key: u64,
    pub bucket: MinuteBucket,
}

/// Aggregates trades into per-symbol, per-minute OHLCV buckets.
pub struct MinuteAggregator {
    buckets: HashMap<String, HashMap<u64, MinuteBucket>>,
}

impl MinuteAggregator {
    pub fn new() -> Self {
        Self {
            buckets: HashMap::new(),
        }
    }

    /// Record a trade into the appropriate minute bucket.
    /// The minute key is derived as `timestamp_ms / 60_000`.
    pub fn record_trade(&mut self, symbol: &str, price: f64, volume: f64, timestamp_ms: u64) {
        let minute_key = timestamp_ms / 60_000;

        let symbol_buckets = self.buckets.entry(symbol.to_string()).or_default();
        let bucket = symbol_buckets.entry(minute_key).or_insert(MinuteBucket {
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 0.0,
            trade_count: 0,
        });

        // Update OHLCV fields
        if price > bucket.high {
            bucket.high = price;
        }
        if price < bucket.low {
            bucket.low = price;
        }
        bucket.close = price;
        bucket.volume += volume;
        bucket.trade_count += 1;
    }

    /// Drain and return all completed minute buckets for minutes
    /// strictly before `current_minute_key`.
    pub fn drain_completed(&mut self, current_minute_key: u64) -> Vec<CompletedMinute> {
        let mut completed = Vec::new();

        for (symbol, symbol_buckets) in self.buckets.iter_mut() {
            let keys_to_drain: Vec<u64> = symbol_buckets
                .keys()
                .filter(|&&k| k < current_minute_key)
                .copied()
                .collect();

            for key in keys_to_drain {
                if let Some(bucket) = symbol_buckets.remove(&key) {
                    completed.push(CompletedMinute {
                        symbol: symbol.clone(),
                        minute_key: key,
                        bucket,
                    });
                }
            }
        }

        completed
    }
}

/// WebSocket live-stream handler that connects to Tiingo Crypto,
/// authenticates, subscribes to symbols, and aggregates trades into per-minute OHLCV buckets.
pub struct LiveStream {
    token: String,
    symbols: Vec<String>,
}

impl LiveStream {
    pub fn new(token: String, symbols: Vec<String>) -> Self {
        Self { token, symbols }
    }

    /// Connect, authenticate, subscribe, and run the aggregation loop.
    /// Returns when Ctrl+C is received or connection drops.
    pub async fn run(&self) -> Result<(), TiingoError> {
        let url = "wss://api.tiingo.com/crypto";

        // Connect with 10-second timeout
        let ws_stream = match time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async(url),
        )
        .await
        {
            Ok(Ok((stream, _response))) => stream,
            Ok(Err(e)) => {
                return Err(TiingoError::WebSocketConnect(e.to_string()));
            }
            Err(_elapsed) => {
                return Err(TiingoError::WebSocketConnect(
                    "connection timed out after 10 seconds".to_string(),
                ));
            }
        };

        println!("Connected: {url}");

        let (mut write, mut read) = ws_stream.split();

        // Send auth + subscribe message
        let subscribe_msg = serde_json::json!({
            "eventName": "subscribe",
            "authorization": self.token,
            "eventData": {
                "thresholdLevel": 2,
                "tickers": self.symbols
            }
        });
        write
            .send(Message::Text(subscribe_msg.to_string().into()))
            .await
            .map_err(|e| TiingoError::WebSocketConnect(e.to_string()))?;

        let mut aggregator = MinuteAggregator::new();
        let mut interval = time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_message(&text, &mut aggregator);
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            return Err(TiingoError::WebSocketDisconnect(
                                "connection closed by server".to_string(),
                            ));
                        }
                        Some(Err(e)) => {
                            return Err(TiingoError::WebSocketDisconnect(e.to_string()));
                        }
                        // Binary, Ping, Pong — silently discard
                        Some(Ok(_)) => {}
                    }
                }
                _ = interval.tick() => {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    let current_minute_key = now_ms / 60_000;

                    let completed = aggregator.drain_completed(current_minute_key);
                    for minute in &completed {
                        print_live_minute(minute);
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    // Graceful shutdown: close WebSocket and exit
                    let _ = write.close().await;
                    return Ok(());
                }
            }
        }
    }

    /// Parse and handle a single Tiingo Crypto WebSocket message.
    ///
    /// Tiingo crypto sends JSON with format:
    /// ```json
    /// {
    ///     "messageType": "A",
    ///     "service": "crypto_data",
    ///     "data": ["Q", "btcusd", "2019-01-30T18:03:40+00:00", "bitfinex",
    ///              38.11162867, 787.82, 787.83, 42.4153887, 787.84]
    /// }
    /// ```
    ///
    /// Data array indices:
    ///   0: update type ("T" = trade, "Q" = top-of-book quote)
    ///   1: ticker (e.g. "btcusd")
    ///   2: ISO 8601 datetime string
    ///   3: exchange name (string)
    ///   4: lastSize (volume of the last trade in base currency)
    ///   5: lastPrice (last trade price)
    ///
    /// Both "T" (trade) and "Q" (quote with last-sale info) messages are
    /// processed for aggregation using lastPrice (index 5) and lastSize (index 4).
    fn handle_message(&self, text: &str, aggregator: &mut MinuteAggregator) {
        let parsed: serde_json::Value = match serde_json::from_str(text) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("warning: failed to parse WebSocket message: {e}");
                return;
            }
        };

        // Only process data messages (messageType == "A")
        let message_type = match parsed.get("messageType").and_then(|v| v.as_str()) {
            Some(mt) => mt,
            None => return, // Not a data message (e.g. heartbeat), skip silently
        };

        if message_type != "A" {
            return;
        }

        // Extract the data array
        let data = match parsed.get("data").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => {
                eprintln!("warning: message has messageType 'A' but no data array");
                return;
            }
        };

        // data[0] is the update type: "T" (trade) or "Q" (quote)
        let update_type = match data.first().and_then(|v| v.as_str()) {
            Some(t) => t,
            None => {
                eprintln!("warning: data array missing type field at index 0");
                return;
            }
        };

        // Process both trade ("T") and quote ("Q") messages for price aggregation
        if update_type != "T" && update_type != "Q" {
            return;
        }

        // data[1]: ticker
        let ticker = match data.get(1).and_then(|v| v.as_str()) {
            Some(t) => t,
            None => {
                eprintln!("warning: crypto message missing ticker at index 1");
                return;
            }
        };

        // data[2]: ISO 8601 datetime — parse to get timestamp in ms
        let datetime_str = match data.get(2).and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                eprintln!("warning: crypto message missing datetime at index 2");
                return;
            }
        };

        let timestamp_ms = match DateTime::parse_from_rfc3339(datetime_str) {
            Ok(dt) => dt.timestamp_millis() as u64,
            Err(_) => {
                // Try a more lenient parse with fractional seconds
                match chrono::DateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M:%S%.f%:z") {
                    Ok(dt) => dt.timestamp_millis() as u64,
                    Err(e) => {
                        eprintln!("warning: could not parse datetime '{datetime_str}': {e}");
                        return;
                    }
                }
            }
        };

        // data[4]: lastSize (volume in base currency)
        let last_size = match data.get(4).and_then(|v| v.as_f64()) {
            Some(s) => s,
            None => {
                eprintln!("warning: crypto message missing lastSize at index 4");
                return;
            }
        };

        // data[5]: lastPrice
        let last_price = match data.get(5).and_then(|v| v.as_f64()) {
            Some(p) => p,
            None => {
                eprintln!("warning: crypto message missing lastPrice at index 5");
                return;
            }
        };

        aggregator.record_trade(ticker, last_price, last_size, timestamp_ms);
    }
}
