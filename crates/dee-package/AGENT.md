# dee-package — Agent Guide

## Install
```bash
cargo install dee-package
# or from repo:
cargo install --path crates/dee-package
```

## Setup
No API keys, credentials, config, or local storage required for the MVP.

Supported ecosystems in this checkpoint:
- `crates`, `crates.io`, `cargo`, `rust` → crates.io

Note: `search` returns registry search fields. Use `info` for full license, repository, documentation, homepage, keyword, category, and version-count metadata.

## Quick Start
```bash
dee-package info crates serde --json
```

## Commands
```bash
dee-package search <ecosystem> <query> [--limit <1-100>] [--json] [--quiet] [--verbose]
dee-package info <ecosystem> <name> [--json] [--quiet] [--verbose]
dee-package latest <ecosystem> <name> [--json] [--quiet] [--verbose]
```

## Examples
```bash
dee-package search crates serde --limit 5 --json
dee-package info crates serde --json
dee-package latest crates serde --json
dee-package search crates "http client" --quiet
```

## Output Format

Search responses:
```json
{
  "ok": true,
  "count": 1,
  "items": [
    {
      "ecosystem": "crates.io",
      "name": "serde",
      "version": "1.0.228",
      "description": "A generic serialization/deserialization framework",
      "license": "MIT OR Apache-2.0",
      "downloads": 342000000,
      "recent_downloads": 12000000,
      "updated_at": "2026-01-01T00:00:00Z",
      "repository": "https://github.com/serde-rs/serde",
      "source_url": "https://crates.io/crates/serde"
    }
  ]
}
```

Info responses:
```json
{
  "ok": true,
  "item": {
    "ecosystem": "crates.io",
    "name": "serde",
    "latest_version": "1.0.228",
    "stable_version": "1.0.228",
    "description": "A generic serialization/deserialization framework",
    "license": "MIT OR Apache-2.0",
    "downloads": 342000000,
    "recent_downloads": 12000000,
    "created_at": "2014-12-06T20:49:48.000000Z",
    "updated_at": "2026-01-01T00:00:00Z",
    "repository": "https://github.com/serde-rs/serde",
    "keywords": ["serde", "serialization"],
    "categories": ["encoding"],
    "versions_count": 280,
    "yanked_versions": 0,
    "source_url": "https://crates.io/crates/serde"
  }
}
```

Latest responses:
```json
{
  "ok": true,
  "item": {
    "ecosystem": "crates.io",
    "name": "serde",
    "version": "1.0.228",
    "yanked": false,
    "license": "MIT OR Apache-2.0",
    "downloads": 1000000,
    "published_at": "2026-01-01T00:00:00Z",
    "updated_at": "2026-01-01T00:00:00Z",
    "crate_size": 82000,
    "source_url": "https://crates.io/crates/serde"
  }
}
```

Error responses:
```json
{
  "ok": false,
  "error": "Package 'missing-package' not found in crates.io",
  "code": "NOT_FOUND"
}
```

## Common Workflows

### Verify a Rust dependency before editing Cargo.toml
```bash
dee-package latest crates serde --json
dee-package info crates serde --json
```

### Find likely crate names
```bash
dee-package search crates "html parser" --limit 10 --json
```

### Machine-check unsupported ecosystem handling
```bash
dee-package info npm react --json
```

## Error Handling
- Exit code 0 = success.
- Exit code 1 = error.
- Data goes to stdout.
- Non-JSON errors go to stderr.
- `--json` errors go to stdout as `{"ok":false,"error":"...","code":"..."}`.
- Common codes: `UNSUPPORTED_ECOSYSTEM`, `INVALID_ARGUMENT`, `NOT_FOUND`, `REQUEST_FAILED`, `HTTP_STATUS`, `PARSE_FAILED`, `INTERNAL`.

## Storage
- No persistent data directory.
- No config file.
- Public read-only crates.io API only.
