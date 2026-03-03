# dee-reddit — Agent Guide

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

## Commands
```bash
dee-reddit search <query> [--limit <n>] [--sort relevance|hot|top|new|comments] [--json] [--quiet] [--verbose]
dee-reddit subreddit <name> [--limit <n>] [--sort relevance|hot|top|new|comments] [--json] [--quiet] [--verbose]
dee-reddit config set reddit.client-id <value> [--json]
dee-reddit config set reddit.client-secret <value> [--json]
dee-reddit config set reddit.user-agent <value> [--json]
dee-reddit config show [--json]
dee-reddit config path [--json]
```

## JSON Contract
- Success:
```json
{"ok":true,"count":1,"items":[{"id":"abc123","title":"Sample post","subreddit":"rust"}]}
```
- Error:
```json
{"ok":false,"error":"Missing Reddit credentials. Set reddit.client-id and reddit.client-secret","code":"AUTH_MISSING"}
```

## Storage
- Config: `~/.config/dee-reddit/config.toml`
