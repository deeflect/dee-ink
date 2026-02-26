# dee-trends

Google Trends CLI for interest-over-time and related queries with JSON output.

## Install

```sh
cargo install dee-trends
```

## Quick start

```sh
dee-trends interest "rust" --geo US --time "today 12-m" --json
dee-trends related "llm" --json
```

## Commands

- `interest <keyword> [--geo <code>] [--time <range>] [--hl <lang>] [--tz <minutes>]`
- `related <keyword> [--geo <code>] [--time <range>] [--hl <lang>] [--tz <minutes>]`
- `explore <keyword> [--geo <code>] [--time <range>] [--hl <lang>] [--tz <minutes>]`

## Agent-friendly output

Use `--json` on every command.

Success list:

```json
{"ok":true,"count":3,"items":[...]}
```

Error:

```json
{"ok":false,"error":"Upstream API error","code":"API_ERROR"}
```

## Notes

- Uses Google Trends web API endpoints.
- Endpoint behavior can change; retries and error handling are important for automation.

## Help

```sh
dee-trends --help
dee-trends <command> --help
```
