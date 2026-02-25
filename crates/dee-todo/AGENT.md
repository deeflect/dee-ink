# dee-todo — Agent Guide

## Install
```bash
cargo install dee-todo
```

## Setup
- No API key required.
- Local SQLite DB is created automatically.

## Quick Start
```bash
dee-todo add "Write changelog" --priority 1 --project release --json
```

## Commands
```bash
dee-todo add <title> [--priority 0|1|2] [--project <name>] [--due-date YYYY-MM-DD] [--notes <text>] [--tags tag1,tag2] [--json] [--quiet] [--verbose]
dee-todo list [--status open|done|all] [--project <name>] [--priority 0|1|2] [--json] [--quiet] [--verbose]
dee-todo project <name> [--status open|done|all] [--json] [--quiet] [--verbose]
dee-todo search <query> [--status open|done|all] [--json] [--quiet] [--verbose]
dee-todo show <id> [--json] [--quiet] [--verbose]
dee-todo done <id> [--json] [--quiet] [--verbose]
dee-todo undone <id> [--json] [--quiet] [--verbose]
dee-todo edit <id> [--title <text>] [--priority 0|1|2] [--project <name>] [--due-date YYYY-MM-DD] [--notes <text>] [--tags tag1,tag2] [--json] [--quiet] [--verbose]
dee-todo delete <id> [--json] [--quiet] [--verbose]
```

Examples:
```bash
dee-todo add "Ship v1" --priority 2 --project launch --tags release,urgent --json
dee-todo list --status open --json
dee-todo show 4 --json
dee-todo search launch --json
dee-todo done 4 --json
dee-todo edit 4 --title "Ship v1.0" --due-date 2026-03-10 --json
dee-todo delete 4 --json
```

## JSON Contract
- Success list:
```json
{"ok": true, "count": 2, "items": [{"id": 1, "title": "Ship v1", "done": false, "priority": 2, "created_at": "2026-02-26T00:00:00Z"}]}
```
- Success action:
```json
{"ok": true, "message": "Todo updated", "id": 1}
```
- Error:
```json
{"ok": false, "error": "Todo not found", "code": "NOT_FOUND"}
```

## Common Workflows
### Workflow: Create, complete, and review tasks
```bash
dee-todo add "Write release post" --project launch --priority 1 --json
dee-todo done 1 --json
dee-todo list --status all --project launch --json
```

### Workflow: Find tasks by keyword and update
```bash
dee-todo search release --json
dee-todo edit 1 --notes "Needs screenshot" --json
```

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- Non-JSON errors are printed to stderr as `error: <message>`.
- JSON errors are printed to stdout.

## Storage
- Data: `~/.local/share/dee-todo/todo.db`
- Config: none
