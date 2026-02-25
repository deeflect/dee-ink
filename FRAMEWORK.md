# dee.ink CLI Framework

**Not a crate. Not a dependency. A set of rules every tool follows.**

Each tool is 100% standalone — `cargo install <tool>` and done. No shared runtime. But they all feel identical because they follow these patterns exactly.

---

## 1. CLI Structure (every tool)

```
<tool> <command> [args] [flags]
<tool> --help
<tool> --version
<tool> <command> --help
<tool> <command> --json       # Machine-readable output
<tool> <command> --quiet      # Suppress decorative output
<tool> <command> --verbose    # Extra debug info to stderr
```

### Global flags (every tool, every command)
| Flag | Short | Description |
|------|-------|-------------|
| `--json` | `-j` | Output as JSON to stdout |
| `--quiet` | `-q` | No decorative output (only data) |
| `--verbose` | `-v` | Debug info to stderr |
| `--help` | `-h` | Show help |
| `--version` | `-V` | Show version |

### Rules
- Data goes to **stdout**
- Errors and debug go to **stderr**
- Exit 0 = success, Exit 1 = error
- `--json` flag changes ALL output to JSON (including errors)
- No interactive prompts. Ever. Agents can't answer prompts.
- All arguments that could be optional MUST have sensible defaults
- Commands are **verbs**: `add`, `list`, `show`, `edit`, `delete`, `check`, `export`

---

## 2. JSON Output Contract

### Success (list-like)
```json
{
  "ok": true,
  "count": 3,
  "items": [...]
}
```

### Success (single item)
```json
{
  "ok": true,
  "item": {...}
}
```

### Success (action)
```json
{
  "ok": true,
  "message": "Contact added",
  "id": 42
}
```

### Error
```json
{
  "ok": false,
  "error": "Contact not found",
  "code": "NOT_FOUND"
}
```

**Rules:**
- Always include `"ok": true/false` — the dumbest LLM can check this
- List responses always have `"count"` — agents don't need to `.length` 
- IDs are always integers
- Dates are always ISO 8601: `"2026-02-24T12:00:00Z"`
- No nulls in JSON output — use empty string `""` or omit the field

---

## 3. Help Text Pattern

Every `--help` must follow this exact structure:

```
<tool-name> - <one-line description>

USAGE:
  <tool> <command> [options]

COMMANDS:
  add        Add a new <thing>
  list       List all <things>
  show       Show details of a <thing>
  edit       Edit a <thing>
  delete     Delete a <thing>
  export     Export data as CSV or JSON

OPTIONS:
  -j, --json       Output as JSON
  -q, --quiet      Suppress decorative output  
  -v, --verbose    Debug output to stderr
  -h, --help       Show this help
  -V, --version    Show version

EXAMPLES:
  <tool> add "Example"
  <tool> list --json
  <tool> show 1 --json
  <tool> export --format csv > data.csv
```

**Rules:**
- EXAMPLES section is mandatory — agents learn by example
- Show 3-5 realistic examples, not `foo bar`
- Every example must actually work

---

## 4. Storage Convention

```
Data:   ~/.local/share/<tool-name>/       (Linux/macOS)
Config: ~/.config/<tool-name>/config.toml (Linux/macOS)
```

Use the `dirs` crate to resolve platform-appropriate paths.

### Config format (TOML)
```toml
# ~/.config/<tool>/config.toml

[general]
default_format = "table"  # or "json"

[api]
# Tool-specific API keys
```

### Config commands (every tool that has config)
```
<tool> config show              # Print current config
<tool> config set <key> <value> # Set a config value
<tool> config path              # Print config file location
```

---

## 5. Agent-Friendliness Checklist

Before shipping any tool, verify:

- [ ] `--json` works on EVERY command (not just some)
- [ ] `--help` shows realistic EXAMPLES
- [ ] Zero interactive prompts — all input via flags/args
- [ ] All required args have clear error messages: `"error": "Missing required argument: name"`
- [ ] Dates accept multiple formats: `2026-02-24`, `today`, `yesterday`, `7d` (7 days ago)
- [ ] Fuzzy matching on names where applicable (typos happen)
- [ ] `config set` for API keys (not env vars — agents can't set env vars easily)
- [ ] Export to CSV and JSON (`export --format csv|json`)
- [ ] Import from common formats where applicable
- [ ] Exit codes are correct (0/1)
- [ ] No ANSI colors in `--json` or `--quiet` mode
- [ ] Errors include `"code"` field for programmatic handling

---

## 6. Naming Convention

- **Crate name** (crates.io): `dee-<toolname>` — e.g., `dee-habit`, `dee-contacts`
- **Binary name**: `dee-<toolname>` — e.g., `dee-habit`, `dee-contacts`
- **Directory name**: `crates/dee-<toolname>`
- **Cargo.toml** should set `autobins = false` and `[[bin]] name = "dee-<toolname>"`
- Command names are verbs: `add`, `list`, `show`, `edit`, `delete`, `check`
- Flag names use kebab-case: `--dry-run`, `--from-date`
- No abbreviations unless universal: `--json` yes, `--fmt` no

---

## 7. Versioning

- Semver: `MAJOR.MINOR.PATCH`
- `0.1.0` for first release
- Breaking CLI changes = major bump
- New commands/flags = minor bump
- Bug fixes = patch bump

---

## 8. Canonical Cargo Dependencies

Every tool starts from this dependency set. Add only what you need, use these exact versions.

```toml
[package]
autobins = false
name = "dee-<toolname>"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "dee-<toolname>"
path = "src/main.rs"

[dependencies]
# CLI
clap = { version = "4.5", features = ["derive", "color"] }

# Serialization
serde      = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow    = "1"
thiserror = "2"

# Date/time
chrono = { version = "0.4", features = ["serde"] }

# Platform paths (~/.config, ~/.local/share)
dirs = "5"

# Config files
toml = "1.0"

# Terminal colors (if needed)
owo-colors = "4"

# SQLite (tools with local storage)
rusqlite           = { version = "0.32.1", features = ["bundled"] }
rusqlite_migration = "2.4"

# HTTP — choose one:
# Async tools (multiple concurrent requests):
reqwest = { version = "0.13.1", features = ["json", "blocking"] }
tokio   = { version = "1", features = ["full"] }
# Sync tools (one request at a time, simpler):
# ureq = { version = "3.2", features = ["tls"] }
```

---

## 9. Database Migrations

All tools with SQLite use `rusqlite_migration` for schema versioning.

```rust
use rusqlite_migration::{Migrations, M};

static MIGRATIONS: Lazy<Migrations> = Lazy::new(|| {
    Migrations::new(vec![
        M::up(include_str!("../migrations/001_initial.sql")),
    ])
});

pub fn open_db(tool_name: &str) -> anyhow::Result<Connection> {
    let path = dirs::data_dir()
        .unwrap()
        .join(tool_name)
        .join(format!("{}.db", tool_name));
    std::fs::create_dir_all(path.parent().unwrap())?;
    let mut conn = Connection::open(&path)?;
    MIGRATIONS.to_latest(&mut conn)?;
    Ok(conn)
}
```

Migrations live in `migrations/001_initial.sql`, `002_add_field.sql`, etc.
Schema changes always go through a migration — never `CREATE TABLE IF NOT EXISTS` inline.

---

## 10. Repo Structure (per tool)

```
crates/dee-<tool-name>/
├── Cargo.toml
├── src/
│   ├── main.rs        # Entry point, clap setup
│   ├── cli.rs         # Clap derive structs
│   ├── commands/      # One file per command
│   │   ├── add.rs
│   │   ├── list.rs
│   │   └── ...
│   ├── db.rs          # SQLite setup + migrations
│   ├── models.rs      # Data structs
│   └── output.rs      # JSON/table output helpers
├── AGENT.md           # LLM agent docs (see AGENT-DOCS-GUIDE.md)
└── README.md          # Human docs
```

---

## 11. Cross-Tool Interoperability

Tools don't depend on each other but CAN work together via pipes:

```bash
# Stash a URL, then track its price
dee-stash add "https://amazon.com/..." --json | dee-pricewatch add --from-stdin

# Export contacts, pipe to crosspost for announcement
dee-contacts list --tag client --json | dee-crosspost post --template "Happy holidays {name}!"

# Receipt scan → habit log
dee-receipt scan photo.jpg --json | dee-habit done "Track expenses"
```

**Pattern:** `--from-stdin` flag reads JSON from stdin where it makes sense.

---

## 12. Error Messages

Good:
```
error: Contact "John" not found. Did you mean "John Doe" (id: 3)?
```

Bad:
```
Error: not found
```

Include:
- What went wrong
- What was expected
- Suggestion if possible (fuzzy match, similar items)
