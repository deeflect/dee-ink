# dee-webpage

Webpage metadata, text, and link extraction CLI for agents.

`dee-webpage` fetches static HTML over HTTP/HTTPS and returns stable JSON for research and coding-agent workflows. The MVP intentionally does not render JavaScript.

## Install

```bash
cargo install dee-webpage
# or from this repo
cargo install --path crates/dee-webpage
```

## Usage

```bash
dee-webpage <command> [options]
```

Commands:

- `metadata <url>` — fetch status, title, meta description, canonical URL, headings, and counts
- `text <url>` — extract readable text from `article`, `main`, `[role=main]`, or `body`
- `markdown <url>` — extract simple Markdown from headings, paragraphs, lists, code blocks, and quotes
- `links <url>` — extract resolved HTTP links

Global flags:

- `-j, --json` — output JSON
- `-q, --quiet` — suppress decorative output
- `-v, --verbose` — debug output to stderr
- `-h, --help` — help
- `-V, --version` — version

## Examples

```bash
dee-webpage metadata https://example.com --json
dee-webpage text https://example.com --max-chars 4000 --json
dee-webpage text https://example.com --selector main --json
dee-webpage markdown https://example.com --json
dee-webpage links https://example.com --limit 50 --json
dee-webpage links https://example.com --external --quiet
```

## JSON contract

Success responses include `ok: true`.

```json
{
  "ok": true,
  "item": {}
}
```

```json
{
  "ok": true,
  "count": 1,
  "items": []
}
```

Errors include `ok`, `error`, and `code`.

```json
{
  "ok": false,
  "error": "Invalid argument: url must be a valid absolute URL",
  "code": "INVALID_ARGUMENT"
}
```

## Notes

- No API key or credentials required.
- No config file or local storage.
- Read-only HTTP/HTTPS GET requests only.
- Static HTML only; no JavaScript rendering.
- UTF-8 HTML responses are supported in the MVP.
- JSON output omits empty optional fields where possible.
- `--json` and `--quiet` output contain no ANSI decorations.
