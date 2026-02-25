# dee-arxiv

Academic paper search CLI for arXiv.

## Install

```sh
cargo install --path crates/dee-arxiv
```

## Quick start

```sh
dee-arxiv search "graph neural networks" --limit 10 --json
dee-arxiv get 2312.12345 --json
dee-arxiv author "Yann LeCun" --limit 5 --json
```

## Commands

- `search`
- `get`
- `author`

## Agent-friendly output

Use `--json` for structured paper metadata.

## Help

```sh
dee-arxiv --help
dee-arxiv <command> --help
```
