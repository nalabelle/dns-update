use crate::dns_client::DnsClient;
use hickory_client::rr::Name;
use hickory_client::rr::Record;
use hickory_client::rr::RecordType;

pub(crate) struct Registry<'a> {
    pub(crate) dns: &'a DnsClient,
    pub(crate) registry_hostname: Name,
    pub(crate) txt: String,
}

impl<'a> Registry<'a> {
    pub(crate) fn new(hostname: Name, dns: &'a DnsClient) -> Self {
        let txt = String::from("REGISTRY");
        let registry_hostname = Registry::get_registry_name(&hostname);
        Self {
            registry_hostname,
            txt,
            dns,
        }
    }

    pub(crate) fn get_registry_name(hostname: &Name) -> Name {
        let mut labels: Vec<_> = hostname.iter().collect();
        let registry_host = [labels[0], b"_registry"].concat();
        labels[0] = registry_host.as_slice();

        let Ok(prefixed_host) = Name::from_labels(labels) else {
            panic!("Failed to create registry name for hostname: {}", hostname);
        };

        prefixed_host
    }

    pub async fn host_in_registry(&self) -> bool {
        let txt: Option<String> = self
            .get_registry_txt()
            .await
            .map(|record| record.data().unwrap().as_txt().unwrap().to_string());
        txt.is_some() && txt.unwrap() == self.txt
    }

    pub(crate) async fn get_registry_txt(&self) -> Option<Record> {
        self.dns
            .fetch_record(&self.registry_hostname, RecordType::TXT)
            .await
    }

    pub(crate) async fn set_registry_txt(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.dns
            .create_record(&self.registry_hostname, RecordType::TXT, self.txt.clone())
            .await;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::str::FromStr;

    use super::*;
    use hickory_client::rr::Name;

    #[test]
    fn test_get_registry_name() {
        let hostname = Name::from_str("test.example.com.").unwrap();
        let registry_name = Registry::get_registry_name(&hostname);
        assert_eq!(registry_name.to_string(), "test_registry.example.com.");
    }
}
