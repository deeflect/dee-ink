# AGENT — dee-events

## Purpose
Search and inspect local events from Eventbrite.

## Typical flow
1. `dee-events config set eventbrite.token <TOKEN>`
2. `dee-events search "San Francisco" --query startup --json`
3. `dee-events show <event-id> --json`
