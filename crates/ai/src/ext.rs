use crate::error::AiError;
use reqwest::Response;

pub trait ResponseExt {
    fn check(
        self
    ) -> impl Future<Output = Result<reqwest::Response, AiError>> + Send;
}

impl ResponseExt for Response {
    async fn check(self) -> Result<reqwest::Response, AiError> {
        let status = self.status();
        if status.is_success() {
            return Ok(self);
        }
        match status.as_u16() {
            401 => Err(AiError::Unauthorized),
            403 => Err(AiError::Unauthorized),
            429 => Err(AiError::RateLimited),
            code => {
                let body = self.text().await.unwrap_or_default();
                Err(AiError::HttpError {
                    status: code,
                    message: body,
                })
            }
        }
    }
}
