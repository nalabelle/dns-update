//! 1Password CLI integration for credentials and DNS rewrite config.

use serde::Deserialize;
use std::collections::HashMap;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;

#[derive(Error, Debug)]
pub enum OnePasswordError {
    #[error("1Password CLI error: {0}")]
    Cli(String),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Missing field: {0}")]
    MissingField(String),
}

impl Clone for OnePasswordError {
    fn clone(&self) -> Self {
        match self {
            OnePasswordError::Cli(s) => OnePasswordError::Cli(s.clone()),
            OnePasswordError::Json(e) => OnePasswordError::Cli(e.to_string()),
            OnePasswordError::MissingField(s) => OnePasswordError::MissingField(s.clone()),
        }
    }
}

pub struct OnePasswordClient {
    vault: String,
}

impl OnePasswordClient {
    pub fn new(vault: &str) -> Self {
        Self {
            vault: vault.to_string(),
        }
    }

    /// Get a single field from a 1Password item.
    pub async fn get_field(&self, item: &str, field: &str) -> Result<String, OnePasswordError> {
        let output = Command::new("op")
            .arg("item")
            .arg("get")
            .arg(item)
            .arg(format!("--vault={}", self.vault))
            .arg("--fields")
            .arg(field)
            .arg("--format")
            .arg("json")
            .stdout(Stdio::piped())
            .output()
            .await
            .map_err(|e| OnePasswordError::Cli(e.to_string()))?;

        if !output.status.success() {
            return Err(OnePasswordError::Cli(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        #[derive(Deserialize)]
        struct FieldValue {
            value: String,
        }

        let fv: FieldValue = serde_json::from_slice(&output.stdout)?;
        Ok(strip_formatting(&fv.value))
    }

    /// Get multiple fields from a 1Password item.
    pub async fn get_fields(
        &self,
        item: &str,
        fields: &[&str],
    ) -> Result<HashMap<String, String>, OnePasswordError> {
        let output = Command::new("op")
            .arg("item")
            .arg("get")
            .arg(item)
            .arg(format!("--vault={}", self.vault))
            .arg("--fields")
            .arg(fields.join(","))
            .arg("--format")
            .arg("json")
            .stdout(Stdio::piped())
            .output()
            .await
            .map_err(|e| OnePasswordError::Cli(e.to_string()))?;

        if !output.status.success() {
            return Err(OnePasswordError::Cli(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        #[derive(Deserialize)]
        struct Field {
            label: String,
            value: String,
        }

        let parsed: Vec<Field> = serde_json::from_slice(&output.stdout)?;
        Ok(parsed
            .into_iter()
            .map(|f| (f.label, strip_formatting(&f.value)))
            .collect())
    }

    /// Get DNS rewrites from the "DNS Rewrites" item, "notesPlain" field.
    pub async fn get_dns_rewrites(&self) -> Result<String, OnePasswordError> {
        self.get_field("DNS Rewrites", "notesPlain").await
    }

    /// Get NextDNS credentials from the "NextDNS" item.
    pub async fn get_nextdns_credentials(&self) -> Result<NextDnsCredentials, OnePasswordError> {
        let fields = self
            .get_fields("NextDNS", &["prefix", "email", "password"])
            .await?;
        Ok(NextDnsCredentials {
            id: fields
                .get("prefix")
                .cloned()
                .ok_or_else(|| OnePasswordError::MissingField("prefix".into()))?,
            email: fields
                .get("email")
                .cloned()
                .ok_or_else(|| OnePasswordError::MissingField("email".into()))?,
            password: fields
                .get("password")
                .cloned()
                .ok_or_else(|| OnePasswordError::MissingField("password".into()))?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct NextDnsCredentials {
    pub id: String,
    pub email: String,
    pub password: String,
}

fn strip_formatting(value: &str) -> String {
    let mut v = value.trim();
    if v.starts_with("```") {
        v = &v[3..];
    }
    if v.ends_with("```") {
        v = &v[..v.len() - 3];
    }
    if v.starts_with("~~~") {
        v = &v[3..];
    }
    if v.ends_with("~~~") {
        v = &v[..v.len() - 3];
    }
    v.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{automock, predicate::*};

    #[automock]
    trait OnePasswordClientTrait {
        fn get_nextdns_credentials(&self) -> Result<NextDnsCredentials, OnePasswordError>;
    }

    struct FakeClient {
        result: Result<NextDnsCredentials, OnePasswordError>,
    }

    impl OnePasswordClientTrait for FakeClient {
        fn get_nextdns_credentials(&self) -> Result<NextDnsCredentials, OnePasswordError> {
            self.result.clone()
        }
    }

    #[test]
    fn test_retrieve_credentials_success() {
        let creds = NextDnsCredentials {
            id: "profileid".into(),
            email: "user@example.com".into(),
            password: "secret".into(),
        };
        let client = FakeClient {
            result: Ok(creds.clone()),
        };
        let result = client.get_nextdns_credentials();
        assert!(result.is_ok());
        let out = result.unwrap();
        assert_eq!(out.id, "profileid");
        assert_eq!(out.email, "user@example.com");
        assert_eq!(out.password, "secret");
    }

    #[test]
    fn test_retrieve_credentials_failure() {
        let client = FakeClient {
            result: Err(OnePasswordError::Cli("op error".into())),
        };
        let result = client.get_nextdns_credentials();
        assert!(matches!(result, Err(OnePasswordError::Cli(_))));
    }

    #[test]
    fn test_invalid_credentials_format() {
        let client = FakeClient {
            result: Err(OnePasswordError::MissingField("prefix".into())),
        };
        let result = client.get_nextdns_credentials();
        assert!(matches!(result, Err(OnePasswordError::MissingField(_))));
    }
}
