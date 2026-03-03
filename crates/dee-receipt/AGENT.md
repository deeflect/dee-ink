# dee-receipt — Agent Guide

## Install
```bash
cargo install dee-receipt
```

## Setup
```bash
dee-receipt config set openai.api-key <KEY>
```

## Commands
```bash
dee-receipt scan <image> [--model <name>] [--prompt <text>] [--json] [--quiet] [--verbose]
dee-receipt config set openai.api-key <value> [--json]
dee-receipt config set openai.base-url <url> [--json]
dee-receipt config show [--json]
dee-receipt config path [--json]
```

Examples:
```bash
dee-receipt scan ./receipt.jpg --json
dee-receipt config show --json
```

## JSON Contract
- Success:
```json
{"ok":true,"item":{"merchant":"Store","total":42.1,"currency":"USD","parsed_at":"2026-03-01T00:00:00Z"}}
```
- Error:
```json
{"ok":false,"error":"Missing OpenAI API key. Set openai.api-key via config set","code":"AUTH_MISSING"}
```

## Error Handling
- Exit code `0` on success.
- Exit code `1` on error.
- JSON errors print to stdout.

## Storage
- Config: `~/.config/dee-receipt/config.toml`
