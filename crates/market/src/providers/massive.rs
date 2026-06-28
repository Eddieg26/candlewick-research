use crate::{
    Asset, Candle, Market, ResponseExt, Ticker, Timeframe,
    error::MarketError,
    providers::{LiveStream, MarketDataProvider, TickerUpdate},
};
use chrono::{DateTime, Duration, Utc};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::borrow::Cow;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

pub struct MassiveProvider {
    http: reqwest::Client,
    api_key: String,
}

impl MassiveProvider {
    pub fn ticker<'a>(ticker: &'a Ticker, market: Market) -> Cow<'a, Ticker> {
        match market {
            Market::Forex => Cow::Owned(ticker.prefix("C:")),
            Market::Stock => Cow::Borrowed(ticker),
            Market::Crypto => Cow::Borrowed(ticker),
        }
    }
}

impl MarketDataProvider for MassiveProvider {
    type LiveStream = MassiveLiveStream;

    async fn search(
        &self,
        ticker: &crate::Ticker,
        market: Market,
    ) -> Result<Option<crate::Asset>, crate::error::MarketError> {
        let ticker = Self::ticker(ticker, market);

        let url = format!(
            "{}/v3/reference/tickers/{}",
            Self::BASE_URL,
            ticker.as_str()
        );

        let market = match market {
            Market::Forex => "fx",
            Market::Stock => "stocks",
            Market::Crypto => "crypto",
        };

        let response = self
            .http
            .get(&url)
            .query(&[
                ("active", "true"),
                ("market", market),
                ("apiKey", &self.api_key),
            ])
            .send()
            .await?
            .check()
            .await?;

        let result = response.json::<MassiveSearchResult>().await?.results;

        Ok(Some(Asset::from(result)))
    }

    async fn candles(
        &self,
        ticker: &Ticker,
        market: Market,
        timeframe: Timeframe,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<crate::Candle>, crate::error::MarketError> {
        let ticker = Self::ticker(ticker, market);

        let (multiplier, timespan) = match timeframe {
            Timeframe::M1 => (1, "minute"),
            Timeframe::M5 => (5, "minute"),
            Timeframe::M15 => (15, "minute"),
            Timeframe::H1 => (1, "hour"),
            Timeframe::H4 => (4, "hour"),
            Timeframe::D1 => (1, "day"),
        };

        let from = from.format("%Y-%m-%d").to_string();
        let to = to.format("%Y-%m-%d").to_string();

        let url = format!(
            "{}/v2/aggs/ticker/{}/range/{}/{}/{}/{}",
            Self::BASE_URL,
            ticker.as_str(),
            multiplier,
            timespan,
            from,
            to,
        );

        let response = self
            .http
            .get(&url)
            .query(&[("apiKey", &self.api_key)])
            .send()
            .await?
            .check()
            .await?;

        let result = response.json::<MassiveCandleResult>().await?;

        Ok(result.results.iter().map(|r| r.candle(timeframe)).collect())
    }

    async fn stream(
        &self,
        market: crate::Market,
        tickers: &[crate::Ticker],
        subscriber: impl Fn(Vec<super::TickerUpdate>) + Send + 'static,
    ) -> Result<Self::LiveStream, crate::error::MarketError> {
        MassiveLiveStream::new(&self.api_key, market, tickers, subscriber).await
    }
}

#[derive(Deserialize)]
pub struct MassiveSearchResult {
    pub request_id: String,
    pub results: MassiveTickerSearchResult,
}

#[derive(Deserialize)]
pub struct MassiveTickerSearchResult {
    name: String,
    ticker: Ticker,
    market: String,
}

impl From<MassiveTickerSearchResult> for Asset {
    fn from(value: MassiveTickerSearchResult) -> Self {
        let market = match value.market.as_str() {
            "stocks" => Market::Stock,
            "fx" => Market::Forex,
            "crypto" => Market::Crypto,
            _ => panic!("unknown asset type: {}", value.market),
        };

        Self {
            ticker: value.ticker,
            name: value.name,
            market,
        }
    }
}

#[derive(Deserialize)]
pub struct MassiveCandleResult {
    pub request_id: String,
    pub results: Vec<MassiveTickerCandleResult>,
}

#[derive(Deserialize, Clone, Copy)]
pub struct MassiveTickerCandleResult {
    pub c: f64,
    pub h: f64,
    pub l: f64,
    pub o: f64,
    pub t: i64,
}

impl MassiveTickerCandleResult {
    pub fn candle(self, timeframe: Timeframe) -> Candle {
        let start = DateTime::<Utc>::from_timestamp_millis(self.t).unwrap();
        let end = start + Duration::seconds(timeframe.secs());
        Candle {
            open: self.o,
            high: self.h,
            low: self.l,
            close: self.c,
            start,
            end,
        }
    }
}

impl MassiveProvider {
    pub const BASE_URL: &'static str = "https://api.massive.com/";
}

pub struct MassiveLiveStream {
    _handle: JoinHandle<()>,
}

impl MassiveLiveStream {
    pub const BASE_URL: &'static str = "wss://socket.massive.com/";

    pub async fn new(
        api_key: &str,
        market: Market,
        tickers: &[Ticker],
        subscriber: impl Fn(Vec<TickerUpdate>) + Send + 'static,
    ) -> Result<MassiveLiveStream, MarketError> {
        let url = match market {
            Market::Forex => format!("{}/forex", Self::BASE_URL),
            Market::Stock => format!("{}/stocks", Self::BASE_URL),
            Market::Crypto => format!("{}/crypto", Self::BASE_URL),
        };

        let (stream, _) = tokio_tungstenite::connect_async(&url).await?;

        let (mut writer, mut reader) = stream.split();

        let auth_msg = serde_json::json!({
            "action": "auth",
            "params": api_key,
        });

        writer
            .send(Message::Text(auth_msg.to_string().into()))
            .await?;

        let prefix = match market {
            Market::Forex => "CA.",
            Market::Stock => "AM.",
            Market::Crypto => "XA.",
        };

        let tickers = tickers
            .iter()
            .map(|t| MassiveProvider::ticker(t, market).prefix(prefix))
            .collect::<Vec<_>>();

        let subscribe_msg = serde_json::json!({
            "action": "subscribe",
            "params": &tickers,
        });

        writer
            .send(Message::Text(subscribe_msg.to_string().into()))
            .await?;

        let handle = tokio::spawn(async move {
            loop {
                let message = match reader.next().await {
                    Some(Ok(message)) => message,
                    Some(Err(_)) => {
                        break;
                    }
                    None => continue,
                };

                let Message::Text(text) = message else {
                    continue;
                };

                let event = match serde_json::from_str::<WsEvent>(&text) {
                    Ok(event) => event,
                    Err(_) => {
                        break;
                    }
                };

                if let Some(update) = event.into(Timeframe::M1) {
                    subscriber(vec![update])
                }
            }
        });

        Ok(Self { _handle: handle })
    }
}

pub enum WsEvent {
    Status {
        ev: String,
        status: String,
        message: String,
    },
    ForexUpdate {
        ev: String,
        pair: String,
        o: f64,
        c: f64,
        h: f64,
        l: f64,
        s: i64,
    },
    StockUpdate {
        ev: String,
        sym: String,
        o: f64,
        c: f64,
        h: f64,
        l: f64,
        s: i64,
        e: i64,
    },
    CryptoUpdate {
        ev: String,
        pair: String,
        o: f64,
        c: f64,
        h: f64,
        l: f64,
        s: i64,
        e: i64,
    },
}

impl WsEvent {
    pub fn into(self, timeframe: Timeframe) -> Option<TickerUpdate> {
        match self {
            WsEvent::ForexUpdate {
                pair,
                o,
                c,
                h,
                l,
                s,
                ..
            } => {
                let start = DateTime::<Utc>::from_timestamp_millis(s).unwrap();
                let end = start + Duration::seconds(timeframe.secs());
                let ticker = Ticker::from(pair);
                Some(TickerUpdate {
                    ticker,
                    candle: Candle {
                        open: o,
                        high: h,
                        low: l,
                        close: c,
                        start,
                        end,
                    },
                })
            }
            WsEvent::StockUpdate {
                sym,
                o,
                c,
                h,
                l,
                s,
                e,
                ..
            } => Some(TickerUpdate {
                ticker: Ticker::from(sym),
                candle: Candle {
                    open: o,
                    high: h,
                    low: l,
                    close: c,
                    start: DateTime::<Utc>::from_timestamp_millis(s).unwrap(),
                    end: DateTime::<Utc>::from_timestamp_millis(e).unwrap(),
                },
            }),
            WsEvent::CryptoUpdate {
                pair,
                o,
                c,
                h,
                l,
                s,
                e,
                ..
            } => {
                let start = DateTime::<Utc>::from_timestamp_millis(s).unwrap();
                let end = DateTime::<Utc>::from_timestamp_millis(e).unwrap();
                let ticker = Ticker::from(pair);
                Some(TickerUpdate {
                    ticker,
                    candle: Candle {
                        open: o,
                        high: h,
                        low: l,
                        close: c,
                        start,
                        end,
                    },
                })
            }
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for WsEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let raw: serde_json::Value = serde_json::Value::deserialize(deserializer)?;

        let ev = raw
            .get("ev")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        match ev {
            "status" => {
                let status = raw
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let message = raw
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                Ok(WsEvent::Status {
                    ev: ev.to_string(),
                    status,
                    message,
                })
            }
            "CA" => {
                let pair = raw
                    .get("pair")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| D::Error::missing_field("pair"))?
                    .to_string();
                let o = raw.get("o").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let c = raw.get("c").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let h = raw.get("h").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let l = raw.get("l").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let s = raw.get("s").and_then(|v| v.as_i64()).unwrap_or(0);

                Ok(WsEvent::ForexUpdate {
                    ev: ev.to_string(),
                    pair,
                    o,
                    c,
                    h,
                    l,
                    s,
                })
            }
            "AM" => {
                let sym = raw
                    .get("sym")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| D::Error::missing_field("sym"))?
                    .to_string();
                let o = raw.get("o").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let c = raw.get("c").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let h = raw.get("h").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let l = raw.get("l").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let s = raw.get("s").and_then(|v| v.as_i64()).unwrap_or(0);
                let e = raw.get("e").and_then(|v| v.as_i64()).unwrap_or(0);

                Ok(WsEvent::StockUpdate {
                    ev: ev.to_string(),
                    sym,
                    o,
                    c,
                    h,
                    l,
                    s,
                    e,
                })
            }
            "XA" => {
                let pair = raw
                    .get("pair")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| D::Error::missing_field("pair"))?
                    .to_string();
                let o = raw.get("o").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let c = raw.get("c").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let h = raw.get("h").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let l = raw.get("l").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let s = raw.get("s").and_then(|v| v.as_i64()).unwrap_or(0);
                let e = raw.get("e").and_then(|v| v.as_i64()).unwrap_or(0);

                Ok(WsEvent::CryptoUpdate {
                    ev: ev.to_string(),
                    pair,
                    o,
                    c,
                    h,
                    l,
                    s,
                    e,
                })
            }
            other => Err(D::Error::custom(format!("unknown event type: '{}'", other))),
        }
    }
}

impl LiveStream for MassiveLiveStream {
    async fn unsubscribe(&self) -> Result<(), crate::error::MarketError> {
        todo!()
    }
}
