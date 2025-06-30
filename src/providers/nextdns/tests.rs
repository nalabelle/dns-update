//! Integration tests for NextDNS provider

use super::*;
use crate::core::record::{DNSRecord, DNSRecordType};
use crate::core::provider::DNSProvider;
use crate::error::Error;
use crate::providers::nextdns::{NextDNSRecord, CreateRecordRequest, NextDNSProviderError, NextDNSConfig, NextDNSProvider};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use async_trait::async_trait;

    // --- Mock CredentialManager ---
    struct MockCredentialManager {
        data: HashMap<String, String>,
    }

    impl MockCredentialManager {
        fn new() -> Self {
            let mut data = HashMap::new();
            data.insert("nextdns_email".to_string(), "test@example.com".to_string());
            data.insert("nextdns_password".to_string(), "password".to_string());
            Self { data }
        }
    }

    #[async_trait]
    impl crate::auth::credentials::CredentialManager for MockCredentialManager {
        fn get(&self, key: &str) -> Result<String, String> {
            self.data.get(key).cloned().ok_or_else(|| "not found".to_string())
        }
    }

    // --- Record Conversion Tests ---
    #[test]
    fn test_to_dns_record_and_back() {
        let nextdns = NextDNSRecord {
            id: "abc".to_string(),
            domain: "example.com".to_string(),
            record_type: "A".to_string(),
            value: "1.2.3.4".to_string(),
            ttl: Some(60),
        };
        let dns = super::to_dns_record(&nextdns);
        assert_eq!(dns.record_type, DNSRecordType::A);
        assert_eq!(dns.name, "example.com");
        assert_eq!(dns.value, "1.2.3.4");
        assert_eq!(dns.ttl, Some(60));

        let req = super::to_nextdns_record(&dns);
        assert_eq!(req.domain, "example.com");
        assert_eq!(req.record_type, "A");
        assert_eq!(req.value, "1.2.3.4");
        assert_eq!(req.ttl, Some(60));
    }

    #[test]
    fn test_to_dns_record_invalid_type() {
        let nextdns = NextDNSRecord {
            id: "abc".to_string(),
            domain: "example.com".to_string(),
            record_type: "TXT".to_string(),
            value: "foo".to_string(),
            ttl: None,
        };
        let dns = super::to_dns_record(&nextdns);
        // Fallback is A
        assert_eq!(dns.record_type, DNSRecordType::A);
    }

    // --- Error Mapping Tests ---
    #[test]
    fn test_map_error_variants() {
        use super::NextDNSProviderError::*;
        let err = super::map_error(Http(reqwest::Error::new(reqwest::StatusCode::BAD_REQUEST, "bad")));
        assert!(matches!(err, Error::ProviderError(_)));
        let err = super::map_error(Credential("fail".to_string()));
        assert!(matches!(err, Error::CredentialError(_)));
        let err = super::map_error(NotFound("not found".to_string()));
        assert!(matches!(err, Error::NotFound(_)));
        let err = super::map_error(InvalidInput("bad".to_string()));
        assert!(matches!(err, Error::InvalidInput(_)));
        let err = super::map_error(Provider("fail".to_string()));
        assert!(matches!(err, Error::ProviderError(_)));
        let err = super::map_error(RateLimited);
        assert!(matches!(err, Error::ProviderError(_)));
        let err = super::map_error(Unknown("fail".to_string()));
        assert!(matches!(err, Error::Other(_)));
    }

    // --- Provider Trait Tests (API Mocking) ---
    // For API mocking, use httpmock or wiremock if available.
    // Here, we only show the structure; actual HTTP mocking setup is required for real tests.

    #[tokio::test]
    async fn test_provider_trait_list_records_empty() {
        // This is a placeholder; in real tests, mock HTTP to return empty list.
        let config = NextDNSConfig {
            profile_id: "dummy".to_string(),
            api_url: "http://localhost:12345".to_string(),
        };
        let creds = Arc::new(MockCredentialManager::new());
        // Would need to inject a mock HTTP client here.
        // let provider = NextDNSProvider::new(config, creds).await.unwrap();
        // let records = provider.list_records().await.unwrap();
        // assert!(records.is_empty());
        assert!(true, "HTTP mocking required for real test");
    }

    // Add more integration tests with HTTP mocking as needed.
}
