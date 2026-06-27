// WebSocket client for Finage real-time trade streaming

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::time;
use tokio_tungstenite::tungstenite::Message;

use crate::error::FinageError;
use crate::format::print_trade;
use crate::models::WsTrade;

/// WebSocket live-stream handler that connects to Finage,
/// subscribes to symbols, and prints individual trade events.
pub struct LiveStream {
    token: String,
    symbols: Vec<String>,
}

impl LiveStream {
    pub fn new(token: String, symbols: Vec<String>) -> Self {
        Self { token, symbols }
    }

    /// Connect to Finage WebSocket, subscribe to symbols, and print trade lines.
    /// Runs until Ctrl+C or server disconnect.
    pub async fn run(&self) -> Result<(), FinageError> {
        let url = format!("wss://stream.finage.co.uk?token={}", self.token);

        // Connect with 10-second timeout
        let ws_stream = match time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async(&url),
        )
        .await
        {
            Ok(Ok((stream, _response))) => stream,
            Ok(Err(e)) => {
                return Err(FinageError::WebSocketConnect(e.to_string()));
            }
            Err(_elapsed) => {
                return Err(FinageError::WebSocketConnect(
                    "connection timed out".to_string(),
                ));
            }
        };

        let (mut write, mut read) = ws_stream.split();

        // Send subscribe message for all symbols
        let sub_msg = serde_json::json!({
            "action": "subscribe",
            "symbols": self.symbols.join(",")
        });
        write
            .send(Message::Text(sub_msg.to_string().into()))
            .await
            .map_err(|e| FinageError::WebSocketConnect(e.to_string()))?;

        // Message loop with graceful Ctrl+C shutdown
        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_message(&text);
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            return Err(FinageError::WebSocketDisconnect(
                                "connection closed by server".to_string(),
                            ));
                        }
                        Some(Err(e)) => {
                            return Err(FinageError::WebSocketDisconnect(e.to_string()));
                        }
                        // Binary, Ping, Pong — silently discard
                        Some(Ok(_)) => {}
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

    /// Handle a single text message from the WebSocket.
    /// - If it parses as a WsTrade, print the trade line.
    /// - If it's valid JSON but not a trade, silently discard.
    /// - If it's not valid JSON at all, print a warning to stderr.
    fn handle_message(&self, text: &str) {
        match serde_json::from_str::<WsTrade>(text) {
            Ok(trade) => {
                print_trade(&trade.s, trade.p, trade.t);
            }
            Err(_) => {
                // Not a trade — check if it's valid JSON (non-trade event like subscribe confirmation)
                if serde_json::from_str::<serde_json::Value>(text).is_err() {
                    eprintln!("warning: failed to parse WebSocket message as JSON: {text}");
                }
                // Valid JSON but not a trade → silently discard
            }
        }
    }
}
