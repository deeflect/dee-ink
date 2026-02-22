# dee-whois — Agent Guide

WHOIS lookup CLI for domains and IPs with agent-friendly JSON output.

## Install
```bash
cargo install --path .
```

## Setup
- No API key required.
- No local config file required.

## Binary names
- Primary: `dee-whois`
- Compatibility alias: `dee-whois`

## Commands
```bash
dee-whois <domain-or-ip> [--json] [--quiet] [--verbose]
dee-whois <domain> --raw [--json]
dee-whois <domain> --expires [--json]
```

## Examples
```bash
dee-whois example.com
dee-whois example.com --json
dee-whois example.com --raw
dee-whois example.com --expires --json
dee-whois 8.8.8.8 --json
dee-whois no-such-domain-deedee-zzzz.invalid --json
```

## JSON contracts

### Success (single item)
```json
{
  "ok": true,
  "item": {
    "domain": "example.com",
    "registrar": "Example Registrar",
    "created": "2000-01-01T00:00:00+00:00",
    "expires": "2027-01-01T00:00:00+00:00",
    "updated": "2026-01-01T00:00:00+00:00",
    "name_servers": ["ns1.example.com"],
    "status": ["clientdeleteprohibited"],
    "days_until_expiry": 311,
    "whois_server": "whois.verisign-grs.com"
  }
}
```

### Error
```json
{
  "ok": false,
  "error": "failed to connect to WHOIS server whois.nic.invalid: ...",
  "code": "NETWORK_ERROR"
}
```

## Common workflows
```bash
dee-whois example.com --json
dee-whois example.com --expires --json
dee-whois example.com --raw --json
```

## Behavior notes for agents
- No interactive prompts.
- Data to stdout; errors to stderr (unless `--json`, where errors are JSON on stdout).
- Exit code: `0` success, `1` failure.
- `--raw` and `--expires` are mutually exclusive.

## Server routing
- `.com`, `.net` → `whois.verisign-grs.com`
- `.org` → `whois.pir.org`
- `.io` → `whois.nic.io`
- `.co` → `whois.nic.co`
- other TLDs → `whois.nic.<tld>`
- IP addresses → `whois.arin.net`

## Storage
- No local database/config required for core lookup.
