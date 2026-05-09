# dee-webpage — Agent Guide

## Install
```bash
cargo install dee-webpage
# or from repo:
cargo install --path crates/dee-webpage
```

## Setup
No API keys, credentials, config, or local storage required.

MVP scope:
- HTTP/HTTPS GET only.
- Static HTML only; no JavaScript rendering.
- UTF-8 HTML responses.
- Read-only network access.

## Quick Start
```bash
dee-webpage metadata https://example.com --json
```

## Commands
```bash
dee-webpage metadata <url> [--timeout-secs <seconds>] [--max-bytes <bytes>] [--json] [--quiet] [--verbose]
dee-webpage text <url> [--selector <css>] [--max-chars <chars>] [--timeout-secs <seconds>] [--max-bytes <bytes>] [--json] [--quiet] [--verbose]
dee-webpage markdown <url> [--selector <css>] [--max-chars <chars>] [--timeout-secs <seconds>] [--max-bytes <bytes>] [--json] [--quiet] [--verbose]
dee-webpage links <url> [--limit <n>] [--internal|--external] [--timeout-secs <seconds>] [--max-bytes <bytes>] [--json] [--quiet] [--verbose]
```

## Examples
```bash
dee-webpage metadata https://example.com --json
dee-webpage text https://example.com --max-chars 4000 --json
dee-webpage text https://example.com --selector main --json
dee-webpage markdown https://example.com --json
dee-webpage links https://example.com --limit 50 --json
dee-webpage links https://example.com --external --quiet
```

## Output Format

Metadata response:
```json
{
  "ok": true,
  "item": {
    "url": "https://example.com",
    "final_url": "https://example.com/",
    "status": 200,
    "content_type": "text/html; charset=utf-8",
    "bytes": 1256,
    "content_sha256": "...",
    "title": "Example Domain",
    "description": "Example description",
    "canonical_url": "https://example.com/",
    "lang": "en",
    "headings_count": 1,
    "headings": [{ "level": 1, "text": "Example Domain" }],
    "links_count": 1,
    "images_count": 0
  }
}
```

Text response:
```json
{
  "ok": true,
  "item": {
    "url": "https://example.com",
    "final_url": "https://example.com/",
    "title": "Example Domain",
    "selector": "main",
    "text": "Example Domain This domain is for use in illustrative examples...",
    "chars": 84,
    "truncated": false,
    "content_sha256": "..."
  }
}
```

Markdown response:
```json
{
  "ok": true,
  "item": {
    "url": "https://example.com",
    "final_url": "https://example.com/",
    "title": "Example Domain",
    "selector": "main",
    "markdown": "# Example Domain\n\nThis domain is for use in illustrative examples...",
    "chars": 86,
    "truncated": false,
    "content_sha256": "..."
  }
}
```

Links response:
```json
{
  "ok": true,
  "count": 1,
  "items": [
    {
      "source_url": "https://example.com/",
      "url": "https://www.iana.org/domains/example",
      "text": "More information...",
      "rel": "",
      "internal": false
    }
  ]
}
```

Error response:
```json
{
  "ok": false,
  "error": "Invalid argument: url must be a valid absolute URL",
  "code": "INVALID_ARGUMENT"
}
```

## Common Workflows

### Summarize a page with metadata first
```bash
dee-webpage metadata https://example.com --json
dee-webpage markdown https://example.com --max-chars 12000 --json
```

### Collect external citations from a page
```bash
dee-webpage links https://example.com/article --external --limit 100 --json
```

### Extract a known content region
```bash
dee-webpage markdown https://example.com/docs --selector article --json
```

## Error Handling
- Exit code 0 = success.
- Exit code 1 = error.
- Data goes to stdout.
- Non-JSON errors go to stderr.
- `--json` errors go to stdout as `{"ok":false,"error":"...","code":"..."}`.
- Common codes: `INVALID_ARGUMENT`, `REQUEST_FAILED`, `HTTP_STATUS`, `RESPONSE_TOO_LARGE`, `PARSE_FAILED`, `INTERNAL`.

## Storage
- No persistent data directory.
- No config file.
