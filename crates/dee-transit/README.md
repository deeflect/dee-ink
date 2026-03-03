# dee-transit

Find directions between two places.

## Install

```bash
cargo install dee-transit
```

## Setup

```bash
dee-transit config set google.api-key <KEY>
```

Optional custom endpoint:

```bash
dee-transit config set google.base-url https://maps.googleapis.com/maps/api/directions/json
```

## Usage

```bash
dee-transit route <origin> <destination> [--mode driving|walking|bicycling|transit] [--alternatives] [--json] [--quiet] [--verbose]
dee-transit config set <key> <value> [--json]
dee-transit config show [--json]
dee-transit config path [--json]
```

## Examples

```bash
dee-transit route "San Francisco, CA" "San Jose, CA" --mode driving --json
dee-transit route "Times Square" "JFK Airport" --mode transit --alternatives --json
dee-transit config show --json
```

## JSON Contract

Success list:

```json
{"ok":true,"count":1,"items":[{"summary":"US-101 S","distance_meters":77000,"duration_seconds":4200,"start_address":"San Francisco, CA","end_address":"San Jose, CA","steps":["Head south..."]}]}
```

Error:

```json
{"ok":false,"error":"Missing Google API key. Set google.api-key via config set","code":"AUTH_MISSING"}
```

## Storage

- Config: `~/.config/dee-transit/config.toml`
- Data: none
