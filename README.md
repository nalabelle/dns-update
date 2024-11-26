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

- `CHECK_INTERVAL`: The interval in seconds to check for IP changes (default: `300`).
- `DNS_SERVER`: The address of the DNS server to update.
- `DNS_ZONE`: The DNS zone to update.
- `KEY_ALG`: The algorithm for the TSIG key (default: `hmac-sha256`).
- `KEY_FILE`: The file path to the TSIG key (default: `/run/secrets/rfc2136-secret`).
- `KEY_NAME`: The name of the TSIG key.
- `LOOKUP_HOSTNAME`: The hostname to monitor for IP changes.
- `TTL`: The TTL for DNS records (default: `300`).

### Example Configuration

```sh
export DNS_SERVER="8.8.8.8"
export DNS_ZONE="example.com"
export KEY_NAME="my-key"
export LOOKUP_HOSTNAME="my-hostname.example.com"
export KEY_ALG="hmac-sha256"
export KEY_FILE="/path/to/keyfile"
export TTL="300"
export CHECK_INTERVAL="300"
```
