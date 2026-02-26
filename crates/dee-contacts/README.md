# dee-contacts

Local personal CRM CLI for contacts and interactions with consistent JSON output.

## Install

```sh
cargo install dee-contacts
```

## Quick start

```sh
dee-contacts add "Ada Lovelace" --email ada@example.com --company "Analytical Engines"
dee-contacts list --json
dee-contacts interaction add ada --kind note --summary "Intro call" --json
```

## Commands

- `add <name> [--email ..] [--phone ..] [--company ..] [--title ..] [--tags a,b] [--notes ..]`
- `list [--tag <tag>] [--company <name>] [--limit <n>]`
- `search <query> [--limit <n>]`
- `show <id-or-name>`
- `edit <id> [--name ..] [--email ..] [--phone ..] [--company ..] [--title ..] [--tags ..] [--notes ..]`
- `delete <id>`
- `import --format json|csv <path>`
- `export --format json|csv`
- `interaction add <id-or-name> --kind note|call|email|meeting --summary <text> [--occurred-at RFC3339]`
- `interaction list <id-or-name> [--limit <n>]`

## Agent-friendly output

Use `--json` on all commands.

- Success list: `{"ok":true,"count":N,"items":[...]}`
- Success item: `{"ok":true,"item":{...}}`
- Success action: `{"ok":true,"message":"...","id":1}`
- Error: `{"ok":false,"error":"...","code":"..."}`

## Storage

- Data: `~/.local/share/dee-contacts/contacts.db`

## Help

```sh
dee-contacts --help
dee-contacts <command> --help
```
