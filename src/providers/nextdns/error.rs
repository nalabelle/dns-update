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

use crate::error::Error;

pub fn map_error(e: NextDNSProviderError) -> Error {
    use NextDNSProviderError::*;
    match e {
        Http(err) => Error::ProviderError(err.to_string()),
        Credential(msg) => Error::CredentialError(msg),
        NotFound(msg) => Error::NotFound(msg),
        InvalidInput(msg) => Error::InvalidInput(msg),
        Provider(msg) => Error::ProviderError(msg),
        RateLimited => Error::ProviderError("Rate limited".to_string()),
        Unknown(msg) => Error::Other(msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

    // --- Error Mapping Tests ---
    #[test]
    fn test_map_error_variants() {
        use NextDNSProviderError::*;

        let err = map_error(Credential("fail".to_string()));
        assert!(matches!(err, Error::CredentialError(_)));
        let err = map_error(NotFound("not found".to_string()));
        assert!(matches!(err, Error::NotFound(_)));
        let err = map_error(InvalidInput("bad".to_string()));
        assert!(matches!(err, Error::InvalidInput(_)));
        let err = map_error(Provider("fail".to_string()));
        assert!(matches!(err, Error::ProviderError(_)));
        let err = map_error(RateLimited);
        assert!(matches!(err, Error::ProviderError(_)));
        let err = map_error(Unknown("fail".to_string()));
        assert!(matches!(err, Error::Other(_)));
    }
}
