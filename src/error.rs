use std::fmt;

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum Error {
    ProviderError(String),
    CredentialError(String),
    NotFound(String),
    InvalidInput(String),
    Other(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ProviderError(msg) => write!(f, "Provider error: {msg}"),
            Error::CredentialError(msg) => write!(f, "Credential error: {msg}"),
            Error::NotFound(msg) => write!(f, "Not found: {msg}"),
            Error::InvalidInput(msg) => write!(f, "Invalid input: {msg}"),
            Error::Other(msg) => write!(f, "Other error: {msg}"),
        }
    }
}
