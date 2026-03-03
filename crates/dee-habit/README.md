# dee-habit

Track habits and streaks locally from the terminal.

## Install

```bash
cargo install dee-habit
```

## Usage

```bash
dee-habit add <name> [--cadence daily|weekly] [--json] [--quiet] [--verbose]
dee-habit list [--json] [--quiet] [--verbose]
dee-habit done <habit|id> [--date today|yesterday|Nd|YYYY-MM-DD] [--json] [--quiet] [--verbose]
dee-habit streak <habit|id> [--json] [--quiet] [--verbose]
dee-habit delete <habit|id> [--json] [--quiet] [--verbose]
```

## Examples

```bash
dee-habit add "Drink water" --cadence daily --json
dee-habit done "Drink water" --json
dee-habit done 1 --date yesterday --json
dee-habit streak "Drink water" --json
dee-habit list --json
```

## JSON Contract

Success list:

```json
{"ok":true,"count":1,"items":[{"id":1,"name":"Drink water","cadence":"daily","created_at":"2026-03-01T00:00:00Z","current_streak":1,"best_streak":1,"last_done_on":"2026-03-01"}]}
```

Success action:

```json
{"ok":true,"message":"Habit added","id":1}
```

Error:

```json
{"ok":false,"error":"Habit not found","code":"NOT_FOUND"}
```

## Storage

- Data: `~/.local/share/dee-habit/habit.db`
- Config: none
