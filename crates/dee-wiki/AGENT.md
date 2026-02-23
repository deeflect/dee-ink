# dee-wiki — Agent Guide

Wikipedia search/summary CLI for agents.

## Install
```bash
cargo install dee-wiki
```

## Setup
- No API key required.
- Requires outbound HTTPS access to `*.wikipedia.org`.

## Quick Start
```bash
dee-wiki search "Rust programming language" --limit 3 --json
```

## Commands
```bash
dee-wiki search <query> [--limit 5] [--lang en] [--json] [--quiet] [--verbose]
dee-wiki get <title> [--lang en] [--json] [--quiet] [--verbose]
dee-wiki summary <title> [--lang en] [--json] [--quiet] [--verbose]
```

Examples:
```bash
dee-wiki search "Taylor Swift" --limit 5 --lang en --json
dee-wiki get "Rust (programming language)" --lang en --json
dee-wiki summary "Berlin" --lang de --json
dee-wiki summary "Rust (programming language)" --quiet
```

## JSON Contract
- Success list:
```json
{"ok": true, "count": 2, "items": [...]}
```
- Success item:
```json
{"ok": true, "item": {...}}
```
- Error:
```json
{"ok": false, "error": "No article found", "code": "NOT_FOUND"}
```

## Common Workflows
### Workflow: Find Candidate Pages Then Read One
```bash
dee-wiki search "tokio rust" --limit 5 --json
dee-wiki get "Tokio" --json
```

### Workflow: Produce A One-Line Summary For A Topic
```bash
dee-wiki summary "Rust (programming language)" --lang en --json
dee-wiki summary "Rust (programming language)" --quiet
```

## Behavior Notes
- `summary` is concise output (first sentence when possible).
- `get` returns the full extract from Wikipedia summary payload.
- `--verbose` writes debug messages to stderr.
- `--quiet` removes decorative human output.
- In `--json` mode, command output is machine-readable and has no nulls.

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- Non-JSON errors are written to stderr as `error: <message>`.
- JSON errors are written to stdout:
```json
{"ok": false, "error": "Wikipedia request failed", "code": "REQUEST_FAILED"}
```

## Storage
- No local DB/config required for current command set.
