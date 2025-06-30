use crate::providers::nextdns::types::NextDNSError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NextDNSProviderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Credential error: {0}")]
    Credential(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Rate limited")]
    RateLimited,

    #[allow(dead_code)]
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<NextDNSError> for NextDNSProviderError {
    fn from(err: NextDNSError) -> Self {
        match err.code.as_str() {
            "not_found" => NextDNSProviderError::NotFound(err.message),
            "invalid_input" => NextDNSProviderError::InvalidInput(err.message),
            "unauthorized" => NextDNSProviderError::Credential(err.message),
            "rate_limited" => NextDNSProviderError::RateLimited,
            _ => NextDNSProviderError::Provider(err.message),
        }
    }
}
