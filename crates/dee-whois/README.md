# dee-whois

WHOIS lookup for domains and IPs.

## Install

```sh
cargo install --path crates/dee-whois
```

## Quick start

```sh
dee-whois example.com
dee-whois example.com --json
dee-whois example.com --raw
dee-whois example.com --expires --json
dee-whois 8.8.8.8 --json
```

## Arguments and options

- Target: `<domain-or-ip>`
- `--raw` output raw WHOIS text
- `--expires` output expiry-focused view

## Agent-friendly output

Use `--json` for normalized records and error codes.

## Help

```sh
dee-whois --help
```
