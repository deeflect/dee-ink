# dee-ssl

TLS certificate inspection for domains.

## Install

```sh
cargo install --path crates/dee-ssl
```

## Quick start

```sh
dee-ssl check example.com
dee-ssl check example.com --chain
dee-ssl check example.com --warn-days 30
dee-ssl check example.com --port 8443
dee-ssl check example.com --timeout-secs 5
dee-ssl check example.com --json
```

## Commands

- `check`

## Agent-friendly output

Use `--json` for parsing cert metadata in scripts.

## Help

```sh
dee-ssl --help
dee-ssl check --help
```
