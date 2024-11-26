use bollard::models::EventMessage;
use bollard::system::EventsOptions;
use bollard::Docker;
use futures_util::StreamExt;
use log::{debug, info, warn};

use crate::{DnsUpdate, TxChannel};

pub struct DockerMonitor {
    docker: Docker,
    tx: TxChannel,
}

impl DockerMonitor {
    pub fn new(tx: &TxChannel) -> Self {
        let docker =
            Docker::connect_with_local_defaults().expect("Failed to connect to Docker daemon");

        Self {
            docker,
            tx: tx.clone(),
        }
    }

    fn extract_hostname(event: EventMessage) -> Option<String> {
        let Some(actor) = event.actor else {
            debug!("No actor found in event");
            return None;
        };
        let Some(attrs) = actor.attributes else {
            debug!("No attributes found in actor");
            return None;
        };

        // If traefik isn't enabled, we don't need to update DNS
        let traefik_enabled = attrs
            .iter()
            .any(|(key, value)| key == "traefik.enable" && value == "true");
        if !traefik_enabled {
            info!("Traefik is not enabled on this container");
            return None;
        }

        // Extract the router rule value and parse the hostname
        let traefik_hostname = attrs
            .iter()
            .find(|(key, _)| key.starts_with("traefik.http.routers") && key.ends_with(".rule"))
            .and_then(|(_, value)| {
                // Extract content between backticks: Host(`example.com`) -> example.com
                value.split('`').nth(1)
            });

        let container_name = attrs.get("name").map(|s| s.as_str());
        let Some(hostname) = traefik_hostname.or(container_name) else {
            warn!("No hostname found in event");
            return None;
        };

        return Some(hostname.to_owned());
    }

    pub async fn monitor_events(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut filters = std::collections::HashMap::new();
        filters.insert("type", vec!["container"]);
        filters.insert("event", vec!["start"]);

        let options = EventsOptions {
            filters,
            ..Default::default()
        };

        let mut events = self.docker.events(Some(options));

        while let Some(Ok(msg)) = events.next().await {
            let event = msg.clone();
            let Some(action) = msg.action else {
                continue;
            };
            let tx = self.tx.clone();
            match action.as_str() {
                "start" => {
                    tokio::spawn(async move {
                        if let Some(hostname) = DockerMonitor::extract_hostname(event) {
                            tx.send(DnsUpdate::Host(hostname)).await.ok();
                        }
                    });
                }
                _ => {}
            }
        }
        return Ok(());
    }
}
