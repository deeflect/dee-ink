# dee-wiki

Wikipedia search and summary CLI.

## Install

```sh
cargo install --path crates/dee-wiki
```

## Quick start

```sh
dee-wiki search "rust programming" --limit 5
dee-wiki search "tokio" --lang en --json
dee-wiki get "Rust (programming language)" --lang en --json
dee-wiki summary "Berlin" --lang de
dee-wiki summary "Taylor Swift" --json
```

## Commands

- `search`
- `get`
- `summary`

## Agent-friendly output

Use `--json` for predictable response fields.

## Help

```sh
dee-wiki --help
dee-wiki <command> --help
```
