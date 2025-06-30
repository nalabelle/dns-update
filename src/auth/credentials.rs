use crate::error::Error;
use crate::onepassword::OnePasswordClient;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub trait CredentialManager: Send + Sync {
    fn get(&self, key: &str) -> Result<String, Error>;
}

/// 1Password-based credential provider
pub struct OnePasswordCredentialManager {
    client: Arc<OnePasswordClient>,
    rt: Runtime,
}

impl OnePasswordCredentialManager {
    pub fn new(client: Arc<OnePasswordClient>) -> Self {
        let rt = Runtime::new().expect("Failed to create Tokio runtime");
        Self { client, rt }
    }
}

impl CredentialManager for OnePasswordCredentialManager {
    fn get(&self, key: &str) -> Result<String, Error> {
        match key {
            "nextdns_email" => self
                .rt
                .block_on(self.client.get_nextdns_credentials())
                .map(|c| c.email)
                .map_err(|e| Error::CredentialError(e.to_string())),
            "nextdns_password" => self
                .rt
                .block_on(self.client.get_nextdns_credentials())
                .map(|c| c.password)
                .map_err(|e| Error::CredentialError(e.to_string())),
            "nextdns_profile_id" => self
                .rt
                .block_on(self.client.get_nextdns_credentials())
                .map(|c| c.id)
                .map_err(|e| Error::CredentialError(e.to_string())),
            _ => Err(Error::CredentialError(format!("Unknown key: {key}"))),
        }
    }
}
