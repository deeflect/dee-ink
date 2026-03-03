# dee-transit — Agent Guide

## Install
```bash
cargo install dee-transit
```

## Setup
```bash
dee-transit config set google.api-key <KEY>
```

## Commands
```bash
dee-transit route <origin> <destination> [--mode driving|walking|bicycling|transit] [--alternatives] [--json] [--quiet] [--verbose]
dee-transit config set google.api-key <value> [--json]
dee-transit config set google.base-url <value> [--json]
dee-transit config show [--json]
dee-transit config path [--json]
```

## JSON Contract
- Success:
```json
{"ok":true,"count":1,"items":[{"summary":"US-101 S","distance_meters":77000}]}
```
- Error:
```json
{"ok":false,"error":"Missing Google API key. Set google.api-key via config set","code":"AUTH_MISSING"}
```

## Storage
- Config: `~/.config/dee-transit/config.toml`
