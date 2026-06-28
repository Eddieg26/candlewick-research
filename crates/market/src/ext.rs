use crate::error::MarketError;
use reqwest::Response;

pub trait ResponseExt {
    fn check(
        self
    ) -> impl Future<Output = Result<reqwest::Response, MarketError>> + Send;
}

impl ResponseExt for Response {
    async fn check(self) -> Result<reqwest::Response, MarketError> {
        let status = self.status();
        if status.is_success() {
            return Ok(self);
        }
        match status.as_u16() {
            401 => Err(MarketError::Unauthorized),
            403 => Err(MarketError::Unauthorized),
            429 => Err(MarketError::RateLimited),
            code => {
                let body = self.text().await.unwrap_or_default();
                Err(MarketError::HttpError {
                    status: code,
                    message: body,
                })
            }
        }
    }
}
