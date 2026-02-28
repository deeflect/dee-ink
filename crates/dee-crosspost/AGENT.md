# dee-crosspost — Agent Guide

## Install
```bash
cargo install dee-crosspost
```

## Setup
- Configure tokens with `auth set-token` or env vars.
- `auth login` currently prints official OAuth docs and expects token setup after auth.

## Quick Start
```bash
dee-crosspost auth set-token --platform x --token "$X_TOKEN" --json
dee-crosspost post --to x --text "status update" --json

dee-crosspost schedule --at 2026-02-27T15:00:00Z --to x,linkedin --text "weekly update" --json
dee-crosspost run --once --json
```

## Commands
```bash
dee-crosspost post --to <platforms> --text <text> [--media <path>] [--title <title>] [--subreddit <name>] [--json] [--quiet] [--verbose]
dee-crosspost schedule --at <rfc3339> --to <platforms> --text <text> [--media <path>] [--title <title>] [--subreddit <name>] [--json] [--quiet] [--verbose]
dee-crosspost queue list [--status pending|running|done|failed|canceled] [--json] [--quiet] [--verbose]
dee-crosspost queue show <job-id> [--json] [--quiet] [--verbose]
dee-crosspost queue cancel <job-id> [--json] [--quiet] [--verbose]
dee-crosspost run --once|--daemon [--interval <seconds>] [--json] [--quiet] [--verbose]

dee-crosspost auth login --platform <x|linkedin|bluesky|threads|reddit> [--json] [--quiet] [--verbose]
dee-crosspost auth set-token --platform <x|linkedin|bluesky|threads|reddit> --token <token> [--json] [--quiet] [--verbose]
dee-crosspost auth status [--json] [--quiet] [--verbose]
dee-crosspost auth logout --platform <x|linkedin|bluesky|threads|reddit> [--json] [--quiet] [--verbose]
```

## JSON Contract
- Success list:
```json
{"ok": true, "count": 1, "items": [{"id": "...", "status": "pending"}]}
```
- Success action:
```json
{"ok": true, "message": "job scheduled", "id": "..."}
```
- Error:
```json
{"ok": false, "error": "Authentication missing for platform: x", "code": "AUTH_MISSING"}
```

## Platform Notes
- LinkedIn requires `DEE_CROSSPOST_LINKEDIN_ACTOR`.
- Bluesky requires `DEE_CROSSPOST_BLUESKY_REPO`.
- Threads requires `DEE_CROSSPOST_THREADS_USER_ID`.
- Reddit requires `--title` and `--subreddit`.

## Storage
- DB path: `~/.local/share/dee-crosspost/crosspost.db`
- Queue and auth tokens are stored locally.
