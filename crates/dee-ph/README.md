# dee-ph

Product Hunt CLI for listing and searching posts.

## Install

```sh
cargo install --path crates/dee-ph
```

## Quick start

```sh
dee-ph top --limit 10
dee-ph search ai --json
dee-ph show chatgpt --json
dee-ph config set ph.api-key <TOKEN>
dee-ph config show --json
dee-ph config path
```

## Commands

- `top`
- `search`
- `show`
- `config`

## Agent-friendly output

Use `--json` for integration into agents/pipelines.

## Help

```sh
dee-ph --help
dee-ph <command> --help
```
