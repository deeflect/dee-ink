# dee-ssl — Agent Guide

SSL certificate inspection CLI for domains.

## Install
```bash
cargo install --path .
# binary: dee-ssl
```

## Setup
- No API key required.
- No local config file required.

## Commands
```bash
dee-ssl check <domain>
dee-ssl check <domain> --chain
dee-ssl check <domain> --warn-days 30
dee-ssl check <domain> --timeout-secs 5
```

## Global flags
```bash
-j, --json
-q, --quiet
-v, --verbose
```

## JSON contracts
### Success (single)
```json
{"ok":true,"item":{"domain":"example.com","port":443,"valid":true,"expires":"2026-05-14T18:57:50Z","days_until_expiry":78,"issuer":"...","subject":"...","sans":["example.com"],"chain_depth":3}}
```

### Success (list / chain)
```json
{"ok":true,"count":3,"items":[{"index":0,"subject":"...","issuer":"...","not_before":"2025-01-01T00:00:00Z","not_after":"2026-01-01T00:00:00Z"}]}
```

### Error
```json
{"ok":false,"error":"...","code":"TLS_HANDSHAKE_FAILED"}
```

## Common workflows
```bash
dee-ssl check example.com --json
dee-ssl check example.com --chain --json
dee-ssl check example.com --warn-days 30 --json
```

## Notes
- Exit code `1` on failure.
- `--warn-days N` returns `EXPIRING_SOON` when certificate expiry is within threshold.
- `--timeout-secs N` (default `10`) controls the TLS handshake timeout.
- No interactive prompts.

## Storage
- Data: none
- Config: none
