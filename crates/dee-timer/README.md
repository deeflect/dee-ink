# dee-timer

Track focused work sessions and pomodoros locally with consistent JSON output.

## Install

```sh
cargo install dee-timer
```

## Quick start

```sh
dee-timer start "Write launch copy" --project launch
dee-timer status
dee-timer stop
dee-timer report --period week --json
```

## Commands

- `start [task] [--project <name>] [--tags tag1,tag2] [--notes <text>] [--pomodoro]`
- `stop`
- `status`
- `show <id>`
- `list [--status running|stopped|all] [--project <name>] [--limit <n>]`
- `report [--period today|week|month|all] [--project <name>]`
- `delete <id>`

## Agent-friendly output

Use `--json` with every command.

Success list:

```json
{"ok":true,"count":2,"items":[...]}
```

Success item:

```json
{"ok":true,"item":{"active":true,...}}
```

Success action:

```json
{"ok":true,"message":"Session started","id":4}
```

Error:

```json
{"ok":false,"error":"No active session found","code":"NO_ACTIVE_SESSION"}
```

## Help

```sh
dee-timer --help
dee-timer <command> --help
```
