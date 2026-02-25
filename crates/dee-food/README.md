# dee-food

Restaurant search CLI using Yelp Fusion.

## Install

```sh
cargo install --path crates/dee-food
```

## Quick start

```sh
dee-food search "New York, NY" --term sushi --limit 10 --json
dee-food show yelp-san-francisco --json
dee-food reviews yelp-san-francisco --json
dee-food config set yelp.api-key <KEY>
```

## Commands

- `search`
- `show`
- `reviews`
- `config`

## Agent-friendly output

Use `--json` for predictable place/review fields.

## Help

```sh
dee-food --help
dee-food <command> --help
```
