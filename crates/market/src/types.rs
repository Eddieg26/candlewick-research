use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ticker(SmolStr);

impl Ticker {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for Ticker {
    type Target = SmolStr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<SmolStr> for Ticker {
    fn as_ref(&self) -> &SmolStr {
        &self.0
    }
}

impl From<&str> for Ticker {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

pub enum Market {
    Forex,
    Stock,
    Crypto,
}

pub enum Timeframe {
    M1,
    M5,
    M15,
    H1,
    H4,
    D1,
}

impl Timeframe {
    pub fn secs(&self) -> i64 {
        match self {
            Timeframe::M1 => 60,
            Timeframe::M5 => 60 * 5,
            Timeframe::M15 => 60 * 15,
            Timeframe::H1 => 60 * 60,
            Timeframe::H4 => 60 * 60 * 4,
            Timeframe::D1 => 60 * 60 * 24,
        }
    }
}

pub struct Asset {
    pub ticker: Ticker,
    pub name: String,
    pub market: Market,
}

#[derive(Debug, Clone, Copy)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct Tick {
    pub price: f64,
    pub timestamp: DateTime<Utc>,
}

impl From<Tick> for Candle {
    fn from(tick: Tick) -> Self {
        Self {
            open: tick.price,
            high: tick.price,
            low: tick.price,
            close: tick.price,
            start: tick.timestamp,
            end: tick.timestamp,
        }
    }
}
