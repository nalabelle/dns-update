use std::sync::Arc;

use hickory_client::rr::RecordType;
use log::{error, info};
use tokio::sync::Mutex;
use tokio::time;

use crate::config::Config;
use crate::dns_client::DnsClient;
use crate::{DnsUpdate, TxChannel};
pub struct SystemMonitor {
    dns: DnsClient,
    hostname: String,
    check_interval: time::Duration,
    current_ip: Arc<Mutex<String>>,
    tx: TxChannel,
}

impl SystemMonitor {
    pub fn new(config: &Config, tx: &TxChannel) -> Self {
        let dns = DnsClient::new(config);
        Self {
            dns,
            hostname: config.lookup_hostname.clone(),
            check_interval: config.check_interval,
            current_ip: Arc::new(Mutex::new(String::new())),
            tx: tx.clone(),
        }
    }

    // Returns true if the system IP has changed
    async fn check_host_ip(&self) -> () {
        let hostname = &self.hostname;
        let Some(current_ip) = self.dns.fetch(hostname, RecordType::A).await else {
            error!("Couldn't look up A record for host {}", hostname);
            return;
        };

        let mut instance_ip = self.current_ip.lock().await;
        if current_ip == *instance_ip {
            return;
        }

        let last_ip = instance_ip.clone();
        *instance_ip = current_ip.clone();
        info!(
            "System hostname IP changed from {} to {}",
            last_ip, current_ip
        );

        self.tx.send(DnsUpdate::IP(current_ip)).await;
    }

    // Set up a recurring task to check the system IP
    pub async fn monitor_system_dns(&self) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
        let check_interval = self.check_interval;
        loop {
            self.check_host_ip().await;
            time::sleep(check_interval).await;
        }
    }
}
