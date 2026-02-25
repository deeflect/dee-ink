# dee-openrouter

OpenRouter model discovery and inspection CLI.

## Install

```sh
cargo install --path crates/dee-openrouter
```

## Quick start

```sh
dee-openrouter list --provider google
dee-openrouter list --free --limit 10 --json
dee-openrouter search gemini --json
dee-openrouter show google/gemini-2.5-pro --json
dee-openrouter config set openrouter.api-key sk-xxx
dee-openrouter config show --json
```

## Commands

- `list`, `search`, `show`
- `config`

## Agent-friendly output

Use `--json` on query commands.

## Help

```sh
dee-openrouter --help
dee-openrouter <command> --help
```
