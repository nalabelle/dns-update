use crate::core::provider::DNSProvider;
// Module declarations for binary crate
mod auth;
mod core;
mod error;
mod onepassword;
mod providers;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::Arc;

use crate::auth::credentials::{CredentialManager, OnePasswordCredentialManager};
use crate::core::record::{DNSRecord, DNSRecordType};
use crate::onepassword::OnePasswordClient;
use crate::providers::nextdns::{NextDNSConfig, NextDNSProvider};

#[tokio::main]
async fn main() {
    // Parse optional file argument
    let args: Vec<String> = env::args().collect();
    let file_arg = args.get(1);

    // 1Password client and credential manager
    let op_client = Arc::new(OnePasswordClient::new("Applications"));
    let creds = Arc::new(OnePasswordCredentialManager::new(op_client.clone()));

    // Load config from 1Password
    let config = match creds.get("nextdns_profile_id") {
        Ok(profile_id) => NextDNSConfig {
            profile_id,
            api_url: "https://api.nextdns.io".to_string(),
        },
        Err(e) => {
            eprintln!("Failed to load NextDNS profile ID: {e}");
            return;
        }
    };

    // Create provider
    let provider = match NextDNSProvider::new(config, creds.clone()).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create provider: {e:?}");
            return;
        }
    };

    // Read rewrites
    let desired_records: Vec<DNSRecord> = if let Some(file_path) = file_arg {
        match read_rewrites_from_file(file_path) {
            Ok(records) => records,
            Err(e) => {
                eprintln!("Failed to read rewrites from file: {e}");
                return;
            }
        }
    } else {
        // Read rewrites from 1Password
        match op_client.get_dns_rewrites().await {
            Ok(raw) => match parse_rewrites_from_str(&raw) {
                Ok(records) => records,
                Err(e) => {
                    eprintln!("Failed to parse rewrites from 1Password: {e}");
                    return;
                }
            },
            Err(e) => {
                eprintln!("Failed to read rewrites from 1Password: {e}");
                return;
            }
        }
    };

    // Fetch current records
    let current_records = match provider.list_records().await {
        Ok(records) => records,
        Err(e) => {
            eprintln!("Failed to list current records: {e:?}");
            return;
        }
    };

    // Compute changes
    let to_add: Vec<_> = desired_records
        .iter()
        .filter(|r| !current_records.contains(r))
        .cloned()
        .collect();
    let to_remove: Vec<_> = current_records
        .iter()
        .filter(|r| !desired_records.contains(r))
        .cloned()
        .collect();

    // Apply changes
    for record in &to_add {
        println!("Adding: {record:?}");
        if let Err(e) = provider.add_record(record.clone()).await {
            eprintln!("Failed to add record: {e:?}");
        }
    }
    for record in &to_remove {
        println!("Removing: {record:?}");
        if let Err(e) = provider.delete_record(record.clone()).await {
            eprintln!("Failed to remove record: {e:?}");
        }
    }
}

// Parse rewrite file lines into DNSRecord
fn read_rewrites_from_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<DNSRecord>> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);
    use std::iter::Iterator;
    parse_rewrites_from_iter(reader.lines().map_while(Result::ok))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

// Parse DNS rewrites from a string (1Password)
fn parse_rewrites_from_str(s: &str) -> Result<Vec<DNSRecord>, String> {
    let lines = s
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    parse_rewrites_from_iter(lines).map_err(|e| format!("Failed to parse rewrites: {e}"))
}

// Shared parser for lines
fn parse_rewrites_from_iter<I>(lines: I) -> Result<Vec<DNSRecord>, String>
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    let mut records = Vec::new();
    for line in lines {
        let line = line.as_ref();
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 {
            continue;
        }
        let (value, name) = (parts[0], parts[1]);
        let record_type = if value.parse::<std::net::Ipv4Addr>().is_ok() {
            DNSRecordType::A
        } else if value.parse::<std::net::Ipv6Addr>().is_ok() {
            DNSRecordType::AAAA
        } else {
            DNSRecordType::CNAME
        };
        records.push(DNSRecord {
            record_type,
            name: name.to_string(),
            value: value.to_string(),
            ttl: Some(300),
        });
    }
    Ok(records)
}
