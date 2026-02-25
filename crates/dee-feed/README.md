# dee-feed

RSS/Atom reader CLI for local feed tracking.

## Install

```sh
cargo install --path crates/dee-feed
```

## Quick start

```sh
dee-feed add https://example.com/feed.xml --name "Example"
dee-feed list --json
dee-feed fetch --limit 20 --json
dee-feed read 1 --json
dee-feed mark-read 1
dee-feed export --format opml
```

## Commands

- `add`, `list`, `remove`
- `fetch`, `read`, `mark-read`
- `export`, `import`, `config`

## Agent-friendly output

Use `--json` for machine-readable output.

```sh
dee-feed list --json
dee-feed fetch --limit 20 --json
```

## Help

```sh
dee-feed --help
dee-feed <command> --help
```
