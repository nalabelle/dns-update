use crate::core::record::DNSRecord;
use crate::error::Error;
use async_trait::async_trait;

#[async_trait]
pub trait DNSProvider: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &str;
    async fn list_records(&self) -> Result<Vec<DNSRecord>, Error>;
    async fn add_record(&self, record: DNSRecord) -> Result<(), Error>;
    #[allow(dead_code)]
    async fn update_record(&self, record: DNSRecord) -> Result<(), Error>;
    async fn delete_record(&self, record: DNSRecord) -> Result<(), Error>;
}
