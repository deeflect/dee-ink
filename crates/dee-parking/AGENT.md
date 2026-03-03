# dee-parking — Agent Guide

## Install
```bash
cargo install dee-parking
```

## Setup
```bash
dee-parking config set google.api-key <KEY>
```

## Commands
```bash
dee-parking search <location> [--query <text>] [--limit <n>] [--json] [--quiet] [--verbose]
dee-parking config set google.api-key <value> [--json]
dee-parking config set google.base-url <value> [--json]
dee-parking config show [--json]
dee-parking config path [--json]
```

## JSON Contract
- Success:
```json
{"ok":true,"count":1,"items":[{"name":"Garage","address":"123 Main St"}]}
```
- Error:
```json
{"ok":false,"error":"Missing Google API key. Set google.api-key via config set","code":"AUTH_MISSING"}
```

## Storage
- Config: `~/.config/dee-parking/config.toml`
