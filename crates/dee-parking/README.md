# dee-parking

Find parking spots near a location.

## Install

```bash
cargo install dee-parking
```

## Setup

```bash
dee-parking config set google.api-key <KEY>
```

Optional custom endpoint:

```bash
dee-parking config set google.base-url https://maps.googleapis.com/maps/api/place/textsearch/json
```

## Usage

```bash
dee-parking search <location> [--query <text>] [--limit <n>] [--json] [--quiet] [--verbose]
dee-parking config set <key> <value> [--json]
dee-parking config show [--json]
dee-parking config path [--json]
```

## Examples

```bash
dee-parking search "Downtown Austin" --limit 10 --json
dee-parking search "Mission District SF" --query "covered parking near Mission District SF" --json
dee-parking config show --json
```

## JSON Contract

Success list:

```json
{"ok":true,"count":1,"items":[{"name":"City Center Garage","address":"123 Main St","rating":4.3,"rating_count":120,"business_status":"OPERATIONAL","open_now":true}]}
```

Error:

```json
{"ok":false,"error":"Missing Google API key. Set google.api-key via config set","code":"AUTH_MISSING"}
```

## Storage

- Config: `~/.config/dee-parking/config.toml`
- Data: none
