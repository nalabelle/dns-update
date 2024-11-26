use crate::config::Config;
use hickory_client::client::{AsyncClient, ClientConnection, ClientHandle, Signer};
use hickory_client::proto::rr::dnssec::tsig::TSigner;
use hickory_client::rr::rdata::tsig::TsigAlgorithm;
use hickory_client::rr::{rdata, IntoName};
use log::error;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;

use hickory_client::{
    op::ResponseCode,
    rr::{DNSClass, Name, RData, Record, RecordType},
    udp::UdpClientConnection,
};

pub trait DnsFetchTrait {
    async fn fetch(&self, hostname: &str, record_type: RecordType) -> Option<String>;
}

pub struct DnsClient {
    name_server: SocketAddr,
    dns_zone: Name,
    signer: Arc<Signer>,
    ttl: u32,
}

impl DnsClient {
    pub fn new(config: &Config) -> Self {
        let key = std::fs::read(&config.key_file)
            .expect(&format!("Failed to read key file: {}", config.key_file));
        let name_server = config.dns_server.parse().expect(&format!(
            "Invalid DNS server address: {}",
            config.dns_server
        ));
        let algorithm = match config.key_alg.as_str() {
            "hmac-sha256" => Some(TsigAlgorithm::HmacSha256),
            _ => None,
        };
        if algorithm.is_none() {
            panic!("Unsupported key algorithm: {}", config.key_alg);
        };
        let signer = Signer::from(
            TSigner::new(
                key,
                algorithm.unwrap(),
                Name::from_utf8(&config.key_name).unwrap(),
                300,
            )
            .unwrap(),
        );
        let zone = Name::from_str(&config.dns_zone).unwrap();
        let ttl = config.ttl;
        Self {
            signer: Arc::new(signer),
            name_server,
            dns_zone: zone,
            ttl,
        }
    }

    pub fn normalize_hostname(&self, hostname: impl IntoName) -> Name {
        let hostname = hostname.into_name().unwrap();
        if hostname.is_fqdn() {
            return hostname.to_lowercase();
        }

        if let Ok(fqdn) = hostname.clone().append_domain(&self.dns_zone) {
            return fqdn.to_lowercase();
        }
        panic!("Failed to normalize hostname: {}", hostname);
    }

    async fn connect(&self) -> Option<AsyncClient> {
        let Ok(conn) = UdpClientConnection::new(self.name_server) else {
            error!("Failed to connect to DNS server: {}", self.name_server);
            return None;
        };
        let Ok((client, bg)) =
            AsyncClient::connect(conn.new_stream(Some(self.signer.clone()))).await
        else {
            error!("Failed to connect to DNS server: {}", self.name_server);
            return None;
        };
        tokio::spawn(bg);
        Some(client)
    }

    pub async fn fetch_record(&self, hostname: &Name, record_type: RecordType) -> Option<Record> {
        let mut client = self.connect().await?;
        let Ok(response) = client
            .query(hostname.clone(), DNSClass::IN, record_type)
            .await
        else {
            return None;
        };
        return response
            .answers()
            .iter()
            .find(|record| record.record_type() == record_type)
            .map(|record| record.clone());
    }

    fn build_rdata(record_type: RecordType, data: String) -> Option<RData> {
        let rdata = match record_type {
            RecordType::A => RData::A(data.parse().unwrap()),
            RecordType::TXT => RData::TXT(rdata::TXT::new(vec![data])),
            _ => {
                error!("Unsupported record type: {:?}", record_type);
                return None;
            }
        };
        Some(rdata)
    }

    pub async fn create_record(
        &self,
        hostname: &Name,
        record_type: RecordType,
        data: String,
    ) -> Option<bool> {
        let mut client = self.connect().await.unwrap();
        let mut record = Record::with(hostname.clone(), record_type, self.ttl);
        let rdata = DnsClient::build_rdata(record_type, data);
        record.set_data(rdata);
        client.create(record, self.dns_zone.clone()).await.ok()?;
        Some(true)
    }

    pub async fn update_record(
        &self,
        record: &Record,
        data: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut client = self.connect().await.unwrap();
        let mut update = record.clone();
        update.set_data(DnsClient::build_rdata(record.record_type(), data));

        // Send the update and handle responses
        let responses = client
            .compare_and_swap(record.clone(), update, self.dns_zone.clone())
            .await;
        let response = responses.into_iter().next().ok_or("No response received")?;

        if response.response_code() == ResponseCode::NoError {
            info!("Successfully updated DNS record for {}", record.name());
            Ok(())
        } else {
            Err(format!("DNS update failed: {:?}", response.response_code()).into())
        }
    }
}

impl DnsFetchTrait for DnsClient {
    async fn fetch(&self, hostname: &str, record_type: RecordType) -> Option<String> {
        let hostname = self.normalize_hostname(hostname);
        self.fetch_record(&hostname, record_type)
            .await
            .map(|record| record.data().unwrap().to_string())
    }
}

pub(crate) mod mock {
    use super::*;

    pub struct MockDnsClient {
        pub ip: String,
    }

    impl DnsFetchTrait for MockDnsClient {
        async fn fetch(&self, _hostname: &str, _record_type: RecordType) -> Option<String> {
            Some(self.ip.clone())
        }
    }

    impl MockDnsClient {
        pub fn new() -> Self {
            Self { ip: String::new() }
        }

        pub fn set_ip(&mut self, ip: String) {
            self.ip = ip;
        }
    }
}
