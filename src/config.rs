use std::env;
use std::time::Duration;

#[derive(Clone)]
pub struct Config {
    pub dns_server: String,
    pub dns_zone: String,
    pub key_name: String,
    pub key_alg: String,
    pub key_file: String,
    pub ttl: u32,
    pub check_interval: Duration,
    pub lookup_hostname: String,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Config {
            dns_server: env::var("DNS_SERVER")?,
            dns_zone: env::var("DNS_ZONE")?,
            key_name: env::var("KEY_NAME")?,
            key_alg: env::var("KEY_ALG").unwrap_or_else(|_| "hmac-sha256".to_string()),
            key_file: env::var("KEY_FILE")
                .unwrap_or_else(|_| "/run/secrets/rfc2136-secret".to_string()),
            ttl: env::var("TTL")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            check_interval: Duration::from_secs(
                env::var("CHECK_INTERVAL")
                    .unwrap_or_else(|_| "300".to_string())
                    .parse()
                    .unwrap_or(300),
            ),
            lookup_hostname: env::var("LOOKUP_HOSTNAME")?,
        })
    }
}

pub(crate) mod mock {
    use super::*;

    impl Default for Config {
        fn default() -> Self {
            Config {
                dns_server: String::from("127.0.0.1:53"),
                dns_zone: String::from("example.com"),
                key_name: String::from("example-com"),
                key_alg: String::from("hmac-sha256"),
                key_file: String::from("tests/fixtures/secret.key"),
                ttl: 300,
                check_interval: Duration::from_secs(300),
                lookup_hostname: String::from("thishost.example.com"),
            }
        }
    }
}
