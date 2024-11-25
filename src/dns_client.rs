use hickory_client::client::{AsyncClient, ClientConnection, ClientHandle, Signer};
use hickory_client::proto::rr::dnssec::tsig::TSigner;
use hickory_client::rr::rdata::tsig::TsigAlgorithm;
use hickory_client::rr::IntoName;
use log::error;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;

use hickory_client::{
    op::{Message, OpCode, Query, ResponseCode, UpdateMessage},
    rr::{DNSClass, Name, RData, Record, RecordType},
    udp::UdpClientConnection,
};

use crate::config::Config;
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

    pub async fn fetch(&self, hostname: &str, record_type: RecordType) -> Option<String> {
        let hostname = self.normalize_hostname(hostname.clone());
        self.fetch_record(&hostname, record_type).await
    }

    pub async fn fetch_record(&self, hostname: &Name, record_type: RecordType) -> Option<String> {
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
            .map(|record| record.to_string());
    }

    pub async fn update_record(
        &self,
        hostname: &Name,
        record_type: RecordType,
        data: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut client = self.connect().await.unwrap();

        // Create zone and record names
        let hostname = Name::from_str(&format!("{}.{}", hostname, self.dns_zone))?;

        // Create update message
        let mut update = Message::default();
        update.set_op_code(OpCode::Update);

        // Set the zone in the query section
        let mut query = Query::default();
        query.set_name(self.dns_zone);
        query.set_query_class(DNSClass::IN);
        query.set_query_type(RecordType::SOA);
        update.add_query(query);

        // Delete any existing A records
        let mut delete_record = Record::with(hostname.clone(), RecordType::A, 0);
        delete_record.set_dns_class(DNSClass::NONE);
        update.add_updates(vec![delete_record]);

        // Add the new A record
        let mut record = Record::with(hostname, RecordType::A, self.ttl);
        record.set_data(Some(RData::A(data.parse()?)));
        update.add_updates(vec![record]);

        // Send the update and handle responses
        let responses = client.send(update);
        let response = responses
            .into_iter()
            .next()
            .ok_or("No response received")?
            .map_err(|e| format!("DNS update failed: {}", e))?;

        if response.response_code() == ResponseCode::NoError {
            info!("Successfully updated DNS record for {}", hostname);
            Ok(())
        } else {
            Err(format!("DNS update failed: {:?}", response.response_code()).into())
        }
    }
}
