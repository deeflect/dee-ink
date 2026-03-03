# dee-ebay — Agent Guide

## Install
```bash
cargo install dee-ebay
```

## Setup
```bash
dee-ebay config set ebay.client-id <ID>
dee-ebay config set ebay.client-secret <SECRET>
```

## Commands
```bash
dee-ebay search <query> [--limit <n>] [--sort best-match|newly-listed|ending-soonest] [--json] [--quiet] [--verbose]
dee-ebay config set ebay.client-id <value> [--json]
dee-ebay config set ebay.client-secret <value> [--json]
dee-ebay config set ebay.sandbox true|false [--json]
dee-ebay config show [--json]
dee-ebay config path [--json]
```

## JSON Contract
- Success:
```json
{"ok":true,"count":1,"items":[{"id":"v1|...","title":"Item","price":10.0,"currency":"USD"}]}
```
- Error:
```json
{"ok":false,"error":"Missing eBay credentials. Set ebay.client-id and ebay.client-secret","code":"AUTH_MISSING"}
```

## Storage
- Config: `~/.config/dee-ebay/config.toml`
