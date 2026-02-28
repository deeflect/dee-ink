# dee-mentions

Track brand mentions across public sources from the terminal.

## Install

```sh
cargo install dee-mentions
```

## Quick start

```sh
dee-mentions check "dee.ink" --sources hn,reddit --limit 5 --json
dee-mentions watch add "dee.ink" --tag brand --json
dee-mentions run --all --json
```

## Commands

- `check <query> [--sources hn,reddit] [--limit <n>]`
- `run --all|--id <watch-id> [--sources ...] [--limit <n>]`
- `watch add <query> [--tag <tag>] [--sources hn,reddit]`
- `watch list`
- `watch remove <id>`

## Agent-friendly output

Use `--json` on all commands.

- Success list: `{"ok":true,"count":N,"items":[...]}`
- Success action: `{"ok":true,"message":"Watch added","id":1}`
- Error: `{"ok":false,"error":"...","code":"..."}`

## Storage

- Data: `~/.local/share/dee-mentions/mentions.db`

## Help

```sh
dee-mentions --help
dee-mentions <command> --help
```
