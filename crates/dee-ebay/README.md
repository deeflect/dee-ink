# dee-ebay

Search eBay listings from the terminal.

## Install

```bash
cargo install dee-ebay
```

## Setup

```bash
dee-ebay config set ebay.client-id <ID>
dee-ebay config set ebay.client-secret <SECRET>
```

Optional sandbox mode:

```bash
dee-ebay config set ebay.sandbox true
```

## Usage

```bash
dee-ebay search <query> [--limit <n>] [--sort best-match|newly-listed|ending-soonest] [--json] [--quiet] [--verbose]
dee-ebay config set <key> <value> [--json]
dee-ebay config show [--json]
dee-ebay config path [--json]
```

## Examples

```bash
dee-ebay search "mechanical keyboard" --limit 10 --json
dee-ebay search "nintendo switch" --sort newly-listed --json
dee-ebay config show --json
```

## JSON Contract

Success list:

```json
{"ok":true,"count":1,"items":[{"id":"v1|123|0","title":"Sample Item","price":29.99,"currency":"USD","url":"https://www.ebay.com/..."}]}
```

Error:

```json
{"ok":false,"error":"Missing eBay credentials. Set ebay.client-id and ebay.client-secret","code":"AUTH_MISSING"}
```

## Storage

- Config: `~/.config/dee-ebay/config.toml`
- Data: none
