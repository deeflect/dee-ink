<p align="center">
  <a href="https://dee.ink"><strong>dee.ink</strong></a>
</p>

<p align="center">
  Small Rust CLI tools that work well with LLMs.<br/>
  Each tool does one thing, outputs JSON, and plays nice with pipes.
</p>

<p align="center">
  <a href="https://dee.ink">Website</a> · <a href="https://deeflect.com">Author</a> · <a href="https://x.com/deeflectcom">X</a>
</p>

---

## Tools

| Crate | What it does |
|---|---|
| [dee-feed](crates/dee-feed) | Read RSS and Atom feeds |
| [dee-hn](crates/dee-hn) | Browse Hacker News |
| [dee-openrouter](crates/dee-openrouter) | Compare LLM models and pricing |
| [dee-porkbun](crates/dee-porkbun) | Manage domains via Porkbun API |
| [dee-qr](crates/dee-qr) | Generate and decode QR codes |
| [dee-rates](crates/dee-rates) | Currency exchange rates |
| [dee-ssl](crates/dee-ssl) | Check SSL certs and TLS info |
| [dee-whois](crates/dee-whois) | Domain WHOIS lookups |
| [dee-wiki](crates/dee-wiki) | Wikipedia article lookup |

## Shared contract

Every tool follows the same rules:

- `--json` for structured output
- `--quiet` for minimal output
- Exit `0` on success, `1` on failure
- Errors go to stderr, data goes to stdout

Full spec in [FRAMEWORK.md](FRAMEWORK.md).

## Install

```bash
cargo install dee-rates
```

Or build from source:

```bash
git clone https://github.com/deeflect/dee-ink
cd dee-ink
cargo build --release -p dee-rates
```

## Build

Cargo workspace. Build and test everything:

```bash
cargo build --workspace
cargo test --workspace
```

Single tool:

```bash
cargo build -p dee-feed
cargo test -p dee-feed
```

## Repo layout

```
crates/dee-*/    Tool crates
website/         dee.ink website (Next.js)
FRAMEWORK.md     CLI contract spec
CLAUDE.md        Agent instructions
```

## License

[MIT](LICENSE)
