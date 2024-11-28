# DNS Update

DNS Update is a Rust-based application that monitors system IP changes, Docker container events, and
updates DNS records accordingly. It uses the `hickory-client` library for DNS operations and
`bollard` for Docker event monitoring.

## Features

- Monitors system IP changes and updates DNS records.
- Monitors Docker container events and updates DNS records for containers with Traefik enabled.
- Configurable via environment variables.

## Configuration

The application is configured using environment variables. Below is a list of the required and
optional environment variables:

### Environment Variables

See .env-dist for an example.

- `DNS_UPDATE_CHECK_INTERVAL`: The interval in seconds to check for IP changes (default: `300`).
- `DNS_UPDATE_DNS_SERVER`: The address of the DNS server to update.
- `DNS_UPDATE_DNS_ZONE`: The DNS zone to update.
- `DNS_UPDATE_KEY_ALG`: The algorithm for the TSIG key (default: `hmac-sha256`).
- `DNS_UPDATE_KEY_FILE`: The file path to the TSIG key (default: `/run/secrets/rfc2136-secret`).
- `DNS_UPDATE_KEY_NAME`: The name of the TSIG key.
- `DNS_UPDATE_LOOKUP_HOSTNAME`: The hostname to monitor for IP changes.
- `DNS_UPDATE_TTL`: The TTL for DNS records (default: `300`).
