# dee-amazon — Agent Guide

## Install
```bash
cargo install dee-amazon
```

## Commands
```bash
dee-amazon search <query> [--limit <n>] [--base-url <url>] [--json] [--quiet] [--verbose]
dee-amazon config set amazon.user-agent <value> [--json]
dee-amazon config set amazon.base-url <value> [--json]
dee-amazon config show [--json]
dee-amazon config path [--json]
```

## JSON Contract
- Success:
```json
{"ok":true,"count":1,"items":[{"id":"B001","title":"Product"}]}
```
- Error:
```json
{"ok":false,"error":"Invalid argument: ...","code":"INVALID_ARGUMENT"}
```

## Storage
- Config: `~/.config/dee-amazon/config.toml`
