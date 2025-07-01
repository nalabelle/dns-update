# dns-update

A Rust-based tool for managing DNS records across multiple providers, with initial support for NextDNS. This provider-agnostic tool allows you to update DNS records from various sources while maintaining a consistent interface.

## Overview

dns-update provides flexible DNS management through:

- Provider-agnostic architecture supporting multiple DNS services
- Strong type safety and async operations
- Automatic record type detection (A, AAAA, CNAME)
- Efficient record synchronization with providers
- Secure credential management through 1Password
- Thread-safe provider registry for extensibility

## Requirements

- Rust toolchain (2024 Edition)
- 1Password CLI (`op`)
- NextDNS account (for NextDNS provider)
- 1Password vault with:
  - NextDNS credentials (under "NextDNS" item)
  - DNS records (under "DNS Records" item)

## Installation

```bash
# Clone and build
git clone <repository-url>
cd dns-update
cargo build --release
```

## Basic Usage

```bash
# Update using records stored in 1Password
dns-update update

# Update using records from a file
dns-update update --file path/to/records.txt

# List available providers
dns-update providers list
```

The records file supports the following format:

```
# IP/hostname followed by the domain
1.2.3.4 example.com          # A record
2001:db8::1 ipv6.example.com # AAAA record
target.example.com cname.example.com # CNAME record
```

## Architecture

The tool uses a provider-agnostic architecture that allows support for multiple DNS services:

- **Core Components**: Provider trait, registry system, and record abstractions
- **NextDNS Provider**: Complete implementation for NextDNS API
- **Credential Management**: Secure integration with 1Password
- **Error Handling**: Comprehensive error types and handling

## Development

```bash
# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- update

# Format code
cargo fmt

# Run lints
cargo clippy
