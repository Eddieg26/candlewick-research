use crate::{Asset, Candle, Market, Ticker, Timeframe, error::MarketError};
use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;

pub struct TickerUpdate {
    pub ticker: Ticker,
    pub candle: Candle,
}

pub trait LiveStream: Send + Sync + 'static {
    fn unsubscribe(&self) -> impl Future<Output = Result<(), MarketError>> + Send;
}

pub trait MarketDataProvider: Send + Sync + 'static {
    type LiveStream: LiveStream;

    fn search(
        &self,
        ticker: &Ticker,
        market: Market,
    ) -> impl Future<Output = Result<Option<Asset>, MarketError>> + Send;

    fn candles(
        &self,
        ticker: &Ticker,
        market: Market,
        timeframe: Timeframe,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> impl Future<Output = Result<Vec<Candle>, MarketError>> + Send;

    fn stream(
        &self,
        market: Market,
        tickers: &[Ticker],
        subscriber: impl Fn(Vec<TickerUpdate>) + Send + 'static,
    ) -> impl Future<Output = Result<Self::LiveStream, MarketError>> + Send;
}

pub trait ErasedLiveStream: Send + Sync + 'static {
    fn unsubscribe<'a>(&'a self) -> BoxFuture<'a, Result<(), MarketError>>;
}

impl<T: LiveStream> ErasedLiveStream for T {
    fn unsubscribe<'a>(&'a self) -> BoxFuture<'a, Result<(), MarketError>> {
        Box::pin(async { <T as LiveStream>::unsubscribe(self).await })
    }
}

pub trait ErasedMarketDataProvider: Send + Sync + 'static {
    fn search<'a>(
        &'a self,
        ticker: &'a Ticker,
        market: Market,
    ) -> BoxFuture<'a, Result<Option<Asset>, MarketError>>;

    fn candles<'a>(
        &'a self,
        ticker: &'a Ticker,
        market: Market,
        timeframe: Timeframe,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Vec<Candle>, MarketError>>;

    fn subscribe<'a>(
        &'a self,
        market: Market,
        tickers: &'a [Ticker],
        subscriber: Box<dyn Fn(Vec<TickerUpdate>) + Send + 'static>,
    ) -> BoxFuture<'a, Result<Box<dyn ErasedLiveStream>, MarketError>>;
}

impl<T: MarketDataProvider> ErasedMarketDataProvider for T {
    fn search<'a>(
        &'a self,
        ticker: &'a Ticker,
        market: Market,
    ) -> BoxFuture<'a, Result<Option<Asset>, MarketError>> {
        Box::pin(async move { <T as MarketDataProvider>::search(self, ticker, market).await })
    }

    fn candles<'a>(
        &'a self,
        ticker: &'a Ticker,
        market: Market,
        timeframe: Timeframe,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Vec<Candle>, MarketError>> {
        Box::pin(async move {
            <T as MarketDataProvider>::candles(self, ticker, market, timeframe, from, to).await
        })
    }

    fn subscribe<'a>(
        &'a self,
        market: Market,
        tickers: &'a [Ticker],
        subscriber: Box<dyn Fn(Vec<TickerUpdate>) + Send + 'static>,
    ) -> BoxFuture<'a, Result<Box<dyn ErasedLiveStream>, MarketError>> {
        Box::pin(async move {
            <T as MarketDataProvider>::stream(self, market, tickers, subscriber)
                .await
                .map(|v| Box::new(v) as Box<dyn ErasedLiveStream>)
        })
    }
}
