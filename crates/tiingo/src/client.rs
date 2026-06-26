// HTTP client for Tiingo REST API

use crate::error::TiingoError;
use crate::models::{PriceBar, SearchResult};

pub struct TiingoClient {
    http: reqwest::Client,
    token: String,
}

impl TiingoClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            token,
        }
    }

    /// Search for symbols matching the query.
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, TiingoError> {
        let response = self
            .http
            .get("https://api.tiingo.com/tiingo/utilities/search")
            .query(&[("query", query), ("token", &self.token)])
            .send()
            .await
            .map_err(|e| TiingoError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<Vec<SearchResult>>()
            .await
            .map_err(|e| TiingoError::ParseError(e.to_string()))
    }

    /// Fetch daily historical price bars for a symbol.
    pub async fn daily_prices(
        &self,
        symbol: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<PriceBar>, TiingoError> {
        let response = self
            .http
            .get(format!(
                "https://api.tiingo.com/tiingo/daily/{}/prices",
                symbol
            ))
            .query(&[
                ("startDate", from),
                ("endDate", to),
                ("token", self.token.as_str()),
            ])
            .send()
            .await
            .map_err(|e| TiingoError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<Vec<PriceBar>>()
            .await
            .map_err(|e| TiingoError::ParseError(e.to_string()))
    }

    /// Fetch intraday price bars for a symbol at a given frequency.
    pub async fn intraday_prices(
        &self,
        symbol: &str,
        frequency: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<PriceBar>, TiingoError> {
        let response = self
            .http
            .get(format!("https://api.tiingo.com/iex/{}/prices", symbol))
            .query(&[
                ("startDate", from),
                ("endDate", to),
                ("resampleFreq", frequency),
                ("token", self.token.as_str()),
            ])
            .send()
            .await
            .map_err(|e| TiingoError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<Vec<PriceBar>>()
            .await
            .map_err(|e| TiingoError::ParseError(e.to_string()))
    }
}

/// Check the HTTP response status and map error codes to typed errors.
/// Returns the response unchanged if the status is successful.
async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response, TiingoError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    match status.as_u16() {
        401 => Err(TiingoError::Unauthorized),
        429 => Err(TiingoError::RateLimited),
        code => {
            let body = resp.text().await.unwrap_or_default();
            Err(TiingoError::HttpError {
                status: code,
                message: body,
            })
        }
    }
}
