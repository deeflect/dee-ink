# dee-habit — Agent Guide

## Install
```bash
cargo install dee-habit
```

## Setup
- No API key required.
- Local SQLite DB is created automatically.

## Commands
```bash
dee-habit add <name> [--cadence daily|weekly] [--json] [--quiet] [--verbose]
dee-habit list [--json] [--quiet] [--verbose]
dee-habit done <habit|id> [--date today|yesterday|Nd|YYYY-MM-DD] [--json] [--quiet] [--verbose]
dee-habit streak <habit|id> [--json] [--quiet] [--verbose]
dee-habit delete <habit|id> [--json] [--quiet] [--verbose]
```

Examples:
```bash
dee-habit add "Read 20 minutes" --json
dee-habit done "Read 20 minutes" --json
dee-habit done 1 --date 2026-03-01 --json
dee-habit streak 1 --json
dee-habit list --json
```

## JSON Contract
- Success list:
```json
{"ok":true,"count":1,"items":[{"id":1,"name":"Read 20 minutes","cadence":"daily","created_at":"2026-03-01T00:00:00Z","current_streak":1,"best_streak":1,"last_done_on":"2026-03-01"}]}
```
- Success action:
```json
{"ok":true,"message":"Habit marked done","id":1}
```
- Error:
```json
{"ok":false,"error":"Habit not found","code":"NOT_FOUND"}
```

## Common Workflow
### Create, check in, and inspect streak
```bash
dee-habit add "Walk" --json
dee-habit done "Walk" --json
dee-habit streak "Walk" --json
```

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- Non-JSON errors are printed to stderr as `error: <message>`.
- JSON errors are printed to stdout.

## Storage
- Data: `~/.local/share/dee-habit/habit.db`
- Config: none
