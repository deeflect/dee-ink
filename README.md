<p align="center">
  <a href="https://dee.ink"><strong>dee.ink</strong></a>
</p>

<p align="center">
  <strong>33 Rust CLI tools built for AI agents</strong><br/>
  Each tool does one thing, outputs JSON, and plays nice with pipes.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/tools-33-blue?style=flat-square" alt="33 tools" />
  <img src="https://img.shields.io/badge/language-Rust-orange?style=flat-square" alt="Rust" />
  <img src="https://img.shields.io/badge/output-JSON-lightgrey?style=flat-square" alt="JSON" />
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="MIT" />
</p>

<p align="center">
  <a href="https://dee.ink">Website</a> · <a href="https://blog.deeflect.com/dee-ink/">Blog Post</a> · <a href="https://deeflect.com">Author</a> · <a href="https://x.com/deeflectcom">X</a>
</p>

---

## Tools

| Crate | What it does |
|---|---|
| [dee-amazon](crates/dee-amazon) | Search Amazon products |
| [dee-arxiv](crates/dee-arxiv) | Search academic papers on arXiv |
| [dee-feed](crates/dee-feed) | Read RSS and Atom feeds |
| [dee-ebay](crates/dee-ebay) | Search eBay listings |
| [dee-events](crates/dee-events) | Find local events by location and date |
| [dee-food](crates/dee-food) | Find restaurants and food spots |
| [dee-gas](crates/dee-gas) | Check gas prices by location |
| [dee-habit](crates/dee-habit) | Track habits and streaks locally |
| [dee-hn](crates/dee-hn) | Browse Hacker News |
| [dee-contacts](crates/dee-contacts) | Personal CRM with interactions and import/export |
| [dee-crosspost](crates/dee-crosspost) | Cross-post and schedule posts across major social platforms |
| [dee-invoice](crates/dee-invoice) | Generate invoice PDFs from JSON or YAML |
| [dee-mentions](crates/dee-mentions) | Track mentions across public sources |
| [dee-openrouter](crates/dee-openrouter) | Compare LLM models and pricing |
| [dee-package](crates/dee-package) | Look up package metadata and versions |
| [dee-parking](crates/dee-parking) | Find parking spots by location |
| [dee-ph](crates/dee-ph) | Browse Product Hunt launches |
| [dee-pricewatch](crates/dee-pricewatch) | Monitor webpage prices and detect drops |
| [dee-porkbun](crates/dee-porkbun) | Manage domains via Porkbun API |
| [dee-qr](crates/dee-qr) | Generate and decode QR codes |
| [dee-receipt](crates/dee-receipt) | Extract structured receipt data from images |
| [dee-reddit](crates/dee-reddit) | Search Reddit posts and subreddits |
| [dee-rates](crates/dee-rates) | Currency exchange rates |
| [dee-ssl](crates/dee-ssl) | Check SSL certs and TLS info |
| [dee-stash](crates/dee-stash) | Bookmark and read-later manager |
| [dee-todo](crates/dee-todo) | Local task list with JSON output |
| [dee-timer](crates/dee-timer) | Time tracking and pomodoro sessions |
| [dee-trends](crates/dee-trends) | Google Trends interest and related queries |
| [dee-transit](crates/dee-transit) | Route and transit directions |
| [dee-webpage](crates/dee-webpage) | Extract webpage metadata, text, and links |
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

---

### Made by

Made by [Dee](https://deeflect.com). 33 small CLIs because one big tool would have been smarter, but here we are.

Star if you ended up using even one of them. Open an issue if your shell breaks. PRs welcome — the framework is in `FRAMEWORK.md`.

Need similar built for you? [dee.agency](https://dee.agency?utm_source=deeink&utm_medium=readme).

[deeflect.com](https://deeflect.com) · [Wikidata](https://www.wikidata.org/entity/Q138828544) · [LinkedIn](https://www.linkedin.com/in/dkargaev/) · [X](https://x.com/deeflectcom)
