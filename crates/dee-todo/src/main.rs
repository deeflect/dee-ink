use std::fs;
use std::path::PathBuf;

use chrono::{SecondsFormat, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "dee-todo",
    version,
    about = "Local todo manager with agent-friendly JSON output",
    long_about = "dee-todo - Manage a local task list from the terminal with consistent JSON output.",
    after_help = "EXAMPLES:\n  dee-todo add \"Ship release notes\" --priority 1 --project launch\n  dee-todo list --status open\n  dee-todo list --json\n  dee-todo show 3 --json\n  dee-todo done 3 --json\n  dee-todo edit 3 --title \"Ship v1 release notes\" --tags release,docs\n  dee-todo search release --json\n  dee-todo project launch --json\n  dee-todo delete 3"
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
    /// Add a new todo
    Add(AddArgs),
    /// List todos
    List(ListArgs),
    /// Show todos for one project
    Project(ProjectArgs),
    /// Search todos by text
    Search(SearchArgs),
    /// Mark todo as done
    Done(IdArgs),
    /// Mark todo as open
    Undone(IdArgs),
    /// Show one todo by id
    Show(IdArgs),
    /// Edit fields on an existing todo
    Edit(EditArgs),
    /// Delete one todo
    Delete(IdArgs),
}

#[derive(Debug, Args)]
struct AddArgs {
    title: String,

    #[arg(long, default_value_t = 0)]
    priority: i64,

    #[arg(long)]
    project: Option<String>,

    #[arg(long)]
    due_date: Option<String>,

    #[arg(long)]
    notes: Option<String>,

    /// Comma-separated tags, e.g. work,release
    #[arg(long)]
    tags: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum StatusArg {
    Open,
    Done,
    All,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long, value_enum, default_value_t = StatusArg::Open)]
    status: StatusArg,

    #[arg(long)]
    project: Option<String>,

    #[arg(long)]
    priority: Option<i64>,
}

#[derive(Debug, Args)]
struct ProjectArgs {
    name: String,

    #[arg(long, value_enum, default_value_t = StatusArg::All)]
    status: StatusArg,
}

#[derive(Debug, Args)]
struct SearchArgs {
    query: String,

    #[arg(long, value_enum, default_value_t = StatusArg::All)]
    status: StatusArg,
}

#[derive(Debug, Args)]
struct IdArgs {
    id: i64,
}

#[derive(Debug, Args)]
struct EditArgs {
    id: i64,

    #[arg(long)]
    title: Option<String>,

    #[arg(long)]
    priority: Option<i64>,

    #[arg(long)]
    project: Option<String>,

    #[arg(long)]
    due_date: Option<String>,

    #[arg(long)]
    notes: Option<String>,

    /// Comma-separated tags, e.g. work,release
    #[arg(long)]
    tags: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
struct TodoItem {
    id: i64,
    title: String,
    done: bool,
    priority: i64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    done_at: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Data directory not found")]
    DataDirMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Todo not found")]
    NotFound,
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
            Self::Database => "DATABASE_ERROR",
            Self::Serialize => "SERIALIZE",
        }
    }
}

type AppResult<T> = Result<T, AppError>;

fn main() {
    let cli = Cli::parse();

    let result = run(&cli);

    if let Err(err) = result {
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
        eprintln!("[dee-todo] db_path={}", db_path.display());
    }

    let conn = Connection::open(db_path).map_err(|_| AppError::Database)?;
    initialize_db(&conn)?;

    match &cli.command {
        Commands::Add(args) => cmd_add(&conn, args, &cli.global),
        Commands::List(args) => cmd_list(&conn, args, &cli.global),
        Commands::Project(args) => cmd_project(&conn, args, &cli.global),
        Commands::Search(args) => cmd_search(&conn, args, &cli.global),
        Commands::Done(args) => cmd_done(&conn, args.id, &cli.global),
        Commands::Undone(args) => cmd_undone(&conn, args.id, &cli.global),
        Commands::Show(args) => cmd_show(&conn, args.id, &cli.global),
        Commands::Edit(args) => cmd_edit(&conn, args, &cli.global),
        Commands::Delete(args) => cmd_delete(&conn, args.id, &cli.global),
    }
}

fn db_path() -> AppResult<PathBuf> {
    let base = dirs::data_dir().ok_or(AppError::DataDirMissing)?;
    Ok(base.join("dee-todo").join("todo.db"))
}

fn initialize_db(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS todos (
          id          INTEGER PRIMARY KEY AUTOINCREMENT,
          title       TEXT    NOT NULL,
          done        INTEGER NOT NULL DEFAULT 0,
          priority    INTEGER NOT NULL DEFAULT 0,
          tags        TEXT,
          project     TEXT,
          due_date    TEXT,
          notes       TEXT,
          created_at  TEXT    NOT NULL,
          done_at     TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_todos_done     ON todos(done);
        CREATE INDEX IF NOT EXISTS idx_todos_project  ON todos(project);
        CREATE INDEX IF NOT EXISTS idx_todos_priority ON todos(priority);
        "#,
    )
    .map_err(|_| AppError::Database)
}

fn validate_command_args(command: &Commands) -> AppResult<()> {
    match command {
        Commands::Add(args) => {
            validate_priority(args.priority)?;
            if let Some(due_date) = &args.due_date {
                validate_due_date(due_date)?;
            }
        }
        Commands::List(args) => {
            if let Some(priority) = args.priority {
                validate_priority(priority)?;
            }
        }
        Commands::Edit(args) => {
            if args.title.is_none()
                && args.priority.is_none()
                && args.project.is_none()
                && args.due_date.is_none()
                && args.notes.is_none()
                && args.tags.is_none()
            {
                return Err(AppError::InvalidArgument(
                    "edit requires at least one field".to_string(),
                ));
            }
            if let Some(priority) = args.priority {
                validate_priority(priority)?;
            }
            if let Some(due_date) = &args.due_date {
                validate_due_date(due_date)?;
            }
        }
        Commands::Done(args)
        | Commands::Undone(args)
        | Commands::Show(args)
        | Commands::Delete(args) => {
            if args.id <= 0 {
                return Err(AppError::InvalidArgument(
                    "id must be greater than 0".to_string(),
                ));
            }
        }
        Commands::Project(_) | Commands::Search(_) => {}
    }
    Ok(())
}

fn validate_priority(priority: i64) -> AppResult<()> {
    if (0..=2).contains(&priority) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "priority must be 0, 1, or 2".to_string(),
        ))
    }
}

fn validate_due_date(input: &str) -> AppResult<()> {
    chrono::NaiveDate::parse_from_str(input, "%Y-%m-%d").map_err(|_| {
        AppError::InvalidArgument("due-date must use YYYY-MM-DD format".to_string())
    })?;
    Ok(())
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn cmd_add(conn: &Connection, args: &AddArgs, global: &GlobalFlags) -> AppResult<()> {
    let tags = parse_tags(args.tags.as_deref());
    let tags_json = serde_json::to_string(&tags).map_err(|_| AppError::Serialize)?;
    let created_at = now_iso();

    conn.execute(
        "INSERT INTO todos (title, done, priority, tags, project, due_date, notes, created_at, done_at) VALUES (?1, 0, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
        params![
            args.title,
            args.priority,
            tags_json,
            args.project.as_deref(),
            args.due_date.as_deref(),
            args.notes.as_deref(),
            created_at,
        ],
    )
    .map_err(|_| AppError::Database)?;

    let id = conn.last_insert_rowid();

    if global.json {
        print_json(&ActionResponse {
            ok: true,
            message: "Todo added".to_string(),
            id: Some(id),
        });
        return Ok(());
    }

    if global.quiet {
        println!("{id}");
    } else {
        println!("added todo #{id}: {}", args.title);
    }

    Ok(())
}

fn cmd_list(conn: &Connection, args: &ListArgs, global: &GlobalFlags) -> AppResult<()> {
    let items = fetch_todos(
        conn,
        TodoFilter {
            status: args.status,
            project: args.project.clone(),
            priority: args.priority,
            query: None,
        },
    )?;

    print_list_result(items, global)
}

fn cmd_project(conn: &Connection, args: &ProjectArgs, global: &GlobalFlags) -> AppResult<()> {
    let items = fetch_todos(
        conn,
        TodoFilter {
            status: args.status,
            project: Some(args.name.clone()),
            priority: None,
            query: None,
        },
    )?;

    print_list_result(items, global)
}

fn cmd_search(conn: &Connection, args: &SearchArgs, global: &GlobalFlags) -> AppResult<()> {
    let items = fetch_todos(
        conn,
        TodoFilter {
            status: args.status,
            project: None,
            priority: None,
            query: Some(args.query.clone()),
        },
    )?;

    print_list_result(items, global)
}

fn cmd_done(conn: &Connection, id: i64, global: &GlobalFlags) -> AppResult<()> {
    let updated = conn
        .execute(
            "UPDATE todos SET done = 1, done_at = ?1 WHERE id = ?2",
            params![now_iso(), id],
        )
        .map_err(|_| AppError::Database)?;

    if updated == 0 {
        return Err(AppError::NotFound);
    }

    print_action_result("Todo marked done", id, global)
}

fn cmd_undone(conn: &Connection, id: i64, global: &GlobalFlags) -> AppResult<()> {
    let updated = conn
        .execute(
            "UPDATE todos SET done = 0, done_at = NULL WHERE id = ?1",
            params![id],
        )
        .map_err(|_| AppError::Database)?;

    if updated == 0 {
        return Err(AppError::NotFound);
    }

    print_action_result("Todo marked open", id, global)
}

fn cmd_delete(conn: &Connection, id: i64, global: &GlobalFlags) -> AppResult<()> {
    let deleted = conn
        .execute("DELETE FROM todos WHERE id = ?1", params![id])
        .map_err(|_| AppError::Database)?;

    if deleted == 0 {
        return Err(AppError::NotFound);
    }

    print_action_result("Todo deleted", id, global)
}

fn cmd_show(conn: &Connection, id: i64, global: &GlobalFlags) -> AppResult<()> {
    let item = get_todo_by_id(conn, id)?.ok_or(AppError::NotFound)?;

    if global.json {
        print_json(&ItemResponse { ok: true, item });
        return Ok(());
    }

    if global.quiet {
        println!("{id}");
        return Ok(());
    }

    let status = if item.done { "done" } else { "open" };
    let project = item.project.as_deref().unwrap_or("-");
    println!(
        "#{} [{}] p{} {} {}",
        item.id, status, item.priority, project, item.title
    );
    Ok(())
}

fn cmd_edit(conn: &Connection, args: &EditArgs, global: &GlobalFlags) -> AppResult<()> {
    let existing = get_todo_by_id(conn, args.id)?;
    let Some(existing_item) = existing else {
        return Err(AppError::NotFound);
    };

    let title = args.title.clone().unwrap_or(existing_item.title);
    let priority = args.priority.unwrap_or(existing_item.priority);
    let project = args.project.clone().or(existing_item.project);
    let due_date = args.due_date.clone().or(existing_item.due_date);
    let notes = args.notes.clone().or(existing_item.notes);

    let tags = match &args.tags {
        Some(input) => parse_tags(Some(input)),
        None => existing_item.tags,
    };
    let tags_json = serde_json::to_string(&tags).map_err(|_| AppError::Serialize)?;

    conn.execute(
        "UPDATE todos SET title = ?1, priority = ?2, tags = ?3, project = ?4, due_date = ?5, notes = ?6 WHERE id = ?7",
        params![
            title,
            priority,
            tags_json,
            project.as_deref(),
            due_date.as_deref(),
            notes.as_deref(),
            args.id,
        ],
    )
    .map_err(|_| AppError::Database)?;

    print_action_result("Todo updated", args.id, global)
}

fn print_list_result(items: Vec<TodoItem>, global: &GlobalFlags) -> AppResult<()> {
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
        println!("no todos found");
        return Ok(());
    }

    for item in &items {
        let status = if item.done { "done" } else { "open" };
        let project = item.project.as_deref().unwrap_or("-");
        println!(
            "#{: <4} [{}] p{} {: <12} {}",
            item.id, status, item.priority, project, item.title
        );
    }

    Ok(())
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

#[derive(Debug)]
struct TodoFilter {
    status: StatusArg,
    project: Option<String>,
    priority: Option<i64>,
    query: Option<String>,
}

fn fetch_todos(conn: &Connection, filter: TodoFilter) -> AppResult<Vec<TodoItem>> {
    let mut sql = String::from(
        "SELECT id, title, done, priority, tags, project, due_date, notes, created_at, done_at FROM todos WHERE 1=1",
    );
    let mut bind_values: Vec<rusqlite::types::Value> = Vec::new();

    match filter.status {
        StatusArg::Open => {
            sql.push_str(" AND done = ?");
            bind_values.push(rusqlite::types::Value::Integer(0));
        }
        StatusArg::Done => {
            sql.push_str(" AND done = ?");
            bind_values.push(rusqlite::types::Value::Integer(1));
        }
        StatusArg::All => {}
    }

    if let Some(project) = filter.project {
        sql.push_str(" AND project = ?");
        bind_values.push(rusqlite::types::Value::Text(project));
    }

    if let Some(priority) = filter.priority {
        sql.push_str(" AND priority = ?");
        bind_values.push(rusqlite::types::Value::Integer(priority));
    }

    if let Some(query) = filter.query {
        sql.push_str(" AND (title LIKE ? OR COALESCE(notes, '') LIKE ?)");
        let pattern = format!("%{query}%");
        bind_values.push(rusqlite::types::Value::Text(pattern.clone()));
        bind_values.push(rusqlite::types::Value::Text(pattern));
    }

    sql.push_str(" ORDER BY done ASC, priority DESC, created_at DESC, id DESC");

    let mut stmt = conn.prepare(&sql).map_err(|_| AppError::Database)?;
    let rows = stmt
        .query_map(params_from_iter(bind_values.iter()), parse_todo_row)
        .map_err(|_| AppError::Database)?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|_| AppError::Database)?);
    }

    Ok(items)
}

fn get_todo_by_id(conn: &Connection, id: i64) -> AppResult<Option<TodoItem>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, title, done, priority, tags, project, due_date, notes, created_at, done_at FROM todos WHERE id = ?1",
        )
        .map_err(|_| AppError::Database)?;

    let row = stmt
        .query_row(params![id], parse_todo_row)
        .optional()
        .map_err(|_| AppError::Database)?;

    Ok(row)
}

fn parse_todo_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TodoItem> {
    let tags_json: String = row.get(4)?;
    let tags = serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default();

    Ok(TodoItem {
        id: row.get(0)?,
        title: row.get(1)?,
        done: row.get::<_, i64>(2)? == 1,
        priority: row.get(3)?,
        tags,
        project: row.get(5)?,
        due_date: row.get(6)?,
        notes: row.get(7)?,
        created_at: row.get(8)?,
        done_at: row.get(9)?,
    })
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

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => println!("{text}"),
        Err(_) => {
            println!(r#"{{"ok":false,"error":"JSON serialization failed","code":"SERIALIZE"}}"#);
            std::process::exit(1);
        }
    }
}
