// WebSocket client for Finnhub real-time trades

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::time;
use tokio_tungstenite::Connector;
use tokio_tungstenite::tungstenite::Message;

use crate::error::FinnhubError;
use crate::format::print_live_minute;
use crate::models::WsMessage;

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

/// WebSocket live-stream handler that connects to Finnhub,
/// subscribes to symbols, and aggregates trades into per-minute OHLCV buckets.
pub struct LiveStream {
    token: String,
    symbols: Vec<String>,
}

impl LiveStream {
    pub fn new(token: String, symbols: Vec<String>) -> Self {
        Self { token, symbols }
    }

    /// Connect, subscribe, and run the aggregation loop.
    /// Returns when Ctrl+C is received or connection drops.
    pub async fn run(&self) -> Result<(), FinnhubError> {
        let url = format!("wss://ws.finnhub.io/?token={}", self.token);

        // Connect with 10-second timeout
        let ws_stream = match time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async(&url),
        )
        .await
        {
            Ok(Ok((stream, _response))) => stream,
            Ok(Err(e)) => {
                return Err(FinnhubError::WebSocketConnect(e.to_string()));
            }
            Err(_elapsed) => {
                return Err(FinnhubError::WebSocketConnect(
                    "connection timed out after 10 seconds".to_string(),
                ));
            }
        };
        
        println!("Connected: {url}");

        let (mut write, mut read) = ws_stream.split();

        // Send subscription messages for each symbol
        for symbol in &self.symbols {
            let sub_msg = serde_json::json!({
                "type": "subscribe",
                "symbol": symbol
            });
            write
                .send(Message::Text(sub_msg.to_string().into()))
                .await
                .map_err(|e| FinnhubError::WebSocketConnect(e.to_string()))?;
        }

        let mut aggregator = MinuteAggregator::new();
        let mut interval = time::interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            match serde_json::from_str::<WsMessage>(&text) {
                                Ok(ws_msg) => {
                                    if ws_msg.msg_type == "trade" {
                                        for trade in &ws_msg.data {
                                            aggregator.record_trade(
                                                &trade.s,
                                                trade.p,
                                                trade.v,
                                                trade.t,
                                            );
                                        }
                                    }
                                    // Non-trade messages are silently discarded
                                }
                                Err(e) => {
                                    eprintln!("warning: malformed WebSocket message: {e}");
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            return Err(FinnhubError::WebSocketDisconnect(
                                "connection closed by server".to_string(),
                            ));
                        }
                        Some(Err(e)) => {
                            return Err(FinnhubError::WebSocketDisconnect(e.to_string()));
                        }
                        // Binary, Ping, Pong, Frame — silently discard
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
                    // Graceful shutdown: close WebSocket and exit with code 0
                    let _ = write.close().await;
                    return Ok(());
                }
            }
        }
    }
}
