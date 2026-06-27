use crate::error::FinageError;
use crate::models::{AggregatesResponse, SymbolListResponse};

pub struct FinageClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl FinageClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: "https://api.finage.co.uk".to_string(),
            token,
        }
    }

    /// Search symbols by keyword within a market type.
    pub async fn search(
        &self,
        query: &str,
        market: &str,
    ) -> Result<SymbolListResponse, FinageError> {
        let url = format!(
            "{}/symbol-list/{}?search={}&apikey={}",
            self.base_url, market, query, self.token
        );

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| FinageError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<SymbolListResponse>()
            .await
            .map_err(|e| FinageError::ParseError(e.to_string()))
    }

    /// Fetch aggregate OHLCV bars.
    #[allow(clippy::too_many_arguments)]
    pub async fn bars(
        &self,
        symbol: &str,
        multiplier: u32,
        timespan: &str,
        from: &str,
        to: &str,
        sort: &str,
        limit: u32,
    ) -> Result<AggregatesResponse, FinageError> {
        let url = format!(
            "{}/agg/stock/{}/{}/{}/{}/{}?apikey={}&sort={}&limit={}",
            self.base_url, symbol, multiplier, timespan, from, to, self.token, sort, limit
        );

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| FinageError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<AggregatesResponse>()
            .await
            .map_err(|e| FinageError::ParseError(e.to_string()))
    }
}

/// Map non-2xx HTTP responses to FinageError variants.
async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response, FinageError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    match status.as_u16() {
        401 => Err(FinageError::Unauthorized),
        429 => Err(FinageError::RateLimited),
        code => {
            let body = resp.text().await.unwrap_or_default();
            let message = if body.len() > 1024 {
                body[..1024].to_string()
            } else {
                body
            };
            Err(FinageError::HttpError {
                status: code,
                message,
            })
        }
    }
}
