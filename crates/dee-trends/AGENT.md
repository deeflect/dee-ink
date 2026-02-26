# dee-trends — Agent Guide

## Install
```bash
cargo install dee-trends
```

## Setup
- No API key required.
- Requires outbound HTTPS access to Google Trends endpoints.
- Optional test override: `DEE_TRENDS_BASE_URL`.

## Quick Start
```bash
dee-trends interest "rust" --geo US --time "today 12-m" --json
```

## Commands
```bash
dee-trends interest <keyword> [--geo <code>] [--time <range>] [--hl en-US] [--tz 0] [--json] [--quiet] [--verbose]
dee-trends related <keyword> [--geo <code>] [--time <range>] [--hl en-US] [--tz 0] [--json] [--quiet] [--verbose]
dee-trends explore <keyword> [--geo <code>] [--time <range>] [--hl en-US] [--tz 0] [--json] [--quiet] [--verbose]
```

Examples:
```bash
dee-trends interest "openrouter" --json
dee-trends interest "llm" --geo US --time "today 3-m" --json
dee-trends related "claude" --geo US --json
dee-trends explore "agent tools" --json
```

## JSON Contract
- Success list:
```json
{"ok": true, "count": 2, "items": [{"timestamp": "1700000000", "formatted_time": "Nov 2023", "value": 42}]}
```
- Error:
```json
{"ok": false, "error": "Upstream API error", "code": "API_ERROR"}
```

## Common Workflows
### Workflow: Track interest trend for a term
```bash
dee-trends interest "rust" --geo US --time "today 12-m" --json
```

### Workflow: Discover related demand terms
```bash
dee-trends related "rust" --geo US --json
```

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- Non-JSON errors are printed to stderr as `error: <message>`.
- JSON errors are printed to stdout.

## Storage
- No local DB/config required.
