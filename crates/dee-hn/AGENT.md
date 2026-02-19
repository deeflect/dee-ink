# dee-hn — Agent Guide

Rust CLI for reading Hacker News feeds, stories, comment trees, and user profiles.

## Install
```bash
cargo install --path .
# binary: dee-hn
```

## Setup
- No API key required.
- No local config file required.

## Commands
```bash
dee-hn top [--limit 30] [--json]
dee-hn new [--limit 30] [--json]
dee-hn best [--limit 30] [--json]
dee-hn ask [--limit 30] [--json]
dee-hn show [--limit 30] [--json]
dee-hn jobs [--limit 30] [--json]
dee-hn search <query> [--limit 20] [--json]
dee-hn item <id> [--json]
dee-hn comments <id> [--depth 2] [--json]
dee-hn user <id> [--json]
```

## Global flags
- `-j, --json` → JSON output contract (`ok`, `count` on list responses)
- `-q, --quiet` → suppress extra human-friendly headings
- `-v, --verbose` → reserved for debug output to stderr

## JSON contract
- Success list:
  ```json
  {"ok":true,"count":N,"items":[...]}
  ```
- Success single:
  ```json
  {"ok":true,"item":{...}}
  ```
- Error:
  ```json
  {"ok":false,"error":"...","code":"NOT_FOUND|NETWORK_ERROR|PARSE_ERROR|INTERNAL_ERROR"}
  ```
- No nulls emitted in JSON payloads.
- Times are ISO 8601 strings.

## Common workflows

### Workflow: fetch current top stories and open one item
```bash
dee-hn top --limit 10 --json
dee-hn item 47157224 --json
```

### Workflow: research by keyword, then inspect discussion
```bash
dee-hn search "tokio rust" --limit 5 --json
dee-hn comments 47157224 --depth 2 --json
```

## Error handling
- Exit code `0` = success
- Exit code `1` = error
- Non-JSON mode: error text on stderr
- JSON mode: error object on stdout, e.g.
  ```json
  {"ok":false,"error":"item 999999999999 not found","code":"NOT_FOUND"}
  ```

## Storage
- Data: none (no local state persisted)
- Config: none (no config file)

## Notes
- API endpoints verified against HN Firebase API:
  `topstories`, `newstories`, `beststories`, `askstories`, `showstories`, `jobstories`, `item/{id}`, `user/{id}`.
- Search uses Algolia HN API (`/api/v1/search?tags=story`).
- Comment output respects `--depth` from root story's child comments.
- `user` subcommand emits `{"ok":true,"item":{"id":"pg","karma":N,"about":"...","created":"..."}}`.
