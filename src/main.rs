use config::Config;
use dns_client::DnsClient;
use log::{error, info};
use tokio::{signal, sync::mpsc, task::JoinSet};

mod config;
mod dns_client;
mod dns_monitor;
mod docker_monitor;
mod registry;
mod system_monitor;

use dns_monitor::DnsMonitor;
use docker_monitor::DockerMonitor;
use system_monitor::SystemMonitor;

enum DnsUpdate {
    Host(String),
    IP(String),
}
type RxChannel = mpsc::Receiver<DnsUpdate>;
type TxChannel = mpsc::Sender<DnsUpdate>;

fn start() -> JoinSet<()> {
    let config = Config::from_env().unwrap();

    let (tx, rx) = mpsc::channel::<DnsUpdate>(20);
    let mut pool = JoinSet::new();

    // Watch for hostnames in the channel and update DNS
    let dns_client = DnsMonitor::new(&config);
    pool.spawn(async move {
        dns_client.monitor(rx).await.ok();
    });

    // Watch for docker container events and push hostnames into the update channel
    let docker_monitor = DockerMonitor::new(&tx);
    pool.spawn(async move {
        docker_monitor.monitor_events().await.ok();
    });

    // Watch the host system for IP changes and request IP updates for hosts
    let system_monitor = SystemMonitor::new(
        DnsClient::new(&config),
        config.lookup_hostname,
        config.check_interval,
        &tx,
    );
    pool.spawn(async move {
        system_monitor.monitor_system_dns().await.ok();
    });

    pool
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let mut handle = start();

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received");
            handle.shutdown().await;
        }
        Err(e) => error!("Failed to listen for shutdown signal: {}", e),
    }

    Ok(())
}
