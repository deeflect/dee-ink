# dee-pricewatch

Monitor webpage prices and detect drops.

## Install

```bash
cargo install dee-pricewatch
```

## Usage

```bash
dee-pricewatch add <url> [--label <name>] [--target-price <price>] [--selector <css>] [--currency <code>] [--initial-price <price>] [--json] [--quiet] [--verbose]
dee-pricewatch list [--json] [--quiet] [--verbose]
dee-pricewatch check [watch] [--timeout-secs <seconds>] [--json] [--quiet] [--verbose]
dee-pricewatch delete <watch> [--json] [--quiet] [--verbose]
```

## Examples

```bash
dee-pricewatch add "https://example.com/product" --label "Desk Lamp" --target-price 29.99 --json
dee-pricewatch list --json
dee-pricewatch check --json
dee-pricewatch check 1 --json
dee-pricewatch delete 1 --json
```

## JSON Contract

Success list:

```json
{"ok":true,"count":1,"items":[{"id":1,"url":"https://example.com/product","label":"Desk Lamp","target_price":29.99,"created_at":"2026-03-01T00:00:00Z","updated_at":"2026-03-01T00:00:00Z"}]}
```

Success action:

```json
{"ok":true,"message":"Watch added","id":1}
```

Error:

```json
{"ok":false,"error":"Watch not found","code":"NOT_FOUND"}
```

## Storage

- Data: `~/.local/share/dee-pricewatch/pricewatch.db`
- Config: none
