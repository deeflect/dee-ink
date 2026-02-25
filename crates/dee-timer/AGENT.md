# dee-timer — Agent Guide

## Install
```bash
cargo install dee-timer
```

## Setup
- No API key required.
- Local SQLite DB is created automatically.

## Quick Start
```bash
dee-timer start "Research competitors" --project growth --json
```

## Commands
```bash
dee-timer start [task] [--project <name>] [--tags tag1,tag2] [--notes <text>] [--pomodoro] [--json] [--quiet] [--verbose]
dee-timer stop [--json] [--quiet] [--verbose]
dee-timer status [--json] [--quiet] [--verbose]
dee-timer show <id> [--json] [--quiet] [--verbose]
dee-timer list [--status running|stopped|all] [--project <name>] [--limit <n>] [--json] [--quiet] [--verbose]
dee-timer report [--period today|week|month|all] [--project <name>] [--json] [--quiet] [--verbose]
dee-timer delete <id> [--json] [--quiet] [--verbose]
```

Examples:
```bash
dee-timer start "Deep work" --pomodoro --tags focus,writing --json
dee-timer status --json
dee-timer stop --json
dee-timer list --status all --json
dee-timer report --period week --project growth --json
dee-timer show 2 --json
```

## JSON Contract
- Success list:
```json
{"ok": true, "count": 2, "items": [{"project": "growth", "total_sec": 3600, "session_count": 3}]}
```
- Success item:
```json
{"ok": true, "item": {"active": true, "elapsed_sec": 520, "session": {"id": 4, "task": "Deep work", "start_time": "2026-02-26T10:00:00Z", "pomodoro": true}}}
```
- Success action:
```json
{"ok": true, "message": "Session stopped", "id": 4}
```
- Error:
```json
{"ok": false, "error": "An active session already exists", "code": "ACTIVE_SESSION_EXISTS"}
```

## Common Workflows
### Workflow: Run one pomodoro session
```bash
dee-timer start "Write spec" --pomodoro --project docs --json
dee-timer status --json
dee-timer stop --json
```

### Workflow: Weekly project time report
```bash
dee-timer report --period week --json
dee-timer report --period week --project docs --json
```

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- Non-JSON errors are printed to stderr as `error: <message>`.
- JSON errors are printed to stdout.

## Storage
- Data: `~/.local/share/dee-timer/timer.db`
- Config: none
