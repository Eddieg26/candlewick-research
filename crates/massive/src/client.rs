// HTTP client for Massive Forex REST API

use crate::error::MassiveError;
use crate::models::{
    AggResponse, ConversionResponse, LastQuoteResponse, SearchResponse, SnapshotResponse,
};

pub struct MassiveClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl MassiveClient {
    pub fn new(api_key: String) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            base_url: "https://api.massive.com".to_string(),
            api_key,
        }
    }

    /// Search for forex tickers.
    /// GET /v3/reference/tickers?search={query}&market=fx&active=true&limit={limit}&apiKey=KEY
    pub async fn search(&self, query: &str, limit: u32) -> Result<SearchResponse, MassiveError> {
        let url = format!("{}/v3/reference/tickers", self.base_url);

        let response = self
            .http
            .get(&url)
            .query(&[
                ("search", query),
                ("market", "fx"),
                ("active", "true"),
                ("limit", &limit.to_string()),
                ("apiKey", &self.api_key),
            ])
            .send()
            .await
            .map_err(|e| MassiveError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<SearchResponse>()
            .await
            .map_err(|e| MassiveError::ParseError(e.to_string()))
    }

    /// Fetch aggregate bars (OHLC).
    /// GET /v2/aggs/ticker/{ticker}/range/{multiplier}/{timespan}/{from}/{to}?adjusted=true&sort=...&limit=...&apiKey=KEY
    pub async fn bars(
        &self,
        ticker: &str,
        multiplier: u32,
        timespan: &str,
        from: &str,
        to: &str,
        sort: &str,
        limit: u32,
    ) -> Result<AggResponse, MassiveError> {
        let url = format!(
            "{}/v2/aggs/ticker/{}/range/{}/{}/{}/{}",
            self.base_url, ticker, multiplier, timespan, from, to
        );

        let response = self
            .http
            .get(&url)
            .query(&[
                ("adjusted", "true"),
                ("sort", sort),
                ("limit", &limit.to_string()),
                ("apiKey", &self.api_key),
            ])
            .send()
            .await
            .map_err(|e| MassiveError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<AggResponse>()
            .await
            .map_err(|e| MassiveError::ParseError(e.to_string()))
    }

    /// Get last quote for a currency pair.
    /// GET /v1/last_quote/currencies/{from}/{to}?apiKey=KEY
    pub async fn last_quote(
        &self,
        from: &str,
        to: &str,
    ) -> Result<LastQuoteResponse, MassiveError> {
        let url = format!(
            "{}/v1/last_quote/currencies/{}/{}",
            self.base_url, from, to
        );

        let response = self
            .http
            .get(&url)
            .query(&[("apiKey", &self.api_key)])
            .send()
            .await
            .map_err(|e| MassiveError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<LastQuoteResponse>()
            .await
            .map_err(|e| MassiveError::ParseError(e.to_string()))
    }

    /// Convert between currencies.
    /// GET /v1/conversion/{from}/{to}?amount=...&precision=...&apiKey=KEY
    pub async fn convert(
        &self,
        from: &str,
        to: &str,
        amount: f64,
        precision: u32,
    ) -> Result<ConversionResponse, MassiveError> {
        let url = format!("{}/v1/conversion/{}/{}", self.base_url, from, to);

        let response = self
            .http
            .get(&url)
            .query(&[
                ("amount", amount.to_string().as_str()),
                ("precision", &precision.to_string()),
                ("apiKey", &self.api_key),
            ])
            .send()
            .await
            .map_err(|e| MassiveError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<ConversionResponse>()
            .await
            .map_err(|e| MassiveError::ParseError(e.to_string()))
    }

    /// Get ticker snapshot.
    /// GET /v2/snapshot/locale/global/markets/forex/tickers/{ticker}?apiKey=KEY
    pub async fn snapshot(&self, ticker: &str) -> Result<SnapshotResponse, MassiveError> {
        let url = format!(
            "{}/v2/snapshot/locale/global/markets/forex/tickers/{}",
            self.base_url, ticker
        );

        let response = self
            .http
            .get(&url)
            .query(&[("apiKey", &self.api_key)])
            .send()
            .await
            .map_err(|e| MassiveError::Other(e.to_string()))?;

        let response = check_response(response).await?;

        response
            .json::<SnapshotResponse>()
            .await
            .map_err(|e| MassiveError::ParseError(e.to_string()))
    }
}

/// Check the HTTP response status and map error codes to typed errors.
/// Returns the response unchanged if the status is successful.
pub(crate) async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response, MassiveError> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    match status.as_u16() {
        401 => Err(MassiveError::Unauthorized),
        403 => Err(MassiveError::PremiumRequired),
        429 => Err(MassiveError::RateLimited),
        code => {
            let body = resp.text().await.unwrap_or_default();
            Err(MassiveError::HttpError {
                status: code,
                message: body,
            })
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use wiremock::matchers::any;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Generate a non-2xx HTTP status code in the 400-599 range (client/server errors).
    /// These are the codes where response bodies are reliably transmitted.
    /// We exclude 401, 403, 429 to test them separately in the same property.
    fn error_status_other() -> impl Strategy<Value = u16> {
        (400u16..600).prop_filter(
            "exclude specially-mapped codes",
            |s| *s != 401 && *s != 403 && *s != 429,
        )
    }

    /// Generate any non-2xx status code for testing the special mappings.
    /// Includes 3xx (redirects), 4xx, and 5xx.
    fn non_success_status() -> impl Strategy<Value = u16> {
        prop_oneof![
            Just(401u16),
            Just(403u16),
            Just(429u16),
            error_status_other(),
            300u16..400, // 3xx redirects — tested for status mapping only
        ]
    }

    // **Validates: Requirements 8.1, 8.2, 8.3, 8.4**
    //
    // Property 7: HTTP error mapping consistency — For any non-2xx status,
    // check_response maps 401→Unauthorized, 403→PremiumRequired,
    // 429→RateLimited, others→HttpError with status and body.
    proptest! {
        #[test]
        fn prop_http_error_mapping_consistency(
            status_code in non_success_status(),
            body in "[a-zA-Z0-9]{1,30}",
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mock_server = MockServer::start().await;

                Mock::given(any())
                    .respond_with(
                        ResponseTemplate::new(status_code).set_body_string(body.clone()),
                    )
                    .mount(&mock_server)
                    .await;

                let client = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .unwrap();
                let resp = client
                    .get(mock_server.uri())
                    .send()
                    .await
                    .expect("request should succeed");

                let result = check_response(resp).await;

                match status_code {
                    401 => {
                        let err = result.unwrap_err();
                        assert!(
                            matches!(err, MassiveError::Unauthorized),
                            "Expected Unauthorized for 401, got: {err:?}"
                        );
                    }
                    403 => {
                        let err = result.unwrap_err();
                        assert!(
                            matches!(err, MassiveError::PremiumRequired),
                            "Expected PremiumRequired for 403, got: {err:?}"
                        );
                    }
                    429 => {
                        let err = result.unwrap_err();
                        assert!(
                            matches!(err, MassiveError::RateLimited),
                            "Expected RateLimited for 429, got: {err:?}"
                        );
                    }
                    code => {
                        let err = result.unwrap_err();
                        match err {
                            MassiveError::HttpError { status, message } => {
                                assert_eq!(
                                    status, code,
                                    "HttpError status should match response status"
                                );
                                // For 4xx/5xx, the body is preserved.
                                // For 3xx, some HTTP implementations may strip the body
                                // per RFC 7231, so we only assert body for 4xx/5xx.
                                if code >= 400 {
                                    assert_eq!(
                                        message, body,
                                        "HttpError message should match response body"
                                    );
                                }
                            }
                            other => panic!(
                                "Expected HttpError for status {code}, got: {other:?}"
                            ),
                        }
                    }
                }
            });
        }
    }
}
