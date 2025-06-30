use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::credentials::CredentialManager;
    use httpmock::prelude::*;
    use mockall::predicate::*;
    use std::sync::Arc;

    struct FakeCredentialManager {
        creds: std::collections::HashMap<String, String>,
        fail: bool,
    }

    use crate::error::Error;
    impl CredentialManager for FakeCredentialManager {
        fn get(&self, key: &str) -> Result<String, Error> {
            if self.fail {
                Err(Error::CredentialError("invalid credentials".into()))
            } else {
                self.creds
                    .get(key)
                    .cloned()
                    .ok_or(Error::CredentialError("missing".into()))
            }
        }
    }

    #[tokio::test]
    async fn test_full_workflow_success() {
        let server = MockServer::start_async().await;
        let profile_id = "profileid";
        let api_url = server.url("");
        // Mock login endpoint
        let login_mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/auth/login");
                then.status(200)
                    .json_body_obj(&serde_json::json!({ "success": true }));
            })
            .await;
        // Mock list rewrites endpoint
        let list_mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path(format!("/profiles/{profile_id}/dns/rewrites"));
                then.status(200)
                    .json_body_obj::<Vec<serde_json::Value>>(&vec![]);
            })
            .await;

        let creds = FakeCredentialManager {
            creds: [
                ("nextdns_email".into(), "user@example.com".into()),
                ("nextdns_password".into(), "secret".into()),
            ]
            .iter()
            .cloned()
            .collect(),
            fail: false,
        };

        let config = NextDNSConfig {
            profile_id: profile_id.into(),
            api_url: api_url.clone(),
        };
        let provider = NextDNSProvider::new(config, Arc::new(creds)).await;
        assert!(provider.is_ok());
        // Actually call list_rewrites to trigger both mocks
        let provider = provider.unwrap();
        let _ = provider.list_rewrites().await;
        login_mock.assert_async().await;
        list_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_workflow_with_invalid_credentials() {
        let server = MockServer::start_async().await;
        let profile_id = "profileid";
        let api_url = server.url("");
        // Mock login endpoint to fail
        let login_mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/auth/login");
                then.status(401)
                    .json_body_obj(&serde_json::json!({ "error": "unauthorized" }));
            })
            .await;

        let creds = FakeCredentialManager {
            creds: [
                ("nextdns_email".into(), "baduser".into()),
                ("nextdns_password".into(), "badpass".into()),
            ]
            .iter()
            .cloned()
            .collect(),
            fail: false,
        };

        let config = NextDNSConfig {
            profile_id: profile_id.into(),
            api_url: api_url.clone(),
        };
        let provider = NextDNSProvider::new(config, Arc::new(creds)).await;
        assert!(provider.is_err());
        login_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_workflow_with_api_failure() {
        let server = MockServer::start_async().await;
        let profile_id = "profileid";
        let api_url = server.url("");
        // Mock login endpoint
        let login_mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/auth/login");
                then.status(200)
                    .json_body_obj(&serde_json::json!({ "success": true }));
            })
            .await;
        // Mock list rewrites endpoint to fail
        let list_mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path(format!("/profiles/{profile_id}/dns/rewrites"));
                then.status(500)
                    .json_body_obj(&serde_json::json!({ "error": "server error" }));
            })
            .await;

        let creds = FakeCredentialManager {
            creds: [
                ("nextdns_email".into(), "user@example.com".into()),
                ("nextdns_password".into(), "secret".into()),
            ]
            .iter()
            .cloned()
            .collect(),
            fail: false,
        };

        let config = NextDNSConfig {
            profile_id: profile_id.into(),
            api_url: api_url.clone(),
        };
        let provider = NextDNSProvider::new(config, Arc::new(creds)).await.unwrap();
        let result = provider.list_rewrites().await;
        assert!(result.is_err());
        login_mock.assert_async().await;
        list_mock.assert_async().await;
    }
}
use reqwest::{Client, StatusCode};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::auth::credentials::CredentialManager;
use crate::providers::nextdns::error::NextDNSProviderError;
use crate::providers::nextdns::types::*;

pub struct NextDNSConfig {
    pub profile_id: String,
    pub api_url: String,
}

pub struct NextDNSProvider {
    config: NextDNSConfig,
    client: Client,
    credentials: Arc<dyn CredentialManager>,
    rate_limiter: RateLimiter,
}

#[derive(Clone)]
struct RateLimiter {
    last_request: Arc<Mutex<Instant>>,
    min_delay: Duration,
}

impl RateLimiter {
    async fn wait(&self) {
        let mut last = self.last_request.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last);

        if elapsed < self.min_delay {
            tokio::time::sleep(self.min_delay - elapsed).await;
        }

        *last = Instant::now();
    }
}

impl NextDNSProvider {
    pub async fn new(
        config: NextDNSConfig,
        credentials: Arc<dyn CredentialManager>,
    ) -> Result<Self, NextDNSProviderError> {
        let client = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(30))
            .build()?;

        let rate_limiter = RateLimiter {
            last_request: Arc::new(Mutex::new(Instant::now())),
            min_delay: Duration::from_millis(500),
        };

        let provider = Self {
            config,
            client,
            credentials,
            rate_limiter,
        };

        provider.authenticate().await?;
        Ok(provider)
    }

    async fn authenticate(&self) -> Result<(), NextDNSProviderError> {
        let email = self
            .credentials
            .get("nextdns_email")
            .map_err(|e| NextDNSProviderError::Credential(e.to_string()))?;
        let password = self
            .credentials
            .get("nextdns_password")
            .map_err(|e| NextDNSProviderError::Credential(e.to_string()))?;

        let login = LoginRequest { email, password };

        let res = self
            .client
            .post(format!("{}/auth/login", self.config.api_url))
            .json(&login)
            .send()
            .await?;

        res.error_for_status_ref()?;
        Ok(())
    }

    async fn handle_request<T, F>(&self, fut: F) -> Result<T, NextDNSProviderError>
    where
        F: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
        T: serde::de::DeserializeOwned,
    {
        let response = fut.await?;

        match response.status() {
            StatusCode::OK => Ok(response.json().await?),
            StatusCode::TOO_MANY_REQUESTS => {
                let retry_after = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(5);

                tokio::time::sleep(Duration::from_secs(retry_after)).await;
                Err(NextDNSProviderError::RateLimited)
            }
            _ => {
                let error: NextDNSError = response.json().await.unwrap_or(NextDNSError {
                    code: "unknown".to_string(),
                    message: "Unknown error".to_string(),
                });
                Err(error.into())
            }
        }
    }

    // Example: List DNS rewrites
    pub async fn list_rewrites(&self) -> Result<Vec<NextDNSRecord>, NextDNSProviderError> {
        self.rate_limiter.wait().await;
        let url = format!(
            "{}/profiles/{}/dns/rewrites",
            self.config.api_url, self.config.profile_id
        );
        self.handle_request(self.client.get(url).send()).await
    }

    // Example: Create DNS rewrite
    pub async fn create_rewrite(
        &self,
        req: &CreateRecordRequest,
    ) -> Result<NextDNSRecord, NextDNSProviderError> {
        self.rate_limiter.wait().await;
        let url = format!(
            "{}/profiles/{}/dns/rewrites",
            self.config.api_url, self.config.profile_id
        );
        self.handle_request(self.client.post(url).json(req).send())
            .await
    }

    // Example: Update DNS rewrite
    pub async fn update_rewrite(
        &self,
        id: &str,
        req: &CreateRecordRequest,
    ) -> Result<NextDNSRecord, NextDNSProviderError> {
        self.rate_limiter.wait().await;
        let url = format!(
            "{}/profiles/{}/dns/rewrites/{}",
            self.config.api_url, self.config.profile_id, id
        );
        self.handle_request(self.client.put(url).json(req).send())
            .await
    }

    // Example: Delete DNS rewrite
    pub async fn delete_rewrite(&self, id: &str) -> Result<(), NextDNSProviderError> {
        self.rate_limiter.wait().await;
        let url = format!(
            "{}/profiles/{}/dns/rewrites/{}",
            self.config.api_url, self.config.profile_id, id
        );
        let response = self.client.delete(url).send().await?;
        match response.status() {
            StatusCode::NO_CONTENT | StatusCode::OK => Ok(()),
            _ => {
                let error: NextDNSError = response.json().await.unwrap_or(NextDNSError {
                    code: "unknown".to_string(),
                    message: "Unknown error".to_string(),
                });
                Err(error.into())
            }
        }
    }
}
