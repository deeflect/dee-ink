# dee-reddit

Search Reddit posts and browse subreddit listings.

## Install

```bash
cargo install dee-reddit
```

## Setup

```bash
dee-reddit config set reddit.client-id <ID>
dee-reddit config set reddit.client-secret <SECRET>
dee-reddit config set reddit.user-agent "dee-reddit/0.1 by u/yourname"
```

## Usage

```bash
dee-reddit search <query> [--limit <n>] [--sort relevance|hot|top|new|comments] [--json] [--quiet] [--verbose]
dee-reddit subreddit <name> [--limit <n>] [--sort relevance|hot|top|new|comments] [--json] [--quiet] [--verbose]
dee-reddit config set <key> <value> [--json]
dee-reddit config show [--json]
dee-reddit config path [--json]
```

## Examples

```bash
dee-reddit search "rust async" --limit 10 --json
dee-reddit subreddit rust --sort top --limit 10 --json
dee-reddit config show --json
```

## JSON Contract

Success list:

```json
{"ok":true,"count":1,"items":[{"id":"abc123","title":"Sample","subreddit":"rust","author":"alice","score":42,"comments":5,"nsfw":false,"created_utc":1700000000.0,"permalink":"/r/rust/...","url":"https://..."}]}
```

Error:

```json
{"ok":false,"error":"Missing Reddit credentials. Set reddit.client-id and reddit.client-secret","code":"AUTH_MISSING"}
```

## Storage

- Config: `~/.config/dee-reddit/config.toml`
- Data: none
