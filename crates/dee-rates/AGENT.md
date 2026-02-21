# dee-rates — Agent Guide

Use `dee-rates` for live FX rates and conversion.

## Install
```bash
cargo install --path .
```

## Setup
- No API key required.
- No config file required.

## Commands
```bash
dee-rates get <from> [to] [--json] [--quiet] [--verbose]
dee-rates convert <amount> <from> <to> [--json] [--quiet] [--verbose]
dee-rates list [--json] [--quiet] [--verbose]
```

## JSON contract
- Always includes `ok: true/false`
- List responses include `count`
- Errors include `code`
- No `null` fields
- Dates are ISO 8601 (`...Z`)

## Examples
```bash
dee-rates get USD EUR --json
dee-rates convert 250 GBP USD --json
dee-rates list --json
```

## Error handling
If `ok` is `false`, inspect:
- `code`: `NOT_FOUND`, `REQUEST_FAILED`, `BAD_RESPONSE`, `INVALID_ARGUMENT`
- `error`: human-readable message

## Output modes
- default: human-readable stdout
- `--json`: machine output on stdout
- `--quiet`: emit minimal plain output (not silence):
  - `get --quiet` → `{BASE} {DATE}` (e.g. `USD 2026-02-25T00:00:00Z`)
  - `convert --quiet` → `{result} {TO}` (e.g. `1.23 EUR`)
  - `list --quiet` → one `{code}` per line
- `--verbose`: debug logs to stderr

## Common workflows
```bash
dee-rates get USD EUR --json
dee-rates convert 100 USD JPY --json
dee-rates list --quiet
```

## Storage
- Data: none
- Config: none
