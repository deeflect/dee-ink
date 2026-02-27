# dee-invoice

Generate invoices from JSON/YAML and export as PDF or JSON.

## Install

```sh
cargo install dee-invoice
```

## Quick start

```sh
dee-invoice template --format yaml > invoice.yaml
dee-invoice calc invoice.yaml --json
dee-invoice generate invoice.yaml --format pdf --output invoice-001.pdf
```

## Commands

- `template [--format json|yaml]`
- `calc <input>`
- `generate <input> [--format pdf|json] [--output <path>]`

## Agent-friendly output

Use `--json` on commands.

- Success item: `{"ok":true,"item":{...}}`
- Success action: `{"ok":true,"message":"Invoice PDF generated","path":"invoice.pdf"}`
- Error: `{"ok":false,"error":"...","code":"..."}`

## Help

```sh
dee-invoice --help
dee-invoice <command> --help
```
