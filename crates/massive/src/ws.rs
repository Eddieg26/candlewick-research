// WebSocket client for Massive Forex real-time streaming

use std::time::Duration;
use futures_util::{SinkExt, StreamExt};
use tokio::time;
use tokio_tungstenite::tungstenite::Message;

use crate::error::MassiveError;
use crate::format::print_ws_minute;
use crate::models::WsMinuteAgg;

pub struct LiveStream {
    api_key: String,
    pairs: Vec<String>,
}

/// Format subscribe params: prefix each pair with "CA." and join with commas.
/// E.g., ["C:EUR-USD", "C:GBP-USD"] → "CA.C:EUR-USD,CA.C:GBP-USD"
pub(crate) fn format_subscribe_params(pairs: &[String]) -> String {
    pairs.iter()
        .map(|p| format!("CA.{p}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Parse a WebSocket message and return the CA (minute aggregate) events it contains.
/// Non-CA events are silently discarded. Returns an error only if the JSON is malformed.
pub(crate) fn route_message(text: &str) -> Result<Vec<WsMinuteAgg>, serde_json::Error> {
    let parsed: serde_json::Value = serde_json::from_str(text)?;

    let events = if parsed.is_array() {
        parsed.as_array().unwrap().clone()
    } else {
        vec![parsed]
    };

    let mut results = Vec::new();
    for event in &events {
        let ev_type = event.get("ev").and_then(|v| v.as_str()).unwrap_or("");
        if ev_type == "CA" {
            if let Ok(agg) = serde_json::from_value::<WsMinuteAgg>(event.clone()) {
                results.push(agg);
            }
        }
    }
    Ok(results)
}

impl LiveStream {
    pub fn new(api_key: String, pairs: Vec<String>) -> Self {
        Self { api_key, pairs }
    }

    /// Connect, authenticate, subscribe, and print minute aggregates.
    /// Returns when Ctrl+C is received or connection drops.
    pub async fn run(&self) -> Result<(), MassiveError> {
        let url = "wss://socket.massive.com/forex";

        // Connect with 10-second timeout
        let ws_stream = match time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async(url),
        ).await {
            Ok(Ok((stream, _))) => stream,
            Ok(Err(e)) => return Err(MassiveError::WebSocketConnect(e.to_string())),
            Err(_) => return Err(MassiveError::WebSocketConnect(
                "connection timed out after 10 seconds".to_string(),
            )),
        };

        println!("Connected: {url}");
        let (mut write, mut read) = ws_stream.split();

        // Step 1: Authenticate
        let auth_msg = serde_json::json!({
            "action": "auth",
            "params": self.api_key
        });
        write.send(Message::Text(auth_msg.to_string().into())).await
            .map_err(|e| MassiveError::WebSocketConnect(e.to_string()))?;

        // Step 2: Subscribe to minute aggregates for all pairs
        let params = format_subscribe_params(&self.pairs);
        let sub_msg = serde_json::json!({
            "action": "subscribe",
            "params": params
        });
        write.send(Message::Text(sub_msg.to_string().into())).await
            .map_err(|e| MassiveError::WebSocketConnect(e.to_string()))?;

        // Step 3: Read and print minute aggregates
        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            println!("MASSIVE: {}", text);
                            self.handle_message(&text);
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            return Err(MassiveError::WebSocketDisconnect(
                                "connection closed by server".to_string(),
                            ));
                        }
                        Some(Err(e)) => {
                            return Err(MassiveError::WebSocketDisconnect(e.to_string()));
                        }
                        Some(Ok(_)) => {} // Binary, Ping, Pong — discard
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    let _ = write.close().await;
                    return Ok(());
                }
            }
        }
    }

    fn handle_message(&self, text: &str) {
        match route_message(text) {
            Ok(aggs) => {
                for agg in &aggs {
                    print_ws_minute(agg);
                }
            }
            Err(e) => eprintln!("warning: failed to parse WebSocket message: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy that generates non-empty pair strings (like "C:EUR-USD").
    fn pair_string() -> impl Strategy<Value = String> {
        "[A-Za-z0-9:_\\-]{1,20}"
    }

    /// Strategy that generates a non-empty Vec of pair strings.
    fn pairs_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(pair_string(), 1..=20)
    }

    /// Strategy for generating valid CA event JSON objects.
    fn ca_event_json() -> impl Strategy<Value = String> {
        (
            "[A-Z]{3}/[A-Z]{3}",
            0.0001f64..10000.0f64,
            0.0001f64..10000.0f64,
            0.0001f64..10000.0f64,
            0.0001f64..10000.0f64,
            1u64..=1_000_000u64,
            1_000_000_000_000u64..=2_000_000_000_000u64,
        )
            .prop_map(|(pair, o, h, l, c, v, s)| {
                serde_json::json!({
                    "ev": "CA",
                    "pair": pair,
                    "o": o,
                    "c": c,
                    "h": h,
                    "l": l,
                    "v": v,
                    "s": s
                })
                .to_string()
            })
    }

    /// Strategy for generating non-CA event type strings.
    fn non_ca_ev_type() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("status".to_string()),
            Just("AM".to_string()),
            Just("A".to_string()),
            Just("T".to_string()),
            Just("Q".to_string()),
            "[a-zA-Z]{1,5}".prop_filter("must not be CA", |s| s != "CA"),
        ]
    }

    /// Strategy for generating non-CA event JSON objects.
    fn non_ca_event_json() -> impl Strategy<Value = String> {
        non_ca_ev_type().prop_map(|ev| {
            serde_json::json!({
                "ev": ev,
                "status": "connected",
                "message": "some message"
            })
            .to_string()
        })
    }

    proptest! {
        /// **Validates: Requirements 7.3**
        ///
        /// Property 15: WebSocket subscribe params formatting — For any non-empty
        /// list of pairs, the params field is comma-separated with each pair
        /// prefixed by "CA."
        #[test]
        fn prop_ws_subscribe_params_formatting(pairs in pairs_strategy()) {
            let result = format_subscribe_params(&pairs);

            let segments: Vec<&str> = result.split(',').collect();
            prop_assert_eq!(
                segments.len(),
                pairs.len(),
                "Expected {} segments but got {} for result: {:?}",
                pairs.len(), segments.len(), result
            );

            for (i, segment) in segments.iter().enumerate() {
                prop_assert!(
                    segment.starts_with("CA."),
                    "Segment {} ({:?}) does not start with 'CA.'",
                    i, segment
                );
            }

            for pair in &pairs {
                let expected = format!("CA.{pair}");
                prop_assert!(
                    result.contains(&expected),
                    "Result {:?} does not contain expected {:?}",
                    result, expected
                );
            }
        }

        /// **Validates: Requirements 7.4, 7.5**
        ///
        /// Property 8: WebSocket minute aggregate pass-through — Messages with ev="CA"
        /// produce output; messages with other ev values produce no output.
        /// Sub-property: A single CA event produces exactly one routed result.
        #[test]
        fn prop_ca_event_produces_output(json in ca_event_json()) {
            let result = route_message(&json).unwrap();
            prop_assert!(
                !result.is_empty(),
                "CA event should produce output, got empty Vec for input: {}",
                json
            );
            prop_assert_eq!(
                result.len(),
                1,
                "Single CA event should produce exactly one result, got {} for input: {}",
                result.len(),
                json
            );
        }

        /// **Validates: Requirements 7.4, 7.5**
        ///
        /// Property 8: WebSocket minute aggregate pass-through — Messages with ev="CA"
        /// produce output; messages with other ev values produce no output.
        /// Sub-property: A non-CA event produces no routed results.
        #[test]
        fn prop_non_ca_event_produces_no_output(json in non_ca_event_json()) {
            let result = route_message(&json).unwrap();
            prop_assert!(
                result.is_empty(),
                "Non-CA event should produce no output, got {} results for input: {}",
                result.len(),
                json
            );
        }

        /// **Validates: Requirements 7.4, 7.5**
        ///
        /// Property 8: WebSocket minute aggregate pass-through — Messages with ev="CA"
        /// produce output; messages with other ev values produce no output.
        /// Sub-property: Mixed arrays return only CA events.
        #[test]
        fn prop_mixed_array_routes_only_ca_events(
            ca_events in prop::collection::vec(ca_event_json(), 1..5),
            non_ca_events in prop::collection::vec(non_ca_event_json(), 1..5),
        ) {
            let mut all_events: Vec<serde_json::Value> = Vec::new();
            for json in &ca_events {
                all_events.push(serde_json::from_str(json).unwrap());
            }
            for json in &non_ca_events {
                all_events.push(serde_json::from_str(json).unwrap());
            }

            let array_json = serde_json::to_string(&all_events).unwrap();
            let result = route_message(&array_json).unwrap();

            prop_assert_eq!(
                result.len(),
                ca_events.len(),
                "Expected {} CA events routed from mixed array of {} total, got {}",
                ca_events.len(),
                all_events.len(),
                result.len()
            );
        }
    }
}
