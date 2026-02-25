# dee-hn

Hacker News CLI for stories, items, comments, and users.

## Install

```sh
cargo install --path crates/dee-hn
```

## Quick start

```sh
dee-hn top --limit 10
dee-hn new --json
dee-hn search "rust async" --limit 5 --json
dee-hn item 8863 --json
dee-hn comments 8863 --depth 2 --json
dee-hn user pg --json
```

## Commands

- Story lists: `top`, `new`, `best`, `ask`, `show`, `jobs`
- Lookup: `search`, `item`, `comments`, `user`

## Agent-friendly output

Use `--json` and `--quiet` for deterministic scripting.

## Help

```sh
dee-hn --help
dee-hn <command> --help
```
