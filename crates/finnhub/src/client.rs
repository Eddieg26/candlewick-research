// HTTP client for Finnhub REST API

use crate::error::FinnhubError;
use crate::models::{CandleResponse, SearchResponse, validate_resolution};

pub struct FinnhubClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl FinnhubClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: "https://finnhub.io/api/v1".to_string(),
            token,
        }
    }

    /// Search for symbols matching the query.
    /// Appends `exchange` parameter if provided.
    pub async fn search(
        &self,
        query: &str,
        exchange: Option<&str>,
    ) -> Result<SearchResponse, FinnhubError> {
        let mut request = self
            .http
            .get(format!("{}/search", self.base_url))
            .query(&[("q", query), ("token", &self.token)]);

        if let Some(ex) = exchange {
            request = request.query(&[("exchange", ex)]);
        }

        let response = request
            .send()
            .await
            .map_err(|e| FinnhubError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<SearchResponse>()
            .await
            .map_err(|e| FinnhubError::ParseError(e.to_string()))
    }

    /// Fetch historical candle data for a symbol.
    pub async fn candles(
        &self,
        symbol: &str,
        resolution: &str,
        from: i64,
        to: i64,
    ) -> Result<CandleResponse, FinnhubError> {
        validate_resolution(resolution)?;

        if from >= to {
            return Err(FinnhubError::InvalidDateRange);
        }

        let response = self
            .http
            .get(format!("{}/stock/candle", self.base_url))
            .query(&[
                ("symbol", symbol),
                ("resolution", resolution),
                ("from", &from.to_string()),
                ("to", &to.to_string()),
                ("token", &self.token),
            ])
            .send()
            .await
            .map_err(|e| FinnhubError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<CandleResponse>()
            .await
            .map_err(|e| FinnhubError::ParseError(e.to_string()))
    }
}

/// Check the HTTP response status and map error codes to typed errors.
/// Returns the response unchanged if the status is successful.
async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response, FinnhubError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    match status.as_u16() {
        401 => Err(FinnhubError::Unauthorized),
        403 => Err(FinnhubError::PremiumRequired),
        429 => Err(FinnhubError::RateLimited),
        code => {
            let body = resp.text().await.unwrap_or_default();
            Err(FinnhubError::HttpError {
                status: code,
                message: body,
            })
        }
    }
}
