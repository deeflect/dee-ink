# dee-amazon

Search Amazon products from the terminal.

## Install

```bash
cargo install dee-amazon
```

## Usage

```bash
dee-amazon search <query> [--limit <n>] [--base-url <url>] [--json] [--quiet] [--verbose]
dee-amazon config set amazon.user-agent <value> [--json]
dee-amazon config set amazon.base-url <value> [--json]
dee-amazon config show [--json]
dee-amazon config path [--json]
```

## Examples

```bash
dee-amazon search "mechanical keyboard" --limit 10 --json
dee-amazon config set amazon.user-agent "dee-amazon/0.1"
dee-amazon config show --json
```

## Notes

This tool parses Amazon HTML search results. Amazon may rate-limit or challenge automated requests.

## JSON Contract

Success list:

```json
{"ok":true,"count":1,"items":[{"id":"B001","title":"Test Keyboard","price":99.99,"currency":"USD","rating":4.5,"review_count":1234,"url":"https://www.amazon.com/dp/B001"}]}
```

Error:

```json
{"ok":false,"error":"Invalid argument: invalid base url 'notaurl'","code":"INVALID_ARGUMENT"}
```

## Storage

- Config: `~/.config/dee-amazon/config.toml`
- Data: none
