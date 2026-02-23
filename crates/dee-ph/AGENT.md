# AGENT — dee-ph

## Purpose
Query Product Hunt posts from CLI with stable machine output.

## Typical flow
1. `dee-ph config set ph.api-key <TOKEN>`
2. `dee-ph top --limit 10 --json`
3. `dee-ph search ai --json`
4. `dee-ph show chatgpt --json`

## Notes
- Use `--json` for machine parsing.
- Use `--quiet` for minimal non-JSON output.
