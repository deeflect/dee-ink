# dee-gas

US gas price lookup CLI powered by EIA data.

## Install

```sh
cargo install --path crates/dee-gas
```

## Quick start

```sh
dee-gas national --json
dee-gas prices --state CA --grade regular --json
dee-gas history --state TX --weeks 6 --json
dee-gas config set eia.api-key <KEY>
```

## Commands

- `national`
- `prices`
- `history`
- `config`

## Agent-friendly output

Use `--json` for structured price series.

## Help

```sh
dee-gas --help
dee-gas <command> --help
```
