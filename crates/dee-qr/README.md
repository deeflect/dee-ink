# dee-qr

Generate and decode QR codes from the terminal.

## Install

```sh
cargo install --path crates/dee-qr
```

## Quick start

```sh
dee-qr generate "https://example.com" --out qr.png
dee-qr generate "hello" --format svg --out qr.svg --json
dee-qr generate "terminal demo" --format terminal
dee-qr decode qr.png
dee-qr decode qr.png --json
```

## Commands

- `generate`
- `decode`

## Agent-friendly output

Use `--json` for structured output.

## Help

```sh
dee-qr --help
dee-qr <command> --help
```
