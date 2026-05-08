# dee-package

Package metadata lookup CLI for agents.

`dee-package` queries public package registries and returns stable JSON for dependency research. The first checkpoint supports crates.io only, with room to add npm/PyPI/GitHub releases later as focused slices.

## Install

```bash
cargo install dee-package
# or from this repo
cargo install --path crates/dee-package
```

## Usage

```bash
dee-package <command> [options]
```

Commands:

- `search <ecosystem> <query>` — search packages
- `info <ecosystem> <name>` — show package metadata
- `latest <ecosystem> <name>` — show latest version metadata

Supported ecosystem aliases:

- `crates`, `crates.io`, `cargo`, `rust`

`search` returns registry search fields. Use `info` for full license, repository, documentation, homepage, keyword, category, and version-count metadata.

Global flags:

- `-j, --json` — output JSON
- `-q, --quiet` — suppress decorative output
- `-v, --verbose` — debug output to stderr
- `-h, --help` — help
- `-V, --version` — version

## Examples

```bash
dee-package search crates serde --limit 5 --json
dee-package info crates serde --json
dee-package latest crates serde --json
dee-package search crates "http client" --quiet
```

## JSON contract

Success responses include `ok: true`.

```json
{
  "ok": true,
  "count": 1,
  "items": []
}
```

```json
{
  "ok": true,
  "item": {}
}
```

Errors include `ok`, `error`, and `code`.

```json
{
  "ok": false,
  "error": "Unsupported ecosystem 'npm'. Supported ecosystems: crates, crates.io, cargo",
  "code": "UNSUPPORTED_ECOSYSTEM"
}
```

## Notes

- No API key or credentials required.
- No config file or local storage.
- Read-only public crates.io API.
- JSON output omits empty optional fields where possible.
- `--json` and `--quiet` output contain no ANSI decorations.
