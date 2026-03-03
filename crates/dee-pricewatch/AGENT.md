# dee-pricewatch — Agent Guide

## Install
```bash
cargo install dee-pricewatch
```

## Setup
- No API key required.
- Uses local SQLite storage.

## Commands
```bash
dee-pricewatch add <url> [--label <name>] [--target-price <price>] [--selector <css>] [--currency <code>] [--initial-price <price>] [--json] [--quiet] [--verbose]
dee-pricewatch list [--json] [--quiet] [--verbose]
dee-pricewatch check [watch] [--timeout-secs <seconds>] [--json] [--quiet] [--verbose]
dee-pricewatch delete <watch> [--json] [--quiet] [--verbose]
```

Examples:
```bash
dee-pricewatch add "https://example.com/p/123" --label "Sample" --target-price 15.00 --json
dee-pricewatch check --json
dee-pricewatch check 1 --json
dee-pricewatch list --json
```

## JSON Contract
- Success list:
```json
{"ok":true,"count":1,"items":[{"id":1,"label":"Sample","url":"https://example.com/p/123"}]}
```
- Success action:
```json
{"ok":true,"message":"Watch added","id":1}
```
- Error:
```json
{"ok":false,"error":"Watch not found","code":"NOT_FOUND"}
```

## Workflow
### Monitor a product and detect drops
```bash
dee-pricewatch add "https://example.com/product" --target-price 20 --json
dee-pricewatch check --json
dee-pricewatch list --json
```

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- JSON errors print to stdout.
- Non-JSON errors print to stderr.

## Storage
- Data: `~/.local/share/dee-pricewatch/pricewatch.db`
- Config: none
