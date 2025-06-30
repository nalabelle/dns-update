# nextdns_rewrite

A Python tool to manage DNS rewrites in NextDNS profiles. This tool allows you to update DNS rewrites either from a file or from entries stored in 1Password.

## Overview

nextdns_rewrite helps manage DNS rewrites for NextDNS by:

- Reading rewrites from a file or 1Password entry
- Automatically detecting record types (A, AAAA, or CNAME)
- Efficiently updating NextDNS profiles by only changing what's needed
- Securely managing credentials through 1Password

## Requirements

- Python 3
- 1Password CLI (`op`)
- NextDNS account
- 1Password vault with:
  - NextDNS credentials (under "NextDNS" item)
  - DNS rewrites (under "DNS Rewrites" item)

## Basic Usage

```bash
# Update using rewrites stored in 1Password
./rewrite.py

# Update using rewrites from a file
./rewrite.py path/to/rewrites.txt
```

The rewrites file should contain entries in the format:

```
# IP/hostname followed by the domain
1.2.3.4 example.com
2001:db8::1 ipv6.example.com
target.example.com cname.example.com
