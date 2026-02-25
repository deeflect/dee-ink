# CLAUDE.md — dee.ink monorepo

## What this repo is

A collection of standalone Rust CLIs for AI agents.
Each tool lives in `crates/dee-<name>/` as its own crate.

## Read first

1. `FRAMEWORK.md` (CLI contract)
2. `AGENT-DOCS-GUIDE.md` (how to write tool `AGENT.md`)
3. `crates/dee-<name>/AGENT.md`

## Structure

- `crates/dee-<name>/` — crate code + `AGENT.md`
- `FRAMEWORK.md` — flags/output/error conventions
- `AGENT-DOCS-GUIDE.md` — concise agent docs format
- `BUILD_ORDER.md` / `MASTER-LIST.md` — roadmap context

## Naming

- crate: `dee-<toolname>`
- binary: `dee-<toolname>`
- directory: `crates/dee-<toolname>`

## Build/test

```bash
cd crates/dee-<toolname>
cargo test
cargo build --release
```

## Critical rules

- no shared runtime crate between tools
- no interactive prompts
- JSON errors must be machine-parseable (`ok`, `error`, `code`)
