# dee-contacts — Agent Guide

## Install
```bash
cargo install dee-contacts
```

## Setup
- No API key required.
- Uses local SQLite storage.

## Quick Start
```bash
dee-contacts add "Ada Lovelace" --email ada@example.com --tags founder,math --json
```

## Commands
```bash
dee-contacts add <name> [--email <email>] [--phone <phone>] [--company <name>] [--title <title>] [--tags t1,t2] [--notes <text>] [--json] [--quiet] [--verbose]
dee-contacts list [--tag <tag>] [--company <name>] [--limit <n>] [--json] [--quiet] [--verbose]
dee-contacts search <query> [--limit <n>] [--json] [--quiet] [--verbose]
dee-contacts show <id-or-name> [--json] [--quiet] [--verbose]
dee-contacts edit <id> [--name <name>] [--email <email>] [--phone <phone>] [--company <name>] [--title <title>] [--tags t1,t2] [--notes <text>] [--json] [--quiet] [--verbose]
dee-contacts delete <id> [--json] [--quiet] [--verbose]
dee-contacts import --format json|csv <path> [--json] [--quiet] [--verbose]
dee-contacts export [--format json|csv] [--json] [--quiet] [--verbose]
dee-contacts interaction add <id-or-name> --kind note|call|email|meeting --summary <text> [--occurred-at RFC3339] [--json] [--quiet] [--verbose]
dee-contacts interaction list <id-or-name> [--limit <n>] [--json] [--quiet] [--verbose]
```

Examples:
```bash
dee-contacts add "Linus Torvalds" --company Linux --json
dee-contacts search linus --json
dee-contacts show linus --json
dee-contacts interaction add linus --kind note --summary "Met at event" --json
dee-contacts export --format json --json
dee-contacts import --format csv contacts.csv --json
```

## JSON Contract
- Success list:
```json
{"ok": true, "count": 1, "items": [{"id": 1, "name": "Ada Lovelace"}]}
```
- Success item:
```json
{"ok": true, "item": {"contact": {"id": 1, "name": "Ada Lovelace"}, "interaction_count": 1, "interactions": [...]}}
```
- Success action:
```json
{"ok": true, "message": "Contact added", "id": 1}
```
- Error:
```json
{"ok": false, "error": "Contact name is ambiguous. Use id instead", "code": "AMBIGUOUS"}
```

## Common Workflows
### Workflow: Create contact and log interaction
```bash
dee-contacts add "Ada Lovelace" --email ada@example.com --json
dee-contacts interaction add ada --kind note --summary "Initial outreach" --json
dee-contacts show ada --json
```

### Workflow: Export and import contacts
```bash
dee-contacts export --format json > contacts.json
dee-contacts import --format json contacts.json --json
```

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- `--json` errors are emitted to stdout.
- Non-JSON errors are emitted to stderr.

## Storage
- Data: `~/.local/share/dee-contacts/contacts.db`
