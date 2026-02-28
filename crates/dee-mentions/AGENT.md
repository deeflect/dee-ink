# dee-mentions — Agent Guide

## Install
```bash
cargo install dee-mentions
```

## Setup
- No API key required for current source set.
- Requires outbound HTTPS access for live checks.
- Optional source base URL overrides for testing:
  - `DEE_MENTIONS_HN_BASE`
  - `DEE_MENTIONS_REDDIT_BASE`

## Quick Start
```bash
dee-mentions check "dee.ink" --sources hn,reddit --limit 5 --json
```

## Commands
```bash
dee-mentions check <query> [--sources hn,reddit] [--limit <n>] [--json] [--quiet] [--verbose]
dee-mentions run --all|--id <watch-id> [--sources hn,reddit] [--limit <n>] [--json] [--quiet] [--verbose]
dee-mentions watch add <query> [--tag <tag>] [--sources hn,reddit] [--json] [--quiet] [--verbose]
dee-mentions watch list [--json] [--quiet] [--verbose]
dee-mentions watch remove <id> [--json] [--quiet] [--verbose]
```

Examples:
```bash
dee-mentions check "openrouter" --sources hn --json
dee-mentions watch add "dee.ink" --tag brand --json
dee-mentions watch list --json
dee-mentions run --all --limit 5 --json
dee-mentions watch remove 1 --json
```

## JSON Contract
- Success list:
```json
{"ok": true, "count": 2, "items": [{"source": "hn", "query": "dee.ink", "title": "Launch post", "url": "https://...", "created_at": "2026-02-26T00:00:00Z"}]}
```
- Success action:
```json
{"ok": true, "message": "Watch added", "id": 1}
```
- Error:
```json
{"ok": false, "error": "use --all or --id <watch-id>", "code": "INVALID_ARGUMENT"}
```

## Common Workflows
### Workflow: One-off brand check
```bash
dee-mentions check "dee.ink" --sources hn,reddit --json
```

### Workflow: Save watch and run recurring checks
```bash
dee-mentions watch add "dee.ink" --tag brand --json
dee-mentions run --all --json
```

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- JSON errors are printed to stdout.

## Storage
- Data: `~/.local/share/dee-mentions/mentions.db`
