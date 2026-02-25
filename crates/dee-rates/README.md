# dee-rates

Currency rates and conversion CLI.

## Install

```sh
cargo install --path crates/dee-rates
```

## Quick start

```sh
dee-rates get USD
dee-rates get USD EUR --json
dee-rates convert 100 USD EUR
dee-rates convert 100 USD EUR --json
dee-rates list --json
```

## Commands

- `get`
- `convert`
- `list`

## Agent-friendly output

Use `--json` for automated workflows.

## Help

```sh
dee-rates --help
dee-rates <command> --help
```
