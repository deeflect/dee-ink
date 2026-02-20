# dee-qr — Agent Guide

QR code generate/decode CLI for agents.

## Install
```bash
cargo install --path .
# binary: dee-qr
```

## Setup
- No API key required.
- No config file required.

## Commands
```bash
dee-qr generate <text> --format png --out qr.png
dee-qr generate <text> --format svg --out qr.svg
dee-qr generate <text> --format terminal
dee-qr generate --stdin --format terminal
dee-qr decode qr.png
```

## Global flags (all commands)
```bash
--json    # machine-readable output (includes errors)
--quiet   # suppress decorative output
--verbose # debug info to stderr
```

## JSON contract
- Success: `{"ok": true, ...}`
- Error: `{"ok": false, "error": "...", "code": "..."}`
- No nulls.

## Common workflows

### Workflow: generate and decode a PNG QR
```bash
dee-qr generate "https://example.com" --out /tmp/qr.png --json
dee-qr decode /tmp/qr.png --json
```

### Workflow: generate from stdin for terminal display
```bash
echo "https://example.com" | dee-qr generate --stdin --format terminal --json
```

### Workflow: decode file path only (quiet mode)
```bash
dee-qr decode /tmp/qr.png --quiet
```

## Error handling
- Exit code `0` = success
- Exit code `1` = error
- JSON mode errors include codes:
  - `MISSING_ARGUMENT`
  - `UNSUPPORTED_FORMAT`
  - `NOT_FOUND`
  - `DECODE_FAILED`
  - `INTERNAL_ERROR`

## Examples
```bash
dee-qr generate "hello" --format png --json
# -> {"ok":false,"error":"Missing required argument: --out for format png","code":"MISSING_ARGUMENT"}
dee-qr decode /tmp/missing.png --json
# -> {"ok":false,"error":"Image file not found: /tmp/missing.png","code":"NOT_FOUND"}
```

## Storage
- Data: none
- Config: none

## Notes
- For `png` and `svg`, `--out` is required.
- For `terminal`, QR is rendered directly to stdout.
- `--stdin` reads text from stdin instead of a positional argument.
