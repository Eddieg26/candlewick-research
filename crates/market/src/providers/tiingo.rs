use crate::{
    Asset, Candle, Market, ResponseExt, Ticker, Timeframe,
    aggregator::CandleAggregator,
    error::MarketError,
    providers::{LiveStream, MarketDataProvider, TickerUpdate},
};
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

pub struct TiingoProvider {
    http: reqwest::Client,
    api_key: String,
}

impl MarketDataProvider for TiingoProvider {
    type LiveStream = TiingoLiveStream;

    async fn search(&self, ticker: &Ticker, _: Market) -> Result<Option<Asset>, MarketError> {
        let response = self
            .http
            .get("https://api.tiingo.com/tiingo/utilities/search")
            .query(&[
                ("query", ticker.as_str()),
                ("exactTickerMatch", "true"),
                ("token", &self.api_key),
            ])
            .send()
            .await?
            .check()
            .await?;

        let mut results = response.json::<Vec<TiingoSearchResult>>().await?;

        let index = match results.iter().position(|r| &r.ticker == ticker) {
            Some(index) => index,
            None => return Ok(None),
        };

        return Ok(Some(Asset::from(results.remove(index))));
    }

    async fn candles(
        &self,
        ticker: &Ticker,
        market: Market,
        timeframe: Timeframe,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Candle>, MarketError> {
        let from_str = from.format("%Y-%m-%d").to_string();
        let to_str = to.format("%Y-%m-%d").to_string();

        let bars = match &timeframe {
            Timeframe::D1 => self.daily_prices(ticker, &from_str, &to_str).await,
            _ => {
                let frequency = match &timeframe {
                    Timeframe::M1 => "1min",
                    Timeframe::M5 => "5min",
                    Timeframe::M15 => "15min",
                    Timeframe::H1 => "1hour",
                    Timeframe::H4 => "4hour",
                    Timeframe::D1 => unreachable!(),
                };
                self.intraday_prices(ticker, market, frequency, &from_str, &to_str)
                    .await
            }
        }?;

        let duration = chrono::Duration::seconds(timeframe.secs());

        Ok(bars
            .iter()
            .filter_map(|bar| {
                let start = DateTime::parse_from_rfc3339(&bar.date)
                    .ok()?
                    .with_timezone(&Utc);
                let end = start + duration;
                Some(Candle {
                    open: bar.open,
                    high: bar.high,
                    low: bar.low,
                    close: bar.close,
                    start,
                    end,
                })
            })
            .collect())
    }

    async fn stream(
        &self,
        market: Market,
        tickers: &[Ticker],
        subscriber: impl Fn(Vec<TickerUpdate>) + Send + 'static,
    ) -> Result<Self::LiveStream, MarketError> {
        TiingoLiveStream::new(&self.api_key, market, tickers, subscriber).await
    }
}

impl TiingoProvider {
    /// Fetch daily historical price bars for a symbol.
    pub async fn daily_prices(
        &self,
        ticker: &Ticker,
        from: &str,
        to: &str,
    ) -> Result<Vec<PriceBar>, MarketError> {
        let response = self
            .http
            .get(format!(
                "https://api.tiingo.com/tiingo/daily/{}/prices",
                ticker.as_str()
            ))
            .query(&[
                ("startDate", from),
                ("endDate", to),
                ("token", self.api_key.as_str()),
            ])
            .send()
            .await?
            .check()
            .await?;

        response
            .json::<Vec<PriceBar>>()
            .await
            .map_err(MarketError::from)
    }

    /// Fetch intraday price bars for a symbol at a given frequency.
    pub async fn intraday_prices(
        &self,
        ticker: &Ticker,
        market: Market,
        frequency: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<PriceBar>, MarketError> {
        let market = match market {
            Market::Forex => "fx",
            Market::Stock => "iex",
            Market::Crypto => "crypto",
        };

        let response = self
            .http
            .get(format!(
                "https://api.tiingo.com/{}/{}/prices",
                market,
                ticker.as_str()
            ))
            .query(&[
                ("startDate", from),
                ("endDate", to),
                ("resampleFreq", frequency),
                ("token", self.api_key.as_str()),
            ])
            .send()
            .await?
            .check()
            .await?;

        response
            .json::<Vec<PriceBar>>()
            .await
            .map_err(MarketError::from)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TiingoSearchResult {
    pub name: String,
    pub ticker: Ticker,
    pub perma_ticker: String,
    pub open_figicomposite: Option<String>,
    pub asset_type: String,
    pub is_active: bool,
    pub country_code: String,
}

impl From<TiingoSearchResult> for Asset {
    fn from(value: TiingoSearchResult) -> Self {
        let market = match value.asset_type.as_str() {
            "STOCK" => Market::Stock,
            "FOREX" => Market::Forex,
            "CRYPTO" => Market::Crypto,
            _ => panic!("Unknown asset type: {}", value.asset_type),
        };

        Self {
            ticker: value.ticker,
            name: value.name,
            market,
        }
    }
}

/// A single price bar (used for both daily and intraday responses).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceBar {
    pub date: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

pub struct TiingoLiveStream {
    _handle: JoinHandle<()>,
}

impl TiingoLiveStream {
    pub const BASE_URL: &'static str = "wss://api.tiingo.com";

    pub async fn new(
        api_key: &str,
        market: Market,
        tickers: &[Ticker],
        subscriber: impl Fn(Vec<TickerUpdate>) + Send + 'static,
    ) -> Result<TiingoLiveStream, MarketError> {
        let url = match market {
            Market::Forex => format!("{}/fx", Self::BASE_URL),
            Market::Stock => format!("{}/iex", Self::BASE_URL),
            Market::Crypto => format!("{}/crypto", Self::BASE_URL),
        };

        let (stream, _) = tokio_tungstenite::connect_async(&url).await?;

        let (mut writer, mut reader) = stream.split();

        let subscribe_msg = serde_json::json!({
            "eventName": "subscribe",
            "authorization": api_key,
            "eventData": {
                "thresholdLevel": 2,
                "tickers": tickers
            }
        });

        writer
            .send(Message::Text(subscribe_msg.to_string().into()))
            .await?;

        let handle = tokio::spawn(async move {
            let mut aggregator = CandleAggregator::new(Timeframe::M1);

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

                if let WsEvent::Update {
                    ticker,
                    date,
                    mid_price,
                    ..
                } = event
                {
                    if let Some(candle) = aggregator.update(&ticker, mid_price, date) {
                        subscriber(vec![TickerUpdate { candle, ticker }])
                    }
                }
            }
        });

        Ok(Self { _handle: handle })
    }
}

impl LiveStream for TiingoLiveStream {
    async fn unsubscribe(&self) -> Result<(), MarketError> {
        todo!()
    }
}

pub enum WsEvent {
    HeartBeat {
        code: u32,
        message: String,
    },
    Success {
        code: u32,
        message: String,
        id: String,
    },
    Update {
        service: String,
        ticker: Ticker,
        date: DateTime<Utc>,
        exchange: String,

        bid_size: f32,
        mid_size: f32,
        ask_size: f32,

        bid_price: f64,
        mid_price: f64,
        ask_price: f64,
    },
}

impl<'de> Deserialize<'de> for WsEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let raw: Value = Value::deserialize(deserializer)?;

        let message_type = raw
            .get("messageType")
            .and_then(|v| v.as_str())
            .ok_or_else(|| D::Error::missing_field("messageType"))?;

        match message_type {
            "H" => {
                let response = raw
                    .get("response")
                    .ok_or_else(|| D::Error::missing_field("response"))?;
                let code = response
                    .get("code")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| D::Error::missing_field("response.code"))?
                    as u32;
                let message = response
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                Ok(WsEvent::HeartBeat { code, message })
            }
            "I" => {
                let response = raw
                    .get("response")
                    .ok_or_else(|| D::Error::missing_field("response"))?;
                let code = response
                    .get("code")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| D::Error::missing_field("response.code"))?
                    as u32;
                let message = response
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let id = raw
                    .get("data")
                    .and_then(|d| d.get("subscriptionId"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                Ok(WsEvent::Success { code, message, id })
            }
            "A" => {
                let service = raw
                    .get("service")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                let data = raw
                    .get("data")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| D::Error::missing_field("data"))?;

                // data[1]: ticker
                let ticker_str = data
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| D::Error::custom("missing ticker at data[1]"))?;
                let ticker = Ticker::from(ticker_str);

                // data[2]: ISO 8601 datetime
                let date_str = data
                    .get(2)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| D::Error::custom("missing date at data[2]"))?;
                let date = DateTime::parse_from_rfc3339(date_str)
                    .or_else(|_| {
                        chrono::DateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S%.f%:z")
                    })
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|e| D::Error::custom(format!("invalid date '{}': {}", date_str, e)))?;

                // data[3]: exchange
                let exchange = data
                    .get(3)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                // data[4]: bid_size
                let bid_size = data.get(4).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                // data[5]: mid_size
                let mid_size = data.get(5).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                // data[6]: ask_size
                let ask_size = data.get(6).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

                // data[7]: bid_price
                let bid_price = data.get(7).and_then(|v| v.as_f64()).unwrap_or(0.0);

                // data[8]: mid_price
                let mid_price = data.get(8).and_then(|v| v.as_f64()).unwrap_or(0.0);

                // data[9]: ask_price
                let ask_price = data.get(9).and_then(|v| v.as_f64()).unwrap_or(0.0);

                Ok(WsEvent::Update {
                    service,
                    ticker,
                    date,
                    exchange,
                    bid_size,
                    mid_size,
                    ask_size,
                    bid_price,
                    mid_price,
                    ask_price,
                })
            }
            other => Err(D::Error::custom(format!(
                "unknown messageType: '{}'",
                other
            ))),
        }
    }
}
