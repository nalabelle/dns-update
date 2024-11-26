use std::sync::Arc;

use hickory_client::rr::RecordType;
use log::{error, info};
use tokio::sync::Mutex;
use tokio::time;

use crate::dns_client::DnsFetchTrait;
use crate::{DnsUpdate, TxChannel};
pub struct SystemMonitor<D: DnsFetchTrait> {
    dns: D,
    hostname: String,
    check_interval: time::Duration,
    current_ip: Arc<Mutex<String>>,
    tx: TxChannel,
}

impl<D: DnsFetchTrait> SystemMonitor<D> {
    pub fn new(dns: D, hostname: String, check_interval: time::Duration, tx: &TxChannel) -> Self {
        Self {
            dns,
            hostname,
            check_interval,
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

        self.tx.send(DnsUpdate::IP(current_ip)).await.ok();
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

#[cfg(test)]
mod tests {
    use crate::dns_client::mock::MockDnsClient;
    use std::time::Duration;
    use tokio::sync::mpsc;

    use super::*;

    #[tokio::test]
    async fn test_check_host_ip_no_change() {
        let (tx, mut rx) = mpsc::channel(1);
        let mut dns_client = MockDnsClient::new();
        dns_client.set_ip("1.2.3.4".to_string());

        let monitor = SystemMonitor::new(
            dns_client,
            "hostname".to_string(),
            Duration::from_secs(60),
            &tx,
        );
        monitor.current_ip.lock().await.push_str("1.2.3.4");

        monitor.check_host_ip().await;

        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_check_host_ip_change() {
        let (tx, mut rx) = mpsc::channel(1);
        let mut dns_client = MockDnsClient::new();
        dns_client.set_ip("1.2.3.4".to_string());

        let monitor = SystemMonitor::new(
            dns_client,
            "hostname".to_string(),
            Duration::from_secs(60),
            &tx,
        );
        monitor.current_ip.lock().await.push_str("4.3.2.1");

        monitor.check_host_ip().await;

        let update = rx.recv().await.unwrap();
        if let DnsUpdate::IP(ip) = update {
            assert_eq!(ip, "1.2.3.4");
        } else {
            panic!("Unexpected update type");
        }
    }

    #[tokio::test]
    async fn test_monitor_system_dns() {
        let (tx, mut rx) = mpsc::channel(1);
        let mut dns_client = MockDnsClient::new();
        dns_client.set_ip("1.2.3.4".to_string());

        let monitor = SystemMonitor::new(
            dns_client,
            "hostname".to_string(),
            Duration::from_secs(60),
            &tx,
        );
        monitor.current_ip.lock().await.push_str("4.3.2.1");

        let monitor_handle = tokio::spawn(async move {
            monitor.monitor_system_dns().await.unwrap();
        });

        let update = rx.recv().await.unwrap();
        if let DnsUpdate::IP(ip) = update {
            assert_eq!(ip, "1.2.3.4");
        } else {
            panic!("Unexpected update type");
        }

        monitor_handle.abort();
    }
}
