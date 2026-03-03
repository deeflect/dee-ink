use std::fs;
use std::path::PathBuf;

use chrono::{SecondsFormat, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "dee-stash",
    version,
    about = "Bookmark and read-later CLI",
    long_about = "dee-stash - Save URLs, organize with tags, and manage a local read-later stash.",
    after_help = "EXAMPLES:\n  dee-stash add https://example.com --title \"Example\" --tags research,tools\n  dee-stash list --status unread --json\n  dee-stash search rust --json\n  dee-stash archive 3 --json\n  dee-stash export --format json --json\n  dee-stash import --format csv bookmarks.csv --json"
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
    /// Add bookmark
    Add(AddArgs),
    /// List bookmarks
    List(ListArgs),
    /// Search bookmarks
    Search(SearchArgs),
    /// Show one bookmark by id
    Show(IdArgs),
    /// Edit bookmark
    Edit(EditArgs),
    /// Delete bookmark
    Delete(IdArgs),
    /// Mark bookmark as archived/read
    Archive(IdArgs),
    /// Mark bookmark as unread
    Unarchive(IdArgs),
    /// Import bookmarks
    Import(ImportArgs),
    /// Export bookmarks
    Export(ExportArgs),
}

#[derive(Debug, Args)]
struct AddArgs {
    url: String,

    #[arg(long)]
    title: Option<String>,

    #[arg(long)]
    notes: Option<String>,

    /// Comma-separated tags
    #[arg(long)]
    tags: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum StatusArg {
    Unread,
    Archived,
    All,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long, value_enum, default_value_t = StatusArg::Unread)]
    status: StatusArg,

    #[arg(long)]
    tag: Option<String>,

    #[arg(long, default_value_t = 100)]
    limit: usize,
}

#[derive(Debug, Args)]
struct SearchArgs {
    query: String,

    #[arg(long, value_enum, default_value_t = StatusArg::All)]
    status: StatusArg,

    #[arg(long, default_value_t = 50)]
    limit: usize,
}

#[derive(Debug, Args)]
struct IdArgs {
    id: i64,
}

#[derive(Debug, Args)]
struct EditArgs {
    id: i64,

    #[arg(long)]
    url: Option<String>,

    #[arg(long)]
    title: Option<String>,

    #[arg(long)]
    notes: Option<String>,

    #[arg(long)]
    tags: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TransferFormat {
    Json,
    Csv,
}

#[derive(Debug, Args)]
struct ImportArgs {
    #[arg(long, value_enum)]
    format: TransferFormat,

    path: String,
}

#[derive(Debug, Args)]
struct ExportArgs {
    #[arg(long, value_enum, default_value_t = TransferFormat::Json)]
    format: TransferFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BookmarkItem {
    id: i64,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    archived: bool,
    created_at: String,
    updated_at: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, Serialize)]
struct CsvItem {
    format: String,
    data: String,
    count: usize,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Data directory not found")]
    DataDirMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Bookmark not found")]
    NotFound,
    #[error("Bookmark already exists")]
    Duplicate,
    #[error("Database operation failed")]
    Database,
    #[error("Request file could not be read")]
    Io,
    #[error("Input parse failed")]
    Parse,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::DataDirMissing => "CONFIG_MISSING",
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::NotFound => "NOT_FOUND",
            Self::Duplicate => "DUPLICATE",
            Self::Database => "DATABASE_ERROR",
            Self::Io => "IO_ERROR",
            Self::Parse => "PARSE_FAILED",
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
    validate_args(&cli.command)?;

    let db_path = db_path()?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|_| AppError::Database)?;
    }
    if cli.global.verbose {
        eprintln!("[dee-stash] db_path={}", db_path.display());
    }

    let conn = Connection::open(db_path).map_err(|_| AppError::Database)?;
    init_db(&conn)?;

    match &cli.command {
        Commands::Add(args) => cmd_add(&conn, args, &cli.global),
        Commands::List(args) => cmd_list(&conn, args, &cli.global),
        Commands::Search(args) => cmd_search(&conn, args, &cli.global),
        Commands::Show(args) => cmd_show(&conn, args.id, &cli.global),
        Commands::Edit(args) => cmd_edit(&conn, args, &cli.global),
        Commands::Delete(args) => cmd_delete(&conn, args.id, &cli.global),
        Commands::Archive(args) => cmd_archive(&conn, args.id, true, &cli.global),
        Commands::Unarchive(args) => cmd_archive(&conn, args.id, false, &cli.global),
        Commands::Import(args) => cmd_import(&conn, args, &cli.global),
        Commands::Export(args) => cmd_export(&conn, args, &cli.global),
    }
}

fn validate_args(command: &Commands) -> AppResult<()> {
    match command {
        Commands::Add(args) => validate_url(&args.url),
        Commands::List(args) => {
            if args.limit == 0 {
                return Err(AppError::InvalidArgument(
                    "limit must be greater than 0".to_string(),
                ));
            }
            Ok(())
        }
        Commands::Search(args) => {
            if args.query.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "query must not be empty".to_string(),
                ));
            }
            if args.limit == 0 {
                return Err(AppError::InvalidArgument(
                    "limit must be greater than 0".to_string(),
                ));
            }
            Ok(())
        }
        Commands::Show(args)
        | Commands::Delete(args)
        | Commands::Archive(args)
        | Commands::Unarchive(args) => validate_id(args.id),
        Commands::Edit(args) => {
            validate_id(args.id)?;
            if args.url.is_none()
                && args.title.is_none()
                && args.notes.is_none()
                && args.tags.is_none()
            {
                return Err(AppError::InvalidArgument(
                    "edit requires at least one field".to_string(),
                ));
            }
            if let Some(url) = &args.url {
                validate_url(url)?;
            }
            Ok(())
        }
        Commands::Import(args) => {
            if args.path.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "path must not be empty".to_string(),
                ));
            }
            Ok(())
        }
        Commands::Export(_) => Ok(()),
    }
}

fn validate_id(id: i64) -> AppResult<()> {
    if id <= 0 {
        Err(AppError::InvalidArgument(
            "id must be greater than 0".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn validate_url(url: &str) -> AppResult<()> {
    let ok = url.starts_with("http://") || url.starts_with("https://");
    if ok {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(
            "url must start with http:// or https://".to_string(),
        ))
    }
}

fn db_path() -> AppResult<PathBuf> {
    let base = dirs::data_dir().ok_or(AppError::DataDirMissing)?;
    Ok(base.join("dee-stash").join("stash.db"))
}

fn init_db(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS bookmarks (
          id          INTEGER PRIMARY KEY AUTOINCREMENT,
          url         TEXT    NOT NULL UNIQUE,
          title       TEXT,
          tags        TEXT    NOT NULL,
          notes       TEXT,
          archived    INTEGER NOT NULL DEFAULT 0,
          created_at  TEXT    NOT NULL,
          updated_at  TEXT    NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_bookmarks_archived ON bookmarks(archived);
        CREATE INDEX IF NOT EXISTS idx_bookmarks_updated ON bookmarks(updated_at);
        "#,
    )
    .map_err(|_| AppError::Database)
}

fn cmd_add(conn: &Connection, args: &AddArgs, global: &GlobalFlags) -> AppResult<()> {
    if bookmark_by_url(conn, &args.url)?.is_some() {
        return Err(AppError::Duplicate);
    }

    let now = now_iso();
    let tags_json =
        serde_json::to_string(&parse_tags(args.tags.as_deref())).map_err(|_| AppError::Parse)?;

    conn.execute(
        "INSERT INTO bookmarks (url,title,tags,notes,archived,created_at,updated_at) VALUES (?1,?2,?3,?4,0,?5,?6)",
        params![args.url, args.title.as_deref(), tags_json, args.notes.as_deref(), now, now],
    )
    .map_err(|_| AppError::Database)?;

    print_action(
        "Bookmark added",
        Some(conn.last_insert_rowid()),
        None,
        global,
    )
}

fn cmd_list(conn: &Connection, args: &ListArgs, global: &GlobalFlags) -> AppResult<()> {
    let mut sql = String::from(
        "SELECT id,url,title,tags,notes,archived,created_at,updated_at FROM bookmarks WHERE 1=1",
    );
    let mut vals = Vec::new();

    match args.status {
        StatusArg::Unread => sql.push_str(" AND archived = 0"),
        StatusArg::Archived => sql.push_str(" AND archived = 1"),
        StatusArg::All => {}
    }

    if let Some(tag) = &args.tag {
        sql.push_str(" AND tags LIKE ?");
        vals.push(rusqlite::types::Value::Text(format!("%{tag}%")));
    }

    sql.push_str(&format!(
        " ORDER BY updated_at DESC, id DESC LIMIT {}",
        args.limit
    ));
    let items = query_bookmarks(conn, &sql, vals)?;
    print_list(items, global)
}

fn cmd_search(conn: &Connection, args: &SearchArgs, global: &GlobalFlags) -> AppResult<()> {
    let pattern = format!("%{}%", args.query);
    let mut sql = String::from(
        "SELECT id,url,title,tags,notes,archived,created_at,updated_at FROM bookmarks WHERE (url LIKE ? OR COALESCE(title,'') LIKE ? OR COALESCE(notes,'') LIKE ? OR tags LIKE ?)",
    );
    let mut vals = vec![
        rusqlite::types::Value::Text(pattern.clone()),
        rusqlite::types::Value::Text(pattern.clone()),
        rusqlite::types::Value::Text(pattern.clone()),
        rusqlite::types::Value::Text(pattern),
    ];

    match args.status {
        StatusArg::Unread => sql.push_str(" AND archived = 0"),
        StatusArg::Archived => sql.push_str(" AND archived = 1"),
        StatusArg::All => {}
    }

    sql.push_str(&format!(
        " ORDER BY updated_at DESC, id DESC LIMIT {}",
        args.limit
    ));

    let items = query_bookmarks(conn, &sql, std::mem::take(&mut vals))?;
    print_list(items, global)
}

fn cmd_show(conn: &Connection, id: i64, global: &GlobalFlags) -> AppResult<()> {
    let item = bookmark_by_id(conn, id)?.ok_or(AppError::NotFound)?;

    if global.json {
        print_json(&ItemResponse { ok: true, item });
    } else if global.quiet {
        println!("{id}");
    } else {
        let status = if item.archived { "archived" } else { "unread" };
        println!("#{} [{}] {}", item.id, status, item.url);
    }
    Ok(())
}

fn cmd_edit(conn: &Connection, args: &EditArgs, global: &GlobalFlags) -> AppResult<()> {
    let existing = bookmark_by_id(conn, args.id)?.ok_or(AppError::NotFound)?;

    let url = args.url.clone().unwrap_or(existing.url);
    let title = args.title.clone().or(existing.title);
    let notes = args.notes.clone().or(existing.notes);
    let tags = match &args.tags {
        Some(v) => parse_tags(Some(v)),
        None => existing.tags,
    };
    let tags_json = serde_json::to_string(&tags).map_err(|_| AppError::Parse)?;

    conn.execute(
        "UPDATE bookmarks SET url=?1,title=?2,tags=?3,notes=?4,updated_at=?5 WHERE id=?6",
        params![
            url,
            title.as_deref(),
            tags_json,
            notes.as_deref(),
            now_iso(),
            args.id
        ],
    )
    .map_err(|_| AppError::Database)?;

    print_action("Bookmark updated", Some(args.id), None, global)
}

fn cmd_delete(conn: &Connection, id: i64, global: &GlobalFlags) -> AppResult<()> {
    let deleted = conn
        .execute("DELETE FROM bookmarks WHERE id = ?1", params![id])
        .map_err(|_| AppError::Database)?;

    if deleted == 0 {
        return Err(AppError::NotFound);
    }

    print_action("Bookmark deleted", Some(id), None, global)
}

fn cmd_archive(conn: &Connection, id: i64, archived: bool, global: &GlobalFlags) -> AppResult<()> {
    let updated = conn
        .execute(
            "UPDATE bookmarks SET archived = ?1, updated_at = ?2 WHERE id = ?3",
            params![if archived { 1 } else { 0 }, now_iso(), id],
        )
        .map_err(|_| AppError::Database)?;

    if updated == 0 {
        return Err(AppError::NotFound);
    }

    let msg = if archived {
        "Bookmark archived"
    } else {
        "Bookmark unarchived"
    };
    print_action(msg, Some(id), None, global)
}

fn cmd_import(conn: &Connection, args: &ImportArgs, global: &GlobalFlags) -> AppResult<()> {
    let content = fs::read_to_string(&args.path).map_err(|_| AppError::Io)?;
    let count = match args.format {
        TransferFormat::Json => import_json(conn, &content)?,
        TransferFormat::Csv => import_csv(conn, &content)?,
    };
    print_action("Import complete", None, Some(count), global)
}

fn cmd_export(conn: &Connection, args: &ExportArgs, global: &GlobalFlags) -> AppResult<()> {
    let items = query_bookmarks(
        conn,
        "SELECT id,url,title,tags,notes,archived,created_at,updated_at FROM bookmarks ORDER BY id ASC",
        Vec::new(),
    )?;

    match args.format {
        TransferFormat::Json => {
            if global.json {
                print_json(&ListResponse {
                    ok: true,
                    count: items.len(),
                    items,
                });
            } else if global.quiet {
                println!("{}", items.len());
            } else {
                let out = serde_json::to_string_pretty(&items).map_err(|_| AppError::Parse)?;
                println!("{out}");
            }
        }
        TransferFormat::Csv => {
            let csv = bookmarks_to_csv(&items);
            if global.json {
                print_json(&ItemResponse {
                    ok: true,
                    item: CsvItem {
                        format: "csv".to_string(),
                        data: csv,
                        count: items.len(),
                    },
                });
            } else {
                print!("{csv}");
            }
        }
    }
    Ok(())
}

fn query_bookmarks(
    conn: &Connection,
    sql: &str,
    values: Vec<rusqlite::types::Value>,
) -> AppResult<Vec<BookmarkItem>> {
    let mut stmt = conn.prepare(sql).map_err(|_| AppError::Database)?;
    let rows = stmt
        .query_map(params_from_iter(values.iter()), parse_bookmark_row)
        .map_err(|_| AppError::Database)?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|_| AppError::Database)?);
    }
    Ok(items)
}

fn bookmark_by_id(conn: &Connection, id: i64) -> AppResult<Option<BookmarkItem>> {
    let mut stmt = conn
        .prepare("SELECT id,url,title,tags,notes,archived,created_at,updated_at FROM bookmarks WHERE id = ?1")
        .map_err(|_| AppError::Database)?;

    stmt.query_row(params![id], parse_bookmark_row)
        .optional()
        .map_err(|_| AppError::Database)
}

fn bookmark_by_url(conn: &Connection, url: &str) -> AppResult<Option<i64>> {
    let mut stmt = conn
        .prepare("SELECT id FROM bookmarks WHERE url = ?1")
        .map_err(|_| AppError::Database)?;
    stmt.query_row(params![url], |r| r.get::<_, i64>(0))
        .optional()
        .map_err(|_| AppError::Database)
}

fn parse_bookmark_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BookmarkItem> {
    let tags_json: String = row.get(3)?;
    let tags = serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default();
    Ok(BookmarkItem {
        id: row.get(0)?,
        url: row.get(1)?,
        title: row.get(2)?,
        tags,
        notes: row.get(4)?,
        archived: row.get::<_, i64>(5)? == 1,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn print_list(items: Vec<BookmarkItem>, global: &GlobalFlags) -> AppResult<()> {
    if global.json {
        print_json(&ListResponse {
            ok: true,
            count: items.len(),
            items,
        });
    } else if global.quiet {
        println!("{}", items.len());
    } else if items.is_empty() {
        println!("no bookmarks found");
    } else {
        for item in items {
            let status = if item.archived { "archived" } else { "unread" };
            println!("#{} [{}] {}", item.id, status, item.url);
        }
    }
    Ok(())
}

fn print_action(
    message: &str,
    id: Option<i64>,
    count: Option<usize>,
    global: &GlobalFlags,
) -> AppResult<()> {
    if global.json {
        print_json(&ActionResponse {
            ok: true,
            message: message.to_string(),
            id,
            count,
        });
    } else if global.quiet {
        if let Some(id) = id {
            println!("{id}");
        } else if let Some(count) = count {
            println!("{count}");
        } else {
            println!("1");
        }
    } else {
        println!("{message}");
    }
    Ok(())
}

fn parse_tags(input: Option<&str>) -> Vec<String> {
    input
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn import_json(conn: &Connection, content: &str) -> AppResult<usize> {
    let items: Vec<BookmarkItem> = serde_json::from_str(content).map_err(|_| AppError::Parse)?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|_| AppError::Database)?;
    let mut imported = 0usize;

    for item in items {
        let tags_json = serde_json::to_string(&item.tags).map_err(|_| AppError::Parse)?;
        tx.execute(
            "INSERT OR IGNORE INTO bookmarks (url,title,tags,notes,archived,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                item.url,
                item.title.as_deref(),
                tags_json,
                item.notes.as_deref(),
                if item.archived { 1 } else { 0 },
                if item.created_at.is_empty() { now_iso() } else { item.created_at },
                if item.updated_at.is_empty() { now_iso() } else { item.updated_at },
            ],
        )
        .map_err(|_| AppError::Database)?;
        imported += 1;
    }

    tx.commit().map_err(|_| AppError::Database)?;
    Ok(imported)
}

fn import_csv(conn: &Connection, content: &str) -> AppResult<usize> {
    let mut lines = content.lines();
    let header = lines.next().ok_or(AppError::Parse)?;
    if !header.contains("url") {
        return Err(AppError::Parse);
    }

    let tx = conn
        .unchecked_transaction()
        .map_err(|_| AppError::Database)?;
    let mut imported = 0usize;

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols = parse_csv_line(line);
        if cols.len() < 5 {
            continue;
        }

        tx.execute(
            "INSERT OR IGNORE INTO bookmarks (url,title,tags,notes,archived,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                cols[0],
                empty_to_none(&cols[1]),
                serde_json::to_string(&parse_tags(Some(&cols[2]))).map_err(|_| AppError::Parse)?,
                empty_to_none(&cols[3]),
                if cols[4].trim() == "1" { 1 } else { 0 },
                now_iso(),
                now_iso(),
            ],
        )
        .map_err(|_| AppError::Database)?;
        imported += 1;
    }

    tx.commit().map_err(|_| AppError::Database)?;
    Ok(imported)
}

fn bookmarks_to_csv(items: &[BookmarkItem]) -> String {
    let mut out = String::from("url,title,tags,notes,archived\n");
    for item in items {
        let fields = [
            csv_escape(&item.url),
            csv_escape(item.title.as_deref().unwrap_or("")),
            csv_escape(&item.tags.join("|")),
            csv_escape(item.notes.as_deref().unwrap_or("")),
            if item.archived { "1" } else { "0" }.to_string(),
        ];
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                if in_quotes && i + 1 < chars.len() && chars[i + 1] == '"' {
                    cur.push('"');
                    i += 1;
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                out.push(cur.clone());
                cur.clear();
            }
            ch => cur.push(ch),
        }
        i += 1;
    }
    out.push(cur);
    out
}

fn empty_to_none(value: &str) -> Option<&str> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => println!("{text}"),
        Err(_) => {
            println!(
                "{{\"ok\":false,\"error\":\"JSON serialization failed\",\"code\":\"SERIALIZE\"}}"
            );
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
