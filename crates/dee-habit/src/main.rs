use std::fs;
use std::path::PathBuf;

use chrono::{Duration, NaiveDate, SecondsFormat, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "dee-habit",
    version,
    about = "Local habit tracker with streaks and agent-friendly JSON output",
    long_about = "dee-habit - Track habits and streaks locally with consistent JSON output.",
    after_help = "EXAMPLES:\n  dee-habit add \"Drink water\" --cadence daily\n  dee-habit list --json\n  dee-habit done \"Drink water\" --json\n  dee-habit done 1 --date yesterday --json\n  dee-habit streak \"Drink water\" --json\n  dee-habit delete 1 --json"
)]
struct Cli {
    #[command(flatten)]
    global: GlobalFlags,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Args)]
struct GlobalFlags {
    /// Output as JSON
    #[arg(short = 'j', long, global = true)]
    json: bool,

    /// Suppress decorative output
    #[arg(short = 'q', long, global = true)]
    quiet: bool,

    /// Debug output to stderr
    #[arg(short = 'v', long, global = true)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Add a habit
    Add(AddArgs),
    /// List habits with streak summary
    List,
    /// Mark habit as done for one date
    Done(DoneArgs),
    /// Show streak details for one habit
    Streak(HabitArg),
    /// Delete a habit
    Delete(HabitArg),
}

#[derive(Debug, Args)]
struct AddArgs {
    name: String,

    #[arg(long, value_enum, default_value_t = CadenceArg::Daily)]
    cadence: CadenceArg,
}

#[derive(Debug, Clone, Args)]
struct HabitArg {
    habit: String,
}

#[derive(Debug, Args)]
struct DoneArgs {
    habit: String,

    /// Accepts: today, yesterday, Nd (e.g. 7d), or YYYY-MM-DD
    #[arg(long)]
    date: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CadenceArg {
    Daily,
    Weekly,
}

impl CadenceArg {
    fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
        }
    }
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    ok: bool,
    count: usize,
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct ItemResponse<T> {
    ok: bool,
    item: T,
}

#[derive(Debug, Serialize)]
struct ActionResponse {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, Clone)]
struct HabitRecord {
    id: i64,
    name: String,
    cadence: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct HabitListItem {
    id: i64,
    name: String,
    cadence: String,
    created_at: String,
    current_streak: i64,
    best_streak: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_done_on: Option<String>,
}

#[derive(Debug, Serialize)]
struct StreakItem {
    id: i64,
    name: String,
    cadence: String,
    current_streak: i64,
    best_streak: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_done_on: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Data directory not found")]
    DataDirMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Habit not found")]
    NotFound,
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Database operation failed")]
    Database,
    #[error("JSON serialization failed")]
    Serialize,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::DataDirMissing => "CONFIG_MISSING",
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict(_) => "CONFLICT",
            Self::Database => "DATABASE_ERROR",
            Self::Serialize => "SERIALIZE",
        }
    }
}

type AppResult<T> = Result<T, AppError>;

fn main() {
    let cli = parse_cli();

    if let Err(err) = run(&cli) {
        if cli.global.json {
            let payload = ErrorResponse {
                ok: false,
                error: err.to_string(),
                code: err.code().to_string(),
            };

            match serde_json::to_string(&payload) {
                Ok(rendered) => println!("{rendered}"),
                Err(_) => {
                    let escaped = escape_json(&err.to_string());
                    println!(
                        "{{\"ok\":false,\"error\":\"{escaped}\",\"code\":\"{}\"}}",
                        err.code()
                    );
                }
            }
        } else {
            eprintln!("error: {err}");
        }
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> AppResult<()> {
    let db_path = db_path()?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|_| AppError::Database)?;
    }

    if cli.global.verbose {
        eprintln!("[dee-habit] db_path={}", db_path.display());
    }

    let conn = Connection::open(db_path).map_err(|_| AppError::Database)?;
    initialize_db(&conn)?;

    match &cli.command {
        Commands::Add(args) => cmd_add(&conn, args, &cli.global),
        Commands::List => cmd_list(&conn, &cli.global),
        Commands::Done(args) => cmd_done(&conn, args, &cli.global),
        Commands::Streak(args) => cmd_streak(&conn, args, &cli.global),
        Commands::Delete(args) => cmd_delete(&conn, args, &cli.global),
    }
}

fn cmd_add(conn: &Connection, args: &AddArgs, global: &GlobalFlags) -> AppResult<()> {
    let name = args.name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidArgument(
            "habit name cannot be empty".to_string(),
        ));
    }

    let created_at = now_timestamp();

    let inserted = conn.execute(
        "INSERT INTO habits (name, cadence, created_at) VALUES (?1, ?2, ?3)",
        params![name, args.cadence.as_str(), created_at],
    );

    match inserted {
        Ok(_) => {
            let id = conn.last_insert_rowid();
            let message = "Habit added".to_string();

            if global.json {
                write_json(&ActionResponse {
                    ok: true,
                    message,
                    id: Some(id),
                })
            } else if global.quiet {
                println!("{id}");
                Ok(())
            } else {
                println!("Added habit #{id}: {name} ({})", args.cadence.as_str());
                Ok(())
            }
        }
        Err(err) => {
            if is_unique_violation(&err, "habits.name") {
                Err(AppError::Conflict("habit already exists".to_string()))
            } else {
                Err(AppError::Database)
            }
        }
    }
}

fn cmd_list(conn: &Connection, global: &GlobalFlags) -> AppResult<()> {
    let mut stmt = conn
        .prepare("SELECT id, name, cadence, created_at FROM habits ORDER BY id")
        .map_err(|_| AppError::Database)?;

    let records = stmt
        .query_map([], |row| {
            Ok(HabitRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                cadence: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|_| AppError::Database)?;

    let habits = records
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::Database)?;

    let mut items = Vec::with_capacity(habits.len());
    for habit in habits {
        let done_dates = fetch_done_dates(conn, habit.id)?;
        let (current_streak, best_streak, last_done_on) = compute_streaks(&done_dates)?;
        items.push(HabitListItem {
            id: habit.id,
            name: habit.name,
            cadence: habit.cadence,
            created_at: habit.created_at,
            current_streak,
            best_streak,
            last_done_on,
        });
    }

    if global.json {
        write_json(&ListResponse {
            ok: true,
            count: items.len(),
            items,
        })
    } else if global.quiet {
        println!("{}", items.len());
        Ok(())
    } else if items.is_empty() {
        println!("No habits yet.");
        Ok(())
    } else {
        for item in &items {
            let last = item.last_done_on.as_deref().unwrap_or("-");
            println!(
                "#{} {} [{}] streak {}/{} last {}",
                item.id, item.name, item.cadence, item.current_streak, item.best_streak, last
            );
        }
        Ok(())
    }
}

fn cmd_done(conn: &Connection, args: &DoneArgs, global: &GlobalFlags) -> AppResult<()> {
    let habit = resolve_habit(conn, &args.habit)?;
    let done_on = parse_done_on(args.date.as_deref())?;
    let created_at = now_timestamp();

    let changed = conn
        .execute(
            "INSERT OR IGNORE INTO checkins (habit_id, done_on, created_at) VALUES (?1, ?2, ?3)",
            params![habit.id, done_on.format("%Y-%m-%d").to_string(), created_at],
        )
        .map_err(|_| AppError::Database)?;

    let message = if changed == 0 {
        "Habit already marked done for date"
    } else {
        "Habit marked done"
    }
    .to_string();

    if global.json {
        write_json(&ActionResponse {
            ok: true,
            message,
            id: Some(habit.id),
        })
    } else if global.quiet {
        println!("{}", habit.id);
        Ok(())
    } else {
        println!(
            "{message}: {} on {}",
            habit.name,
            done_on.format("%Y-%m-%d")
        );
        Ok(())
    }
}

fn cmd_streak(conn: &Connection, args: &HabitArg, global: &GlobalFlags) -> AppResult<()> {
    let habit = resolve_habit(conn, &args.habit)?;
    let done_dates = fetch_done_dates(conn, habit.id)?;
    let (current_streak, best_streak, last_done_on) = compute_streaks(&done_dates)?;

    let item = StreakItem {
        id: habit.id,
        name: habit.name,
        cadence: habit.cadence,
        current_streak,
        best_streak,
        last_done_on,
    };

    if global.json {
        write_json(&ItemResponse { ok: true, item })
    } else if global.quiet {
        println!("{current_streak}");
        Ok(())
    } else {
        let last = item.last_done_on.as_deref().unwrap_or("-");
        println!(
            "{}: current streak {}, best streak {}, last {}",
            item.name, item.current_streak, item.best_streak, last
        );
        Ok(())
    }
}

fn cmd_delete(conn: &Connection, args: &HabitArg, global: &GlobalFlags) -> AppResult<()> {
    let habit = resolve_habit(conn, &args.habit)?;

    conn.execute(
        "DELETE FROM checkins WHERE habit_id = ?1",
        params![habit.id],
    )
    .map_err(|_| AppError::Database)?;

    conn.execute("DELETE FROM habits WHERE id = ?1", params![habit.id])
        .map_err(|_| AppError::Database)?;

    let message = "Habit deleted".to_string();

    if global.json {
        write_json(&ActionResponse {
            ok: true,
            message,
            id: Some(habit.id),
        })
    } else if global.quiet {
        println!("{}", habit.id);
        Ok(())
    } else {
        println!("Deleted habit #{}: {}", habit.id, habit.name);
        Ok(())
    }
}

fn resolve_habit(conn: &Connection, habit: &str) -> AppResult<HabitRecord> {
    let query = habit.trim();
    if query.is_empty() {
        return Err(AppError::InvalidArgument(
            "habit must be an id or name".to_string(),
        ));
    }

    if let Ok(id) = query.parse::<i64>() {
        if let Some(found) = query_habit_by_id(conn, id)? {
            return Ok(found);
        }
    }

    if let Some(found) = query_habit_exact_name(conn, query)? {
        return Ok(found);
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, name, cadence, created_at
             FROM habits
             WHERE lower(name) LIKE '%' || lower(?1) || '%'
             ORDER BY id
             LIMIT 2",
        )
        .map_err(|_| AppError::Database)?;

    let rows = stmt
        .query_map(params![query], |row| {
            Ok(HabitRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                cadence: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|_| AppError::Database)?;

    let matches = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::Database)?;

    match matches.len() {
        0 => Err(AppError::NotFound),
        1 => Ok(matches[0].clone()),
        _ => Err(AppError::InvalidArgument(
            "habit query matched multiple habits; use id or exact name".to_string(),
        )),
    }
}

fn query_habit_by_id(conn: &Connection, id: i64) -> AppResult<Option<HabitRecord>> {
    conn.query_row(
        "SELECT id, name, cadence, created_at FROM habits WHERE id = ?1",
        params![id],
        |row| {
            Ok(HabitRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                cadence: row.get(2)?,
                created_at: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|_| AppError::Database)
}

fn query_habit_exact_name(conn: &Connection, name: &str) -> AppResult<Option<HabitRecord>> {
    conn.query_row(
        "SELECT id, name, cadence, created_at FROM habits WHERE lower(name) = lower(?1)",
        params![name],
        |row| {
            Ok(HabitRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                cadence: row.get(2)?,
                created_at: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|_| AppError::Database)
}

fn fetch_done_dates(conn: &Connection, habit_id: i64) -> AppResult<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT done_on FROM checkins WHERE habit_id = ?1 ORDER BY done_on DESC")
        .map_err(|_| AppError::Database)?;

    let rows = stmt
        .query_map(params![habit_id], |row| row.get::<_, String>(0))
        .map_err(|_| AppError::Database)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::Database)
}

fn compute_streaks(done_dates: &[String]) -> AppResult<(i64, i64, Option<String>)> {
    if done_dates.is_empty() {
        return Ok((0, 0, None));
    }

    let parsed = done_dates
        .iter()
        .map(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| AppError::Database))
        .collect::<AppResult<Vec<_>>>()?;

    let mut current = 1_i64;
    for i in 1..parsed.len() {
        let previous = parsed[i - 1];
        let next = parsed[i];
        if previous - Duration::days(1) == next {
            current += 1;
        } else {
            break;
        }
    }

    let mut best = 1_i64;
    let mut run = 1_i64;
    for i in 1..parsed.len() {
        let previous = parsed[i - 1];
        let next = parsed[i];
        if previous - Duration::days(1) == next {
            run += 1;
        } else {
            run = 1;
        }
        if run > best {
            best = run;
        }
    }

    Ok((
        current,
        best,
        Some(parsed[0].format("%Y-%m-%d").to_string()),
    ))
}

fn parse_done_on(input: Option<&str>) -> AppResult<NaiveDate> {
    let today = Utc::now().date_naive();

    match input.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(today),
        Some(raw) => {
            let lower = raw.to_ascii_lowercase();
            if lower == "today" {
                return Ok(today);
            }
            if lower == "yesterday" {
                return Ok(today - Duration::days(1));
            }
            if let Some(days) = lower.strip_suffix('d') {
                let parsed_days: i64 = days.parse().map_err(|_| {
                    AppError::InvalidArgument(format!(
                        "invalid relative date '{raw}'. Use Nd like 7d"
                    ))
                })?;
                if parsed_days < 0 {
                    return Err(AppError::InvalidArgument(
                        "relative date must be non-negative".to_string(),
                    ));
                }
                return Ok(today - Duration::days(parsed_days));
            }

            NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| {
                AppError::InvalidArgument(format!(
                    "invalid date '{raw}'. Use today, yesterday, Nd, or YYYY-MM-DD"
                ))
            })
        }
    }
}

fn db_path() -> AppResult<PathBuf> {
    let base = dirs::data_dir().ok_or(AppError::DataDirMissing)?;
    Ok(base.join("dee-habit").join("habit.db"))
}

fn initialize_db(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS habits (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             name TEXT NOT NULL UNIQUE,
             cadence TEXT NOT NULL,
             created_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS checkins (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             habit_id INTEGER NOT NULL,
             done_on TEXT NOT NULL,
             created_at TEXT NOT NULL,
             UNIQUE(habit_id, done_on),
             FOREIGN KEY(habit_id) REFERENCES habits(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_checkins_habit_id_done_on
         ON checkins(habit_id, done_on DESC);",
    )
    .map_err(|_| AppError::Database)
}

fn write_json<T: Serialize>(value: &T) -> AppResult<()> {
    let rendered = serde_json::to_string(value).map_err(|_| AppError::Serialize)?;
    println!("{rendered}");
    Ok(())
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn is_unique_violation(err: &rusqlite::Error, target: &str) -> bool {
    match err {
        rusqlite::Error::SqliteFailure(_, Some(message)) => message.contains(target),
        _ => false,
    }
}

fn parse_cli() -> Cli {
    match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => handle_clap_parse_error(err),
    }
}

fn handle_clap_parse_error(err: clap::Error) -> ! {
    use clap::error::ErrorKind;

    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            let _ = err.print();
            std::process::exit(0);
        }
        _ => {
            let wants_json = std::env::args().any(|arg| arg == "--json" || arg == "-j");
            if wants_json {
                let payload = serde_json::json!({
                    "ok": false,
                    "error": err.to_string().trim(),
                    "code": "INVALID_ARGUMENT"
                });
                println!("{payload}");
            } else {
                let _ = err.print();
            }
            std::process::exit(2);
        }
    }
}
