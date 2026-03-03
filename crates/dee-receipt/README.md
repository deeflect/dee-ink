# dee-receipt

Extract structured receipt JSON from an image.

## Install

```bash
cargo install dee-receipt
```

## Setup

```bash
dee-receipt config set openai.api-key <KEY>
```

Optional base URL:

```bash
dee-receipt config set openai.base-url https://api.openai.com/v1
```

## Usage

```bash
dee-receipt scan <image> [--model <name>] [--prompt <text>] [--json] [--quiet] [--verbose]
dee-receipt config set <key> <value> [--json]
dee-receipt config show [--json]
dee-receipt config path [--json]
```

## Examples

```bash
dee-receipt scan ./receipt.jpg --json
dee-receipt scan ./receipt.png --model gpt-4o-mini --json
dee-receipt config show --json
```

## JSON Contract

Success:

```json
{"ok":true,"item":{"merchant":"Coffee Shop","date":"2026-03-01","currency":"USD","total":12.5,"line_items":[{"name":"Latte","qty":1,"unit_price":5.0,"total":5.0}],"parsed_at":"2026-03-01T00:00:00Z"}}
```

Error:

```json
{"ok":false,"error":"Missing OpenAI API key. Set openai.api-key via config set","code":"AUTH_MISSING"}
```

## Storage

- Config: `~/.config/dee-receipt/config.toml`
- Data: none
