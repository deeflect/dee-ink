# dee-feed — Agent Guide

Standalone RSS/Atom feed CLI with stable JSON output.

## Build

```bash
cd /Volumes/retroboy/Coding/Personal/dee-ink/tools/feed
cargo build --release
```

Binaries:
- `./target/release/dee-feed` (primary)
- `./target/release/dee-feed` (compat alias)

## Commands

```bash
dee-feed add <url> [--name "My Feed"] [--json]
dee-feed list [--json]
dee-feed remove <name-or-id> [--json]
dee-feed fetch [<name-or-id>] [--limit 20] [--unread] [--json]
dee-feed read <item-id> [--json]
dee-feed mark-read <name-or-id> --all [--json]
dee-feed export [--format opml|json] [--json]
dee-feed import <file.opml> [--json]
dee-feed config show [--json]
```

## JSON contract

- Success payloads always include `"ok": true`
- Error payloads always include `"ok": false`, `"error"`, `"code"`
- List responses include `"count"` and `"items"`
- Datetimes are ISO 8601 strings

Common shapes:

```json
{"ok":true,"count":2,"items":[{"id":1}]}
{"ok":true,"item":{"id":1}}
{"ok":true,"message":"Feed added","id":1}
{"ok":false,"error":"Feed not found: x","code":"RUNTIME_ERROR"}
```

## Storage

- Feeds config: `~/.config/dee-feed/feeds.toml`
- Optional config: `~/.config/dee-feed/config.toml`
- SQLite DB: `~/.local/share/dee-feed/feed.db`

On macOS this maps under `~/Library/Application Support/dee-feed/`.

## Operational notes

- `fetch [<name-or-id>]` deduplicates items by `(feed_id, ext_id)`.
- `read <item-id>` marks the item as read and returns `"item.read": true` in that same response.
- `import` expects OPML outlines containing `xmlUrl`.
- `--quiet` emits minimal machine-readable output:
  - `add` -> new feed id
  - `list` -> feed ids (one per line)
  - `remove` -> removed feed id
  - `mark-read --all` -> updated item count

## Real-world smoke test

```bash
dee-feed add https://xkcd.com/rss.xml --name xkcd --json
dee-feed fetch xkcd --limit 3 --json
dee-feed read <item-id> --json
```
