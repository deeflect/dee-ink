# dee-events

Local events search CLI via Eventbrite.

## Install

```sh
cargo install --path crates/dee-events
```

## Quick start

```sh
dee-events search "San Francisco" --query tech --limit 10 --json
dee-events show 1234567890 --json
dee-events config set eventbrite.token <TOKEN>
```

## Commands

- `search`
- `show`
- `config`

## Agent-friendly output

Use `--json` for machine-readable event payloads.

## Help

```sh
dee-events --help
dee-events <command> --help
```
