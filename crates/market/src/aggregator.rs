use crate::{Candle, Tick, Ticker, Timeframe};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

pub struct CandleAggregator {
    timeframe: Timeframe,
    candles: HashMap<Ticker, Candle>,
}

impl CandleAggregator {
    pub fn new(timeframe: Timeframe) -> Self {
        Self {
            timeframe,
            candles: HashMap::new(),
        }
    }

    pub fn update(
        &mut self,
        ticker: &Ticker,
        price: f64,
        timestamp: DateTime<Utc>,
    ) -> Option<Candle> {
        let mut result: Option<Candle> = None;

        if self
            .candles
            .get_mut(ticker)
            .is_some_and(|candle| timestamp > candle.end)
        {
            result = self.candles.remove(ticker);
        } else {
            let current = self.candles.entry(ticker.clone()).or_insert_with(|| {
                let mut candle = Candle::from(Tick { timestamp, price });
                candle.end = candle.start + Duration::seconds(self.timeframe.secs());
                candle
            });

            current.high = current.high.max(price);
            current.low = current.low.min(price);
            current.close = price;
        }

        result
    }
}
