# dee-llmfit

Hardware-aware local LLM fit and planning CLI for agents.

`dee-llmfit` detects the current machine, reads a bundled model catalog, and returns structured recommendations for which local LLMs are likely to run well.

## Install

```bash
cargo install dee-llmfit
# or from this repo
cargo install --path crates/dee-llmfit
```

## Usage

```bash
dee-llmfit <command> [options]
```

Commands:

- `system` — show detected CPU/RAM/GPU/backend info
- `fit` — rank all compatible models by fit score
- `search <query>` — search the bundled model catalog
- `info <model>` — show model details and local fit analysis
- `recommend` — opinionated top recommendations
- `plan <model>` — estimate hardware needs for a model/context

Global flags:

- `-j, --json` — output JSON
- `-q, --quiet` — suppress decorative output
- `-v, --verbose` — debug output to stderr
- `-h, --help` — help
- `-V, --version` — version

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
  "error": "Model selector 'qwen' is ambiguous. Matches: ...",
  "code": "AMBIGUOUS"
}
```

## Notes

- No network access required.
- No API key or config file required.
- Estimates are heuristics, not benchmarks.
- JSON output omits null fields.
- Human/table output may use terminal formatting; `--json` and `--quiet` do not.
