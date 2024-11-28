use crate::registry::Registry;
use crate::{config::Config, dns_client::DnsClient};
use crate::{DnsUpdate, RxChannel};
use futures_util::{stream::FuturesUnordered, StreamExt};
use log::error;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use hickory_client::rr::{Name, RecordType};

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
