# dee-crosspost

Cross-post to X, LinkedIn, Bluesky, Threads, and Reddit from one CLI. Supports immediate posting and local scheduling with a SQLite queue.

## Install

```sh
cargo install dee-crosspost
```

## Quick start

```sh
dee-crosspost auth set-token --platform x --token "$X_TOKEN"
dee-crosspost post --to x --text "Hello from dee-crosspost" --json

dee-crosspost schedule \
  --at 2026-02-27T15:00:00Z \
  --to bluesky,reddit \
  --text "Launch update" \
  --title "Launch update" \
  --subreddit startups \
  --json

dee-crosspost run --once --json
```

## Commands

- `post --to <platforms> --text <text> [--media <path>] [--title <title>] [--subreddit <name>]`
- `schedule --at <rfc3339> --to <platforms> --text <text> [--media <path>] [--title <title>] [--subreddit <name>]`
- `queue list [--status pending|running|done|failed|canceled]`
- `queue show <job-id>`
- `queue cancel <job-id>`
- `run --once | --daemon [--interval <seconds>]`
- `auth login --platform <name>`
- `auth set-token --platform <name> --token <token>`
- `auth status`
- `auth logout --platform <name>`

## Auth and config

Token env vars (override DB tokens):

- `DEE_CROSSPOST_X_TOKEN`
- `DEE_CROSSPOST_LINKEDIN_TOKEN`
- `DEE_CROSSPOST_BLUESKY_TOKEN`
- `DEE_CROSSPOST_THREADS_TOKEN`
- `DEE_CROSSPOST_REDDIT_TOKEN`

Provider-specific required vars:

- LinkedIn: `DEE_CROSSPOST_LINKEDIN_ACTOR`
- Bluesky: `DEE_CROSSPOST_BLUESKY_REPO`
- Threads: `DEE_CROSSPOST_THREADS_USER_ID`
- Reddit: `--title` and `--subreddit` flags on post/schedule

Optional API base overrides (useful for tests/proxies):

- `DEE_CROSSPOST_X_BASE`
- `DEE_CROSSPOST_LINKEDIN_BASE`
- `DEE_CROSSPOST_BLUESKY_BASE`
- `DEE_CROSSPOST_THREADS_BASE`
- `DEE_CROSSPOST_REDDIT_BASE`

## Storage

Local SQLite database:

- `~/.local/share/dee-crosspost/crosspost.db`

## Agent-friendly output

Use `--json` for machine-readable responses.

- Success list: `{"ok":true,"count":2,"items":[...]}`
- Success action: `{"ok":true,"message":"job scheduled","id":"..."}`
- Error: `{"ok":false,"error":"...","code":"..."}`
