# dee-openrouter — Agent Guide

CLI for searching and inspecting OpenRouter models.

## Install
```bash
cargo install --path .
# binary: dee-openrouter
```

## Setup
1. Optional: create an API key at https://openrouter.ai/keys
2. Optional: set key for authenticated requests
```bash
dee-openrouter config set openrouter.api-key sk-or-v1-...
```

## Commands
```bash
dee-openrouter list --json
dee-openrouter list --provider google --context-min 128000
dee-openrouter list --free --max-price 0.0 --json
dee-openrouter show google/gemini-3.1-pro-preview --json
dee-openrouter search "reasoning" --json
dee-openrouter config set openrouter.api-key sk-or-v1-...
dee-openrouter config show --json
dee-openrouter config path
```

## Output modes
- `--json` machine-readable output (`ok`, `count`, `items` / `item`)
- `--quiet` minimal stdout
- `--verbose` debug logs to stderr

## Config
- Path: `~/.config/dee-openrouter/config.toml`
- Key supported by `config set`: `openrouter.api-key`

## Common workflows

### Workflow: find low-cost high-context models
```bash
dee-openrouter list --context-min 128000 --max-price 0.5 --json
```

### Workflow: inspect a model from search results
```bash
dee-openrouter search "gemini" --limit 5 --json
dee-openrouter show google/gemini-3.1-pro-preview --json
```

### Workflow: set and verify API key config
```bash
dee-openrouter config set openrouter.api-key sk-or-v1-...
dee-openrouter config show --json
dee-openrouter config path
```

## Error handling
- Exit code `0` = success
- Exit code `1` = error
- JSON mode error shape:
```json
{"ok":false,"error":"...","code":"NOT_FOUND|INVALID_ARGUMENT|API_ERROR|NETWORK_ERROR|INTERNAL_ERROR"}
```

## Storage
- Data: none (no local database)
- Config: platform config dir + `dee-openrouter/config.toml`

## Notes
- `list` and `search` convert OpenRouter per-token prices into `*_per_1m` fields.
- Model listing endpoint works without an API key, but setting a key is supported.
