# dee-stash — Agent Guide

## Install
```bash
cargo install dee-stash
```

## Setup
- No API key required.
- Uses local SQLite storage.

## Quick Start
```bash
dee-stash add https://example.com --title "Example" --tags research,tools --json
```

## Commands
```bash
dee-stash add <url> [--title <text>] [--notes <text>] [--tags t1,t2] [--json] [--quiet] [--verbose]
dee-stash list [--status unread|archived|all] [--tag <tag>] [--limit <n>] [--json] [--quiet] [--verbose]
dee-stash search <query> [--status unread|archived|all] [--limit <n>] [--json] [--quiet] [--verbose]
dee-stash show <id> [--json] [--quiet] [--verbose]
dee-stash edit <id> [--url <url>] [--title <text>] [--notes <text>] [--tags t1,t2] [--json] [--quiet] [--verbose]
dee-stash delete <id> [--json] [--quiet] [--verbose]
dee-stash archive <id> [--json] [--quiet] [--verbose]
dee-stash unarchive <id> [--json] [--quiet] [--verbose]
dee-stash import --format json|csv <path> [--json] [--quiet] [--verbose]
dee-stash export [--format json|csv] [--json] [--quiet] [--verbose]
```

Examples:
```bash
dee-stash add https://example.com --title "Docs" --tags reference --json
dee-stash list --status unread --json
dee-stash search rust --json
dee-stash archive 1 --json
dee-stash export --format json --json
dee-stash import --format csv bookmarks.csv --json
```

## JSON Contract
- Success list:
```json
{"ok": true, "count": 1, "items": [{"id": 1, "url": "https://example.com", "archived": false}]}
```
- Success item:
```json
{"ok": true, "item": {"id": 1, "url": "https://example.com", "tags": ["research"]}}
```
- Success action:
```json
{"ok": true, "message": "Bookmark added", "id": 1}
```
- Error:
```json
{"ok": false, "error": "Bookmark not found", "code": "NOT_FOUND"}
```

## Common Workflows
### Workflow: Save and archive links
```bash
dee-stash add https://example.com --title "Example" --json
dee-stash archive 1 --json
dee-stash list --status archived --json
```

### Workflow: Export and restore
```bash
dee-stash export --format json > stash.json
dee-stash import --format json stash.json --json
```

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- `--json` errors are emitted to stdout.
- Non-JSON errors are emitted to stderr.

## Storage
- Data: `~/.local/share/dee-stash/stash.db`
