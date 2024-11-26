use crate::{config::Config, dns_client::DnsClient};
use crate::{DnsUpdate, RxChannel};
use futures_util::{stream::FuturesUnordered, StreamExt};
use log::error;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use hickory_client::rr::{Name, Record, RecordType};

pub struct DnsMonitor {
    dns: DnsClient,
    hosts: Arc<Mutex<HashMap<String, Name>>>,
    current_ip: Arc<Mutex<String>>,
}

impl DnsMonitor {
    pub fn new(config: &Config) -> Self {
        Self {
            dns: DnsClient::new(config),
            hosts: Arc::new(Mutex::new(HashMap::<String, Name>::new())),
            current_ip: Arc::new(Mutex::new(String::new())),
        }
    }

    pub async fn monitor(
        &self,
        mut rx: RxChannel,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Watch the hostname channel
        loop {
            while let Some(data) = rx.recv().await {
                match data {
                    DnsUpdate::Host(hostname) => {
                        self.update_host(hostname).await;
                    }
                    DnsUpdate::IP(ip) => {
                        self.set_current_ip(&ip).await.ok();
                        self.update_all_hostnames().await.ok();
                    }
                }
            }
        }
    }

    async fn set_current_ip(
        &self,
        ip: &String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut current_ip = self.current_ip.lock().await;
        *current_ip = ip.clone();
        Ok(())
    }

    async fn update_all_hostnames(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Update all tracked container records
        let hostnames = self.hosts.lock().await;
        let mut futures = FuturesUnordered::new();
        hostnames.keys().for_each(|hostname| {
            futures.push(self.update_host(hostname.clone()));
        });
        drop(hostnames);

        // FuturesUnordered run async and next returns whatever finishes next
        while let Some(()) = futures.next().await {}
        Ok(())
    }

    async fn normalized_hostname(&self, hostname: &String) -> Name {
        let mut map = self.hosts.lock().await;
        if let Some(data) = map.get(hostname) {
            return data.clone();
        }
        let normalized = self.dns.normalize_hostname(hostname);
        map.insert(hostname.clone(), normalized.clone());
        normalized
    }

    async fn update_host(&self, hostname: String) -> () {
        let hostname = self.normalized_hostname(&hostname).await;

        let ip_guard = self.current_ip.lock().await;
        let ip = ip_guard.clone();
        drop(ip_guard);

        let old_record = self.dns.fetch_record(&hostname, RecordType::A).await;
        let registry = Registry::new(hostname.clone(), &self.dns);
        if let Some(old_record) = old_record {
            // If there's an A record, verify that it's ours
            if !registry.host_in_registry().await {
                error!(
                    "Existing A record on hostname: {} is not in the registry",
                    hostname
                );
                return;
            }
            // Then update it
            self.dns.update_record(&old_record, ip).await.ok();
            return;
        } else {
            // If there's no A record, create a registry entry and a new A record
            registry.set_registry_txt().await.ok();
            self.dns.create_record(&hostname, RecordType::A, ip).await;
        }
    }
}

struct Registry<'a> {
    dns: &'a DnsClient,
    registry_hostname: Name,
    txt: String,
}

impl<'a> Registry<'a> {
    fn new(hostname: Name, dns: &'a DnsClient) -> Self {
        let txt = String::from("REGISTRY");
        let registry_hostname = Registry::get_registry_name(&hostname);
        Self {
            registry_hostname,
            txt,
            dns,
        }
    }

    fn get_registry_name(hostname: &Name) -> Name {
        let mut hostname_iterator = hostname.iter();
        let original_host = hostname_iterator.next().unwrap();
        let Ok(mut prefixed_host) = Name::from_str(&format!("{:?}_{}", original_host, "_registry"))
        else {
            panic!("Failed to create registry name for hostname: {}", hostname);
        };
        for label in hostname_iterator {
            prefixed_host = prefixed_host.append_label(label).unwrap();
        }
        prefixed_host
    }

    pub async fn host_in_registry(&self) -> bool {
        let txt: Option<String> = self
            .get_registry_txt()
            .await
            .map(|record| record.data().unwrap().as_txt().unwrap().to_string());
        txt.is_some() && txt.unwrap() == self.txt
    }

    async fn get_registry_txt(&self) -> Option<Record> {
        self.dns
            .fetch_record(&self.registry_hostname, RecordType::TXT)
            .await
    }

    async fn set_registry_txt(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.dns
            .create_record(&self.registry_hostname, RecordType::TXT, self.txt.clone())
            .await;
        Ok(())
    }
}
