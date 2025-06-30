//! NextDNS provider implementation

pub mod client;
pub mod error;
pub mod types;

pub use client::{NextDNSConfig, NextDNSProvider};
pub use error::NextDNSProviderError;
pub use types::{CreateRecordRequest, NextDNSRecord};
// --- DNSProvider trait implementation for NextDNSProvider ---
use crate::core::provider::DNSProvider;
use crate::core::record::{DNSRecord, DNSRecordType};
use crate::error::Error;
use async_trait::async_trait;

fn to_dns_record(nr: &NextDNSRecord) -> DNSRecord {
    DNSRecord {
        record_type: match nr.record_type.as_str() {
            "A" => DNSRecordType::A,
            "AAAA" => DNSRecordType::AAAA,
            "CNAME" => DNSRecordType::CNAME,
            _ => DNSRecordType::A, // fallback, should handle error
        },
        name: nr.domain.clone(),
        value: nr.value.clone(),
        ttl: nr.ttl,
    }
}

fn to_nextdns_record(rec: &DNSRecord) -> CreateRecordRequest {
    CreateRecordRequest {
        domain: rec.name.clone(),
        record_type: match rec.record_type {
            DNSRecordType::A => "A".to_string(),
            DNSRecordType::AAAA => "AAAA".to_string(),
            DNSRecordType::CNAME => "CNAME".to_string(),
        },
        value: rec.value.clone(),
        ttl: rec.ttl,
    }
}

fn map_error(e: NextDNSProviderError) -> Error {
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

#[async_trait]
impl DNSProvider for NextDNSProvider {
    fn name(&self) -> &str {
        "nextdns"
    }

    async fn list_records(&self) -> Result<Vec<DNSRecord>, Error> {
        self.list_rewrites()
            .await
            .map(|v| v.into_iter().map(|r| to_dns_record(&r)).collect())
            .map_err(map_error)
    }

    async fn add_record(&self, record: DNSRecord) -> Result<(), Error> {
        let req = to_nextdns_record(&record);
        self.create_rewrite(&req)
            .await
            .map(|_| ())
            .map_err(map_error)
    }

    async fn update_record(&self, record: DNSRecord) -> Result<(), Error> {
        // NextDNS needs record id, so we must fetch all and match
        let records = self.list_rewrites().await.map_err(map_error)?;
        if let Some(existing) = records
            .iter()
            .find(|r| r.domain == record.name && r.value == record.value)
        {
            let req = to_nextdns_record(&record);
            self.update_rewrite(&existing.id, &req)
                .await
                .map(|_| ())
                .map_err(map_error)
        } else {
            Err(Error::NotFound("Record not found".to_string()))
        }
    }

    async fn delete_record(&self, record: DNSRecord) -> Result<(), Error> {
        let records = self.list_rewrites().await.map_err(map_error)?;
        if let Some(existing) = records
            .iter()
            .find(|r| r.domain == record.name && r.value == record.value)
        {
            self.delete_rewrite(&existing.id).await.map_err(map_error)
        } else {
            Err(Error::NotFound("Record not found".to_string()))
        }
    }
}
