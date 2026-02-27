# dee-invoice — Agent Guide

## Install
```bash
cargo install dee-invoice
```

## Setup
- No API key required.
- Input supports `.json`, `.yaml`, `.yml`.

## Quick Start
```bash
dee-invoice template --format yaml > invoice.yaml
dee-invoice calc invoice.yaml --json
dee-invoice generate invoice.yaml --format pdf --output invoice-001.pdf --json
```

## Commands
```bash
dee-invoice template [--format json|yaml] [--json] [--quiet] [--verbose]
dee-invoice calc <input> [--json] [--quiet] [--verbose]
dee-invoice generate <input> [--format pdf|json] [--output <path>] [--json] [--quiet] [--verbose]
```

## JSON Contract
- Success item:
```json
{"ok": true, "item": {"invoice_number": "INV-001", "subtotal": 2640.0, "tax_amount": 264.0, "total": 2904.0}}
```
- Success action:
```json
{"ok": true, "message": "Invoice PDF generated", "path": "invoice-001.pdf"}
```
- Error:
```json
{"ok": false, "error": "Invalid argument: currency must be a 3-letter code", "code": "INVALID_ARGUMENT"}
```

## Common Workflows
### Workflow: Validate invoice before export
```bash
dee-invoice calc invoice.yaml --json
dee-invoice generate invoice.yaml --format pdf --output invoice.pdf --json
```

### Workflow: Create JSON payload for API upload
```bash
dee-invoice generate invoice.yaml --format json --json
```

## Error Handling
- Exit code `0` = success.
- Exit code `1` = error.
- JSON errors are printed to stdout.

## Storage
- No local DB/config required.
