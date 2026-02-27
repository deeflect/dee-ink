# dee-stash

Bookmark and read-later CLI with local SQLite storage and JSON output.

## Install

```sh
cargo install dee-stash
```

## Quick start

```sh
dee-stash add https://example.com --title "Example" --tags research,tools
dee-stash list --status unread --json
dee-stash archive 1 --json
```

## Commands

- `add <url> [--title <text>] [--notes <text>] [--tags a,b]`
- `list [--status unread|archived|all] [--tag <tag>] [--limit <n>]`
- `search <query> [--status unread|archived|all] [--limit <n>]`
- `show <id>`
- `edit <id> [--url <url>] [--title <text>] [--notes <text>] [--tags a,b]`
- `delete <id>`
- `archive <id>`
- `unarchive <id>`
- `import --format json|csv <path>`
- `export --format json|csv`

## Agent-friendly output

Use `--json` on all commands.

- Success list: `{"ok":true,"count":N,"items":[...]}`
- Success item: `{"ok":true,"item":{...}}`
- Success action: `{"ok":true,"message":"...","id":1}`
- Error: `{"ok":false,"error":"...","code":"..."}`

## Storage

- Data: `~/.local/share/dee-stash/stash.db`

## Help

```sh
dee-stash --help
dee-stash <command> --help
```
