use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "dee-timer",
    version,
    about = "Time tracking and pomodoro sessions with JSON output",
    long_about = "dee-timer - Track focused work sessions locally with agent-friendly output.",
    after_help = "EXAMPLES:\n  dee-timer start \"Write release notes\" --project launch\n  dee-timer start \"Deep work\" --pomodoro --json\n  dee-timer status --json\n  dee-timer stop --json\n  dee-timer list --status all --json\n  dee-timer report --period week --json\n  dee-timer delete 3"
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
    /// Start a new session
    Start(StartArgs),
    /// Stop active session
    Stop,
    /// Show active session status
    Status,
    /// Show one session by id
    Show(IdArgs),
    /// List sessions
    List(ListArgs),
    /// Report grouped totals
    Report(ReportArgs),
    /// Delete one session
    Delete(IdArgs),
}

#[derive(Debug, Args)]
struct StartArgs {
    /// Task label for this session
    task: Option<String>,

    /// Project name
    #[arg(long)]
    project: Option<String>,

    /// Comma-separated tags, e.g. deep,writing
    #[arg(long)]
    tags: Option<String>,

    /// Optional notes
    #[arg(long)]
    notes: Option<String>,

    /// Mark as pomodoro session
    #[arg(long)]
    pomodoro: bool,
}

#[derive(Debug, Args)]
struct IdArgs {
    id: i64,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SessionStatus {
    Running,
    Stopped,
    All,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long, value_enum, default_value_t = SessionStatus::All)]
    status: SessionStatus,

    #[arg(long)]
    project: Option<String>,

    #[arg(long, default_value_t = 20)]
    limit: usize,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReportPeriod {
    Today,
    Week,
    Month,
    All,
}

#[derive(Debug, Args)]
struct ReportArgs {
    #[arg(long, value_enum, default_value_t = ReportPeriod::Week)]
    period: ReportPeriod,

    #[arg(long)]
    project: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SessionItem {
    id: i64,
    task: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration_sec: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    pomodoro: bool,
}

#[derive(Debug, Serialize)]
struct ReportGroup {
    project: String,
    total_sec: i64,
    session_count: usize,
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

#[derive(Debug, Serialize)]
struct StatusItem {
    active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<SessionItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_sec: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Data directory not found")]
    DataDirMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("An active session already exists")]
    ActiveSessionExists,
    #[error("No active session found")]
    NoActiveSession,
    #[error("Session not found")]
    NotFound,
    #[error("Database operation failed")]
    Database,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::DataDirMissing => "CONFIG_MISSING",
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::ActiveSessionExists => "ACTIVE_SESSION_EXISTS",
            Self::NoActiveSession => "NO_ACTIVE_SESSION",
            Self::NotFound => "NOT_FOUND",
            Self::Database => "DATABASE_ERROR",
        }
    }
}

type AppResult<T> = Result<T, AppError>;

fn main() {
    let cli = parse_cli();

    if let Err(err) = run(&cli) {
        if cli.global.json {
            print_json(&ErrorResponse {
                ok: false,
                error: err.to_string(),
                code: err.code().to_string(),
            });
        } else {
            eprintln!("error: {err}");
        }
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> AppResult<()> {
    validate_command_args(&cli.command)?;

    let db_path = db_path()?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|_| AppError::Database)?;
    }

    if cli.global.verbose {
        eprintln!("[dee-timer] db_path={}", db_path.display());
    }

    let conn = Connection::open(db_path).map_err(|_| AppError::Database)?;
    initialize_db(&conn)?;

    match &cli.command {
        Commands::Start(args) => cmd_start(&conn, args, &cli.global),
        Commands::Stop => cmd_stop(&conn, &cli.global),
        Commands::Status => cmd_status(&conn, &cli.global),
        Commands::Show(args) => cmd_show(&conn, args.id, &cli.global),
        Commands::List(args) => cmd_list(&conn, args, &cli.global),
        Commands::Report(args) => cmd_report(&conn, args, &cli.global),
        Commands::Delete(args) => cmd_delete(&conn, args.id, &cli.global),
    }
}

fn validate_command_args(command: &Commands) -> AppResult<()> {
    match command {
        Commands::Show(args) | Commands::Delete(args) => {
            if args.id <= 0 {
                return Err(AppError::InvalidArgument(
                    "id must be greater than 0".to_string(),
                ));
            }
        }
        Commands::List(args) => {
            if args.limit == 0 {
                return Err(AppError::InvalidArgument(
                    "limit must be greater than 0".to_string(),
                ));
            }
        }
        Commands::Start(_) | Commands::Stop | Commands::Status | Commands::Report(_) => {}
    }

    Ok(())
}

fn db_path() -> AppResult<PathBuf> {
    let base = dirs::data_dir().ok_or(AppError::DataDirMissing)?;
    Ok(base.join("dee-timer").join("timer.db"))
}

fn initialize_db(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
          id           INTEGER PRIMARY KEY AUTOINCREMENT,
          task         TEXT    NOT NULL,
          project      TEXT,
          tags         TEXT,
          start_time   TEXT    NOT NULL,
          end_time     TEXT,
          duration_sec INTEGER,
          notes        TEXT,
          pomodoro     INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS projects (
          id         INTEGER PRIMARY KEY AUTOINCREMENT,
          name       TEXT UNIQUE NOT NULL,
          color      TEXT,
          created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_project    ON sessions(project);
        CREATE INDEX IF NOT EXISTS idx_sessions_start_time ON sessions(start_time);
        CREATE INDEX IF NOT EXISTS idx_sessions_end_time   ON sessions(end_time);
        "#,
    )
    .map_err(|_| AppError::Database)
}

fn cmd_start(conn: &Connection, args: &StartArgs, global: &GlobalFlags) -> AppResult<()> {
    if get_active_session(conn)?.is_some() {
        return Err(AppError::ActiveSessionExists);
    }

    let task = args
        .task
        .clone()
        .unwrap_or_else(|| "Focus Session".to_string());
    let tags_json =
        serde_json::to_string(&parse_tags(args.tags.as_deref())).map_err(|_| AppError::Database)?;
    let start_time = now_iso();
    let pomodoro = if args.pomodoro { 1 } else { 0 };

    conn.execute(
        "INSERT INTO sessions (task, project, tags, start_time, end_time, duration_sec, notes, pomodoro) VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6)",
        params![
            task,
            args.project.as_deref(),
            tags_json,
            start_time,
            args.notes.as_deref(),
            pomodoro,
        ],
    )
    .map_err(|_| AppError::Database)?;

    if let Some(project) = &args.project {
        conn.execute(
            "INSERT OR IGNORE INTO projects (name, color, created_at) VALUES (?1, '', ?2)",
            params![project, now_iso()],
        )
        .map_err(|_| AppError::Database)?;
    }

    let id = conn.last_insert_rowid();
    print_action_result("Session started", id, global)
}

fn cmd_stop(conn: &Connection, global: &GlobalFlags) -> AppResult<()> {
    let active = get_active_session(conn)?.ok_or(AppError::NoActiveSession)?;
    let end_time = now_iso();
    let duration = compute_duration_sec(&active.start_time, &end_time)?;

    conn.execute(
        "UPDATE sessions SET end_time = ?1, duration_sec = ?2 WHERE id = ?3",
        params![end_time, duration, active.id],
    )
    .map_err(|_| AppError::Database)?;

    let message = if active.pomodoro {
        "Pomodoro session stopped. Take a 5-minute break."
    } else {
        "Session stopped"
    };

    print_action_result(message, active.id, global)
}

fn cmd_status(conn: &Connection, global: &GlobalFlags) -> AppResult<()> {
    let active = get_active_session(conn)?;

    let status = if let Some(session) = active {
        let elapsed = compute_duration_sec(&session.start_time, &now_iso())?;
        StatusItem {
            active: true,
            session: Some(session),
            elapsed_sec: Some(elapsed),
            message: None,
        }
    } else {
        StatusItem {
            active: false,
            session: None,
            elapsed_sec: None,
            message: Some("No active session".to_string()),
        }
    };

    if global.json {
        print_json(&ItemResponse {
            ok: true,
            item: status,
        });
        return Ok(());
    }

    if global.quiet {
        println!("{}", if status.active { "1" } else { "0" });
        return Ok(());
    }

    if let Some(session) = status.session {
        println!("active session #{}: {}", session.id, session.task);
        if let Some(elapsed) = status.elapsed_sec {
            println!("elapsed_sec: {elapsed}");
        }
    } else {
        println!("no active session");
    }

    Ok(())
}

fn cmd_show(conn: &Connection, id: i64, global: &GlobalFlags) -> AppResult<()> {
    let item = get_session_by_id(conn, id)?.ok_or(AppError::NotFound)?;

    if global.json {
        print_json(&ItemResponse { ok: true, item });
        return Ok(());
    }

    if global.quiet {
        println!("{id}");
        return Ok(());
    }

    let status = if item.end_time.is_some() {
        "stopped"
    } else {
        "running"
    };
    println!("#{} [{}] {}", item.id, status, item.task);
    Ok(())
}

fn cmd_list(conn: &Connection, args: &ListArgs, global: &GlobalFlags) -> AppResult<()> {
    let items = fetch_sessions(
        conn,
        SessionFilter {
            status: args.status,
            project: args.project.clone(),
            since: None,
            limit: Some(args.limit),
        },
    )?;

    print_session_list(items, global)
}

fn cmd_report(conn: &Connection, args: &ReportArgs, global: &GlobalFlags) -> AppResult<()> {
    let since = period_start(args.period);

    let items = fetch_sessions(
        conn,
        SessionFilter {
            status: SessionStatus::Stopped,
            project: args.project.clone(),
            since,
            limit: None,
        },
    )?;

    let mut by_project: BTreeMap<String, (i64, usize)> = BTreeMap::new();
    for item in &items {
        let key = item
            .project
            .clone()
            .unwrap_or_else(|| "unassigned".to_string());
        let duration = item.duration_sec.unwrap_or(0);
        let entry = by_project.entry(key).or_insert((0, 0));
        entry.0 += duration;
        entry.1 += 1;
    }

    let groups: Vec<ReportGroup> = by_project
        .into_iter()
        .map(|(project, (total_sec, session_count))| ReportGroup {
            project,
            total_sec,
            session_count,
        })
        .collect();

    if global.json {
        print_json(&ListResponse {
            ok: true,
            count: groups.len(),
            items: groups,
        });
        return Ok(());
    }

    if global.quiet {
        let total: i64 = groups.iter().map(|g| g.total_sec).sum();
        println!("{total}");
        return Ok(());
    }

    if groups.is_empty() {
        println!("no sessions in selected period");
        return Ok(());
    }

    for group in groups {
        println!(
            "{}: {} sec across {} session(s)",
            group.project, group.total_sec, group.session_count
        );
    }

    Ok(())
}

fn cmd_delete(conn: &Connection, id: i64, global: &GlobalFlags) -> AppResult<()> {
    let deleted = conn
        .execute("DELETE FROM sessions WHERE id = ?1", params![id])
        .map_err(|_| AppError::Database)?;

    if deleted == 0 {
        return Err(AppError::NotFound);
    }

    print_action_result("Session deleted", id, global)
}

fn print_action_result(message: &str, id: i64, global: &GlobalFlags) -> AppResult<()> {
    if global.json {
        print_json(&ActionResponse {
            ok: true,
            message: message.to_string(),
            id: Some(id),
        });
        return Ok(());
    }

    if global.quiet {
        println!("{id}");
    } else {
        println!("{message}: #{id}");
    }

    Ok(())
}

fn print_session_list(items: Vec<SessionItem>, global: &GlobalFlags) -> AppResult<()> {
    if global.json {
        print_json(&ListResponse {
            ok: true,
            count: items.len(),
            items,
        });
        return Ok(());
    }

    if global.quiet {
        println!("{}", items.len());
        return Ok(());
    }

    if items.is_empty() {
        println!("no sessions found");
        return Ok(());
    }

    for item in items {
        let status = if item.end_time.is_some() {
            "stopped"
        } else {
            "running"
        };
        let duration = item.duration_sec.unwrap_or(0);
        println!(
            "#{: <4} [{}] {: <6} {}",
            item.id, status, duration, item.task
        );
    }

    Ok(())
}

#[derive(Debug)]
struct SessionFilter {
    status: SessionStatus,
    project: Option<String>,
    since: Option<DateTime<Utc>>,
    limit: Option<usize>,
}

fn fetch_sessions(conn: &Connection, filter: SessionFilter) -> AppResult<Vec<SessionItem>> {
    let mut sql = String::from(
        "SELECT id, task, project, tags, start_time, end_time, duration_sec, notes, pomodoro FROM sessions WHERE 1=1",
    );
    let mut bind_values: Vec<rusqlite::types::Value> = Vec::new();

    match filter.status {
        SessionStatus::Running => sql.push_str(" AND end_time IS NULL"),
        SessionStatus::Stopped => sql.push_str(" AND end_time IS NOT NULL"),
        SessionStatus::All => {}
    }

    if let Some(project) = filter.project {
        sql.push_str(" AND project = ?");
        bind_values.push(rusqlite::types::Value::Text(project));
    }

    if let Some(since) = filter.since {
        sql.push_str(" AND start_time >= ?");
        bind_values.push(rusqlite::types::Value::Text(
            since.to_rfc3339_opts(SecondsFormat::Secs, true),
        ));
    }

    sql.push_str(" ORDER BY start_time DESC, id DESC");

    if let Some(limit) = filter.limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }

    let mut stmt = conn.prepare(&sql).map_err(|_| AppError::Database)?;
    let rows = stmt
        .query_map(params_from_iter(bind_values.iter()), parse_session_row)
        .map_err(|_| AppError::Database)?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|_| AppError::Database)?);
    }

    Ok(items)
}

fn get_active_session(conn: &Connection) -> AppResult<Option<SessionItem>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task, project, tags, start_time, end_time, duration_sec, notes, pomodoro FROM sessions WHERE end_time IS NULL ORDER BY start_time DESC LIMIT 1",
        )
        .map_err(|_| AppError::Database)?;

    stmt.query_row([], parse_session_row)
        .optional()
        .map_err(|_| AppError::Database)
}

fn get_session_by_id(conn: &Connection, id: i64) -> AppResult<Option<SessionItem>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, task, project, tags, start_time, end_time, duration_sec, notes, pomodoro FROM sessions WHERE id = ?1",
        )
        .map_err(|_| AppError::Database)?;

    stmt.query_row(params![id], parse_session_row)
        .optional()
        .map_err(|_| AppError::Database)
}

fn parse_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionItem> {
    let tags_json: String = row.get(3)?;
    let tags = serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default();

    Ok(SessionItem {
        id: row.get(0)?,
        task: row.get(1)?,
        project: row.get(2)?,
        tags,
        start_time: row.get(4)?,
        end_time: row.get(5)?,
        duration_sec: row.get(6)?,
        notes: row.get(7)?,
        pomodoro: row.get::<_, i64>(8)? == 1,
    })
}

fn period_start(period: ReportPeriod) -> Option<DateTime<Utc>> {
    let now = Utc::now();
    match period {
        ReportPeriod::Today => Some(now - Duration::days(1)),
        ReportPeriod::Week => Some(now - Duration::days(7)),
        ReportPeriod::Month => Some(now - Duration::days(30)),
        ReportPeriod::All => None,
    }
}

fn compute_duration_sec(start: &str, end: &str) -> AppResult<i64> {
    let start_dt = DateTime::parse_from_rfc3339(start)
        .map_err(|_| AppError::Database)?
        .with_timezone(&Utc);
    let end_dt = DateTime::parse_from_rfc3339(end)
        .map_err(|_| AppError::Database)?
        .with_timezone(&Utc);

    Ok((end_dt - start_dt).num_seconds().max(0))
}

fn parse_tags(input: Option<&str>) -> Vec<String> {
    input
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => println!("{text}"),
        Err(_) => {
            println!(r#"{{"ok":false,"error":"JSON serialization failed","code":"SERIALIZE"}}"#);
            std::process::exit(1);
        }
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
