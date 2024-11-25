use crate::{config::Config, dns_client::DnsClient};
use crate::{DnsUpdate, RxChannel};
use futures_util::{stream::FuturesUnordered, StreamExt};
use log::error;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use hickory_client::rr::{Name, RecordType};

pub struct DnsMonitor {
    dns: DnsClient,
    hosts: Arc<Mutex<HashMap<String, Name>>>,
    current_ip: Arc<Mutex<String>>,
    ttl: u32,
}

impl DnsMonitor {
    pub fn new(config: &Config) -> Self {
        Self {
            dns: DnsClient::new(config),
            hosts: Arc::new(Mutex::new(HashMap::<String, Name>::new())),
            current_ip: Arc::new(Mutex::new(String::new())),
            ttl: config.ttl,
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
                        self.set_current_ip(&ip).await;
                        self.update_all_hostnames().await;
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
        let old_a_record = self.dns.fetch_record(&hostname, RecordType::A).await;
        if old_a_record.is_some() {
            let results = self.set_registry_txt(&hostname).await;
            if results.is_err() {
                error!(
                    "Unable to set registry string for existing A record on hostname: {} - {}",
                    hostname,
                    results.unwrap_err()
                );
                return;
            }
        }

        let ip_guard = self.current_ip.lock().await;
        let ip = ip_guard.clone();
        drop(ip_guard);

        self.dns.update_record(&hostname, RecordType::A, ip).await;
        // let hostname = normalize_hostname(hostname, &self.dns_zone);
        // self.hostnames.lock().await.insert(hostname);
    }

    fn get_registry_name<'a>(&self, hostname: &Name) -> Name {
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

    async fn set_registry_txt(&self, hostname: &Name) -> Result<(), Box<dyn std::error::Error>> {
        let txt = String::from("REGISTRY");
        let registry_hostname = self.get_registry_name(hostname);
        let old_txt = self
            .dns
            .fetch_record(&registry_hostname, RecordType::TXT)
            .await;
        if old_txt.is_some() && old_txt.unwrap() == txt {
            return Ok(());
        }

        self.dns
            .update_record(&registry_hostname, RecordType::TXT, txt)
            .await;
        Ok(())
    }
}
