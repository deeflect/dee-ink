# dee.ink

Open-source Rust CLI tools for AI agents.

Each tool is standalone and follows a shared CLI contract:
- `--json` for machine output
- `--quiet` for minimal output
- deterministic exit codes (`0` success, `1` failure)

## Current tools in this repository

| Tool crate | Binary | Purpose |
|---|---|---|
| `dee-feed` | `dee-feed` | RSS/Atom reader |
| `dee-hn` | `dee-hn` | Hacker News client |
| `dee-openrouter` | `dee-openrouter` | OpenRouter model/info tooling |
| `dee-qr` | `dee-qr` | QR generate/decode |
| `dee-rates` | `dee-rates` | Currency rates/conversion |
| `dee-ssl` | `dee-ssl` | TLS cert inspection |
| `dee-whois` | `dee-whois` | Domain WHOIS parser |
| `dee-wiki` | `dee-wiki` | Wikipedia lookup |
| `dee-porkbun` | `dee-porkbun` | Porkbun domains API CLI |

## Repo structure

```text
dee-ink/
├── FRAMEWORK.md         # CLI/output contract all tools follow
├── AGENT-DOCS-GUIDE.md  # how to write tool AGENT.md docs
├── BUILD_ORDER.md       # roadmap/order notes
├── MASTER-LIST.md       # longer-term tool inventory
├── crates/
│   └── dee-<tool>/      # standalone Cargo crate + AGENT docs
└── *.md                 # public root docs (README, FRAMEWORK, CLAUDE, etc.)
```

## Build and test

```bash
cd crates/dee-feed
cargo test
cargo build --release
```

Run this per tool directory.

## Notes

- Public source is focused on the CLI crates and root markdown docs.
- Local `website/`, `scripts/`, and `docs/` folders are ignored in git.

## License

MIT
