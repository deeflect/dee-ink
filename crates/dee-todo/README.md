# dee-todo

Local todo manager CLI with consistent, agent-friendly JSON output.

## Install

```sh
cargo install dee-todo
```

## Quick start

```sh
dee-todo add "Ship launch post" --priority 1 --project launch
dee-todo list --status open
dee-todo show 1
dee-todo done 1
dee-todo list --status all --json
```

## Commands

- `add <title> [--priority 0|1|2] [--project <name>] [--due-date YYYY-MM-DD] [--notes <text>] [--tags tag1,tag2]`
- `list [--status open|done|all] [--project <name>] [--priority 0|1|2]`
- `project <name> [--status open|done|all]`
- `search <query> [--status open|done|all]`
- `show <id>`
- `done <id>`
- `undone <id>`
- `edit <id> [--title ...] [--priority ...] [--project ...] [--due-date ...] [--notes ...] [--tags ...]`
- `delete <id>`

## Agent-friendly output

Use `--json` with every command.

Success list:

```json
{"ok":true,"count":2,"items":[...]}
```

Success action:

```json
{"ok":true,"message":"Todo added","id":3}
```

Error:

```json
{"ok":false,"error":"Todo not found","code":"NOT_FOUND"}
```

## Help

```sh
dee-todo --help
dee-todo <command> --help
```
