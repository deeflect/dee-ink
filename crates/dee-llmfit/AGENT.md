# dee-llmfit — Agent Guide

## Install
```bash
cargo install dee-llmfit
# or from repo:
cargo install --path crates/dee-llmfit
```

## Setup
No API keys or network access required. `dee-llmfit` reads a bundled model catalog and detects local hardware.

## Quick Start
```bash
dee-llmfit recommend --use-case coding --json
```

## Commands
```bash
dee-llmfit system [--json] [--quiet] [--verbose]
dee-llmfit fit [--perfect] [--all] [--use-case general|coding|reasoning|chat|multimodal|embedding] [-n <limit>] [--json] [--quiet] [--verbose]
dee-llmfit search <query> [-n <limit>] [--json] [--quiet] [--verbose]
dee-llmfit info <model-selector> [--json] [--quiet] [--verbose]
dee-llmfit recommend [--use-case general|coding|reasoning|chat|multimodal|embedding] [-n <limit>] [--json] [--quiet] [--verbose]
dee-llmfit plan <model-selector> [--context <tokens>] [--quant <name>] [--target-tps <tokens-per-sec>] [--json] [--quiet] [--verbose]
```

## Examples
```bash
dee-llmfit system --json
dee-llmfit fit --perfect -n 10 --json
dee-llmfit fit --use-case coding --json
dee-llmfit search "qwen 14b" --json
dee-llmfit info "bigcode/gpt_bigcode-santacoder" --json
dee-llmfit recommend --use-case coding --json
dee-llmfit plan "bigcode/gpt_bigcode-santacoder" --context 8192 --json
```

## Output Format

List responses:
```json
{
  "ok": true,
  "count": 1,
  "items": [
    {
      "name": "bigcode/gpt_bigcode-santacoder",
      "provider": "BigCode",
      "parameter_count": "1.1B",
      "fit_level": "perfect",
      "estimated_tps": 58.7
    }
  ]
}
```

Single-item responses:
```json
{
  "ok": true,
  "item": {
    "name": "bigcode/gpt_bigcode-santacoder",
    "provider": "BigCode",
    "fit_level": "perfect",
    "run_mode": "gpu",
    "runtime": "MLX",
    "score": 60.2,
    "estimated_tps": 58.7,
    "memory_required_gb": 1.64,
    "memory_available_gb": 16.0,
    "backend_compatible": true,
    "notes": []
  }
}
```

Error responses:
```json
{
  "ok": false,
  "error": "Model selector 'qwen' is ambiguous. Matches: ...",
  "code": "AMBIGUOUS"
}
```

## Common Workflows

### Pick a local coding model
```bash
dee-llmfit system --json
dee-llmfit recommend --use-case coding --json
dee-llmfit info "bigcode/gpt_bigcode-santacoder" --json
```

### Check whether a specific model can run
```bash
dee-llmfit search "qwen coder" --json
dee-llmfit plan "Qwen/Qwen2.5-Coder-14B-Instruct" --context 8192 --json
```

### List only strong fits
```bash
dee-llmfit fit --perfect -n 20 --json
```

## Error Handling
- Exit code 0 = success.
- Exit code 1 = error.
- Data goes to stdout.
- Non-JSON errors go to stderr.
- `--json` errors go to stdout as `{"ok":false,"error":"...","code":"..."}`.
- Common codes: `INVALID_ARGUMENT`, `NOT_FOUND`, `AMBIGUOUS`, `INTERNAL`.

## Storage
- No persistent data directory.
- No config file.
- Bundled catalog: `crates/dee-llmfit/data/models.json`.
