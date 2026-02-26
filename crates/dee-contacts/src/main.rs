use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, SecondsFormat, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "dee-contacts",
    version,
    about = "Personal CRM with local storage and JSON output",
    long_about = "dee-contacts - Manage contacts and interactions from the terminal.",
    after_help = "EXAMPLES:\n  dee-contacts add \"Ada Lovelace\" --email ada@example.com --tags founder,math\n  dee-contacts list --json\n  dee-contacts show ada --json\n  dee-contacts interaction add ada --kind note --summary \"intro call\" --json\n  dee-contacts export --format json --json\n  dee-contacts import --format json contacts.json --json"
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
    /// Add a contact
    Add(AddArgs),
    /// List contacts
    List(ListArgs),
    /// Search contacts
    Search(SearchArgs),
    /// Show one contact by id or fuzzy name
    Show(ResolveArgs),
    /// Edit contact fields
    Edit(EditArgs),
    /// Delete a contact by id
    Delete(IdArgs),
    /// Import contacts from file
    Import(ImportArgs),
    /// Export contacts
    Export(ExportArgs),
    /// Interaction commands
    Interaction(InteractionArgs),
}

#[derive(Debug, Args)]
struct AddArgs {
    name: String,

    #[arg(long)]
    email: Option<String>,

    #[arg(long)]
    phone: Option<String>,

    #[arg(long)]
    company: Option<String>,

    #[arg(long)]
    title: Option<String>,

    #[arg(long)]
    notes: Option<String>,

    /// Comma-separated tags
    #[arg(long)]
    tags: Option<String>,
}

#[derive(Debug, Args)]
struct ListArgs {
    #[arg(long)]
    tag: Option<String>,

    #[arg(long)]
    company: Option<String>,

    #[arg(long, default_value_t = 100)]
    limit: usize,
}

#[derive(Debug, Args)]
struct SearchArgs {
    query: String,

    #[arg(long, default_value_t = 25)]
    limit: usize,
}

#[derive(Debug, Args)]
struct ResolveArgs {
    /// Contact id or name query
    contact: String,
}

#[derive(Debug, Args)]
struct EditArgs {
    id: i64,

    #[arg(long)]
    name: Option<String>,

    #[arg(long)]
    email: Option<String>,

    #[arg(long)]
    phone: Option<String>,

    #[arg(long)]
    company: Option<String>,

    #[arg(long)]
    title: Option<String>,

    #[arg(long)]
    notes: Option<String>,

    /// Comma-separated tags
    #[arg(long)]
    tags: Option<String>,
}

#[derive(Debug, Args)]
struct IdArgs {
    id: i64,
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

#[derive(Debug, Args)]
struct InteractionArgs {
    #[command(subcommand)]
    command: InteractionCommand,
}

#[derive(Debug, Subcommand)]
enum InteractionCommand {
    /// Add interaction for contact (id or fuzzy name)
    Add(InteractionAddArgs),
    /// List interactions for contact (id or fuzzy name)
    List(InteractionListArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum InteractionKind {
    Note,
    Call,
    Email,
    Meeting,
}

#[derive(Debug, Args)]
struct InteractionAddArgs {
    contact: String,

    #[arg(long, value_enum)]
    kind: InteractionKind,

    #[arg(long)]
    summary: String,

    /// RFC3339 timestamp. Defaults to now.
    #[arg(long)]
    occurred_at: Option<String>,
}

#[derive(Debug, Args)]
struct InteractionListArgs {
    contact: String,

    #[arg(long, default_value_t = 50)]
    limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ContactItem {
    id: i64,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InteractionItem {
    id: i64,
    contact_id: i64,
    kind: String,
    summary: String,
    occurred_at: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ContactDetails {
    contact: ContactItem,
    interaction_count: usize,
    interactions: Vec<InteractionItem>,
}

#[derive(Debug, Serialize)]
struct CsvItem {
    format: String,
    data: String,
    count: usize,
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

#[derive(Debug, Serialize, Deserialize)]
struct ExportBundle {
    contacts: Vec<ContactItem>,
    interactions: Vec<InteractionItem>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Data directory not found")]
    DataDirMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Contact not found")]
    NotFound,
    #[error("Contact name is ambiguous. Use id instead")]
    Ambiguous,
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
            Self::Ambiguous => "AMBIGUOUS",
            Self::Database => "DATABASE_ERROR",
            Self::Io => "IO_ERROR",
            Self::Parse => "PARSE_FAILED",
        }
    }
}

type AppResult<T> = Result<T, AppError>;

fn main() {
    let cli = Cli::parse();

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
        eprintln!("[dee-contacts] db_path={}", db_path.display());
    }

    let conn = Connection::open(db_path).map_err(|_| AppError::Database)?;
    init_db(&conn)?;

    match &cli.command {
        Commands::Add(args) => cmd_add(&conn, args, &cli.global),
        Commands::List(args) => cmd_list(&conn, args, &cli.global),
        Commands::Search(args) => cmd_search(&conn, args, &cli.global),
        Commands::Show(args) => cmd_show(&conn, &args.contact, &cli.global),
        Commands::Edit(args) => cmd_edit(&conn, args, &cli.global),
        Commands::Delete(args) => cmd_delete(&conn, args.id, &cli.global),
        Commands::Import(args) => cmd_import(&conn, args, &cli.global),
        Commands::Export(args) => cmd_export(&conn, args, &cli.global),
        Commands::Interaction(args) => cmd_interaction(&conn, args, &cli.global),
    }
}

fn validate_args(command: &Commands) -> AppResult<()> {
    match command {
        Commands::Add(args) => {
            if args.name.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "name must not be empty".to_string(),
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
        }
        Commands::Edit(args) => {
            if args.id <= 0 {
                return Err(AppError::InvalidArgument(
                    "id must be greater than 0".to_string(),
                ));
            }
            if args.name.is_none()
                && args.email.is_none()
                && args.phone.is_none()
                && args.company.is_none()
                && args.title.is_none()
                && args.notes.is_none()
                && args.tags.is_none()
            {
                return Err(AppError::InvalidArgument(
                    "edit requires at least one field".to_string(),
                ));
            }
        }
        Commands::Delete(args) => {
            if args.id <= 0 {
                return Err(AppError::InvalidArgument(
                    "id must be greater than 0".to_string(),
                ));
            }
        }
        Commands::Import(args) => {
            if args.path.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "path must not be empty".to_string(),
                ));
            }
        }
        Commands::Interaction(args) => match &args.command {
            InteractionCommand::Add(cmd) => {
                if cmd.summary.trim().is_empty() {
                    return Err(AppError::InvalidArgument(
                        "summary must not be empty".to_string(),
                    ));
                }
                if let Some(ts) = &cmd.occurred_at {
                    parse_rfc3339(ts)?;
                }
            }
            InteractionCommand::List(cmd) => {
                if cmd.limit == 0 {
                    return Err(AppError::InvalidArgument(
                        "limit must be greater than 0".to_string(),
                    ));
                }
            }
        },
        Commands::Show(_) | Commands::Export(_) => {}
    }

    Ok(())
}

fn db_path() -> AppResult<PathBuf> {
    let base = dirs::data_dir().ok_or(AppError::DataDirMissing)?;
    Ok(base.join("dee-contacts").join("contacts.db"))
}

fn init_db(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS contacts (
          id          INTEGER PRIMARY KEY AUTOINCREMENT,
          name        TEXT    NOT NULL,
          email       TEXT,
          phone       TEXT,
          company     TEXT,
          title       TEXT,
          tags        TEXT    NOT NULL,
          notes       TEXT,
          created_at  TEXT    NOT NULL,
          updated_at  TEXT    NOT NULL
        );

        CREATE TABLE IF NOT EXISTS interactions (
          id          INTEGER PRIMARY KEY AUTOINCREMENT,
          contact_id  INTEGER NOT NULL,
          kind        TEXT    NOT NULL,
          summary     TEXT    NOT NULL,
          occurred_at TEXT    NOT NULL,
          created_at  TEXT    NOT NULL,
          FOREIGN KEY(contact_id) REFERENCES contacts(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_contacts_name ON contacts(name);
        CREATE INDEX IF NOT EXISTS idx_contacts_company ON contacts(company);
        CREATE INDEX IF NOT EXISTS idx_interactions_contact_id ON interactions(contact_id);
        "#,
    )
    .map_err(|_| AppError::Database)
}

fn cmd_add(conn: &Connection, args: &AddArgs, global: &GlobalFlags) -> AppResult<()> {
    let now = now_iso();
    let tags = parse_tags(args.tags.as_deref());
    let tags_json = serde_json::to_string(&tags).map_err(|_| AppError::Parse)?;

    conn.execute(
        "INSERT INTO contacts (name, email, phone, company, title, tags, notes, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            args.name,
            args.email.as_deref(),
            args.phone.as_deref(),
            args.company.as_deref(),
            args.title.as_deref(),
            tags_json,
            args.notes.as_deref(),
            now,
            now,
        ],
    )
    .map_err(|_| AppError::Database)?;

    let id = conn.last_insert_rowid();
    print_action("Contact added", Some(id), None, global)
}

fn cmd_list(conn: &Connection, args: &ListArgs, global: &GlobalFlags) -> AppResult<()> {
    let mut sql = String::from(
        "SELECT id,name,email,phone,company,title,tags,notes,created_at,updated_at FROM contacts WHERE 1=1",
    );
    let mut values = Vec::new();

    if let Some(company) = &args.company {
        sql.push_str(" AND company = ?");
        values.push(rusqlite::types::Value::Text(company.clone()));
    }
    if let Some(tag) = &args.tag {
        sql.push_str(" AND tags LIKE ?");
        values.push(rusqlite::types::Value::Text(format!("%{tag}%")));
    }

    sql.push_str(&format!(
        " ORDER BY updated_at DESC, id DESC LIMIT {}",
        args.limit
    ));

    let items = query_contacts(conn, &sql, values)?;
    print_contact_list(items, global)
}

fn cmd_search(conn: &Connection, args: &SearchArgs, global: &GlobalFlags) -> AppResult<()> {
    let pattern = format!("%{}%", args.query);
    let sql = format!(
        "SELECT id,name,email,phone,company,title,tags,notes,created_at,updated_at FROM contacts
         WHERE name LIKE ? OR COALESCE(email,'') LIKE ? OR COALESCE(company,'') LIKE ? OR COALESCE(notes,'') LIKE ?
         ORDER BY updated_at DESC, id DESC LIMIT {}",
        args.limit
    );

    let items = query_contacts(
        conn,
        &sql,
        vec![
            rusqlite::types::Value::Text(pattern.clone()),
            rusqlite::types::Value::Text(pattern.clone()),
            rusqlite::types::Value::Text(pattern.clone()),
            rusqlite::types::Value::Text(pattern),
        ],
    )?;

    print_contact_list(items, global)
}

fn cmd_show(conn: &Connection, input: &str, global: &GlobalFlags) -> AppResult<()> {
    let id = resolve_contact_id(conn, input)?;
    let contact = get_contact_by_id(conn, id)?.ok_or(AppError::NotFound)?;
    let interactions = list_interactions(conn, id, 100)?;

    let details = ContactDetails {
        contact,
        interaction_count: interactions.len(),
        interactions,
    };

    if global.json {
        print_json(&ItemResponse {
            ok: true,
            item: details,
        });
        return Ok(());
    }

    if global.quiet {
        println!("{id}");
        return Ok(());
    }

    println!("#{} {}", details.contact.id, details.contact.name);
    println!("interactions: {}", details.interaction_count);
    Ok(())
}

fn cmd_edit(conn: &Connection, args: &EditArgs, global: &GlobalFlags) -> AppResult<()> {
    let existing = get_contact_by_id(conn, args.id)?.ok_or(AppError::NotFound)?;

    let name = args.name.clone().unwrap_or(existing.name);
    let email = args.email.clone().or(existing.email);
    let phone = args.phone.clone().or(existing.phone);
    let company = args.company.clone().or(existing.company);
    let title = args.title.clone().or(existing.title);
    let notes = args.notes.clone().or(existing.notes);
    let tags = match &args.tags {
        Some(v) => parse_tags(Some(v)),
        None => existing.tags,
    };
    let tags_json = serde_json::to_string(&tags).map_err(|_| AppError::Parse)?;

    conn.execute(
        "UPDATE contacts SET name = ?1, email = ?2, phone = ?3, company = ?4, title = ?5, tags = ?6, notes = ?7, updated_at = ?8 WHERE id = ?9",
        params![
            name,
            email.as_deref(),
            phone.as_deref(),
            company.as_deref(),
            title.as_deref(),
            tags_json,
            notes.as_deref(),
            now_iso(),
            args.id,
        ],
    )
    .map_err(|_| AppError::Database)?;

    print_action("Contact updated", Some(args.id), None, global)
}

fn cmd_delete(conn: &Connection, id: i64, global: &GlobalFlags) -> AppResult<()> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|_| AppError::Database)?;
    tx.execute(
        "DELETE FROM interactions WHERE contact_id = ?1",
        params![id],
    )
    .map_err(|_| AppError::Database)?;
    let deleted = tx
        .execute("DELETE FROM contacts WHERE id = ?1", params![id])
        .map_err(|_| AppError::Database)?;
    tx.commit().map_err(|_| AppError::Database)?;

    if deleted == 0 {
        return Err(AppError::NotFound);
    }

    print_action("Contact deleted", Some(id), None, global)
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
    let contacts = query_contacts(
        conn,
        "SELECT id,name,email,phone,company,title,tags,notes,created_at,updated_at FROM contacts ORDER BY id ASC",
        Vec::new(),
    )?;

    let interactions = query_all_interactions(conn)?;

    match args.format {
        TransferFormat::Json => {
            if global.json {
                print_json(&ListResponse {
                    ok: true,
                    count: contacts.len(),
                    items: contacts,
                });
            } else if global.quiet {
                println!("{}", contacts.len());
            } else {
                let bundle = ExportBundle {
                    contacts,
                    interactions,
                };
                let data = serde_json::to_string_pretty(&bundle).map_err(|_| AppError::Parse)?;
                println!("{data}");
            }
        }
        TransferFormat::Csv => {
            let csv = contacts_to_csv(&contacts);
            if global.json {
                print_json(&ItemResponse {
                    ok: true,
                    item: CsvItem {
                        format: "csv".to_string(),
                        data: csv,
                        count: contacts.len(),
                    },
                });
            } else {
                print!("{csv}");
            }
        }
    }

    Ok(())
}

fn cmd_interaction(
    conn: &Connection,
    args: &InteractionArgs,
    global: &GlobalFlags,
) -> AppResult<()> {
    match &args.command {
        InteractionCommand::Add(cmd) => {
            let contact_id = resolve_contact_id(conn, &cmd.contact)?;
            let occurred_at = cmd.occurred_at.clone().unwrap_or_else(now_iso);

            conn.execute(
                "INSERT INTO interactions (contact_id, kind, summary, occurred_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    contact_id,
                    kind_to_str(cmd.kind),
                    cmd.summary,
                    occurred_at,
                    now_iso(),
                ],
            )
            .map_err(|_| AppError::Database)?;

            let id = conn.last_insert_rowid();
            print_action("Interaction added", Some(id), None, global)
        }
        InteractionCommand::List(cmd) => {
            let contact_id = resolve_contact_id(conn, &cmd.contact)?;
            let items = list_interactions(conn, contact_id, cmd.limit)?;

            if global.json {
                print_json(&ListResponse {
                    ok: true,
                    count: items.len(),
                    items,
                });
            } else if global.quiet {
                println!("{}", items.len());
            } else if items.is_empty() {
                println!("no interactions found");
            } else {
                for item in items {
                    println!("#{} [{}] {}", item.id, item.kind, item.summary);
                }
            }
            Ok(())
        }
    }
}

fn query_contacts(
    conn: &Connection,
    sql: &str,
    values: Vec<rusqlite::types::Value>,
) -> AppResult<Vec<ContactItem>> {
    let mut stmt = conn.prepare(sql).map_err(|_| AppError::Database)?;
    let rows = stmt
        .query_map(params_from_iter(values.iter()), parse_contact_row)
        .map_err(|_| AppError::Database)?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|_| AppError::Database)?);
    }

    Ok(items)
}

fn get_contact_by_id(conn: &Connection, id: i64) -> AppResult<Option<ContactItem>> {
    let mut stmt = conn
        .prepare(
            "SELECT id,name,email,phone,company,title,tags,notes,created_at,updated_at FROM contacts WHERE id = ?1",
        )
        .map_err(|_| AppError::Database)?;

    stmt.query_row(params![id], parse_contact_row)
        .optional()
        .map_err(|_| AppError::Database)
}

fn parse_contact_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContactItem> {
    let tags_json: String = row.get(6)?;
    let tags = serde_json::from_str::<Vec<String>>(&tags_json).unwrap_or_default();

    Ok(ContactItem {
        id: row.get(0)?,
        name: row.get(1)?,
        email: row.get(2)?,
        phone: row.get(3)?,
        company: row.get(4)?,
        title: row.get(5)?,
        tags,
        notes: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn resolve_contact_id(conn: &Connection, input: &str) -> AppResult<i64> {
    if let Ok(id) = input.parse::<i64>() {
        if get_contact_by_id(conn, id)?.is_some() {
            return Ok(id);
        }
    }

    let mut exact_stmt = conn
        .prepare("SELECT id FROM contacts WHERE lower(name) = lower(?1) LIMIT 1")
        .map_err(|_| AppError::Database)?;

    if let Some(id) = exact_stmt
        .query_row(params![input], |row| row.get::<_, i64>(0))
        .optional()
        .map_err(|_| AppError::Database)?
    {
        return Ok(id);
    }

    let pattern = format!("%{input}%");
    let mut fuzzy_stmt = conn
        .prepare("SELECT id FROM contacts WHERE lower(name) LIKE lower(?1) ORDER BY updated_at DESC LIMIT 2")
        .map_err(|_| AppError::Database)?;

    let rows = fuzzy_stmt
        .query_map(params![pattern], |row| row.get::<_, i64>(0))
        .map_err(|_| AppError::Database)?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row.map_err(|_| AppError::Database)?);
    }

    match ids.len() {
        0 => Err(AppError::NotFound),
        1 => Ok(ids[0]),
        _ => Err(AppError::Ambiguous),
    }
}

fn list_interactions(
    conn: &Connection,
    contact_id: i64,
    limit: usize,
) -> AppResult<Vec<InteractionItem>> {
    let mut stmt = conn
        .prepare(
            &format!(
                "SELECT id,contact_id,kind,summary,occurred_at,created_at FROM interactions WHERE contact_id = ?1 ORDER BY occurred_at DESC, id DESC LIMIT {}",
                limit
            ),
        )
        .map_err(|_| AppError::Database)?;

    let rows = stmt
        .query_map(params![contact_id], parse_interaction_row)
        .map_err(|_| AppError::Database)?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|_| AppError::Database)?);
    }

    Ok(items)
}

fn query_all_interactions(conn: &Connection) -> AppResult<Vec<InteractionItem>> {
    let mut stmt = conn
        .prepare(
            "SELECT id,contact_id,kind,summary,occurred_at,created_at FROM interactions ORDER BY id ASC",
        )
        .map_err(|_| AppError::Database)?;
    let rows = stmt
        .query_map([], parse_interaction_row)
        .map_err(|_| AppError::Database)?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|_| AppError::Database)?);
    }

    Ok(items)
}

fn parse_interaction_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<InteractionItem> {
    Ok(InteractionItem {
        id: row.get(0)?,
        contact_id: row.get(1)?,
        kind: row.get(2)?,
        summary: row.get(3)?,
        occurred_at: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn print_contact_list(items: Vec<ContactItem>, global: &GlobalFlags) -> AppResult<()> {
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
        println!("no contacts found");
        return Ok(());
    }

    for item in items {
        println!("#{} {}", item.id, item.name);
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
        return Ok(());
    }

    if global.quiet {
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
        .filter(|t| !t.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_rfc3339(input: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(input)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::InvalidArgument("occurred-at must be RFC3339".to_string()))
}

fn kind_to_str(kind: InteractionKind) -> &'static str {
    match kind {
        InteractionKind::Note => "note",
        InteractionKind::Call => "call",
        InteractionKind::Email => "email",
        InteractionKind::Meeting => "meeting",
    }
}

fn import_json(conn: &Connection, content: &str) -> AppResult<usize> {
    let parsed: serde_json::Value = serde_json::from_str(content).map_err(|_| AppError::Parse)?;

    let bundle: ExportBundle = if parsed.is_array() {
        let contacts: Vec<ContactItem> =
            serde_json::from_value(parsed).map_err(|_| AppError::Parse)?;
        ExportBundle {
            contacts,
            interactions: Vec::new(),
        }
    } else if parsed.get("contacts").is_some() {
        serde_json::from_value(parsed).map_err(|_| AppError::Parse)?
    } else if parsed.get("items").is_some() {
        let contacts: Vec<ContactItem> =
            serde_json::from_value(parsed.get("items").cloned().ok_or(AppError::Parse)?)
                .map_err(|_| AppError::Parse)?;
        ExportBundle {
            contacts,
            interactions: Vec::new(),
        }
    } else {
        return Err(AppError::Parse);
    };

    let tx = conn
        .unchecked_transaction()
        .map_err(|_| AppError::Database)?;
    let mut imported = 0usize;

    for contact in bundle.contacts {
        let created_at = if contact.created_at.is_empty() {
            now_iso()
        } else {
            contact.created_at
        };
        let updated_at = if contact.updated_at.is_empty() {
            created_at.clone()
        } else {
            contact.updated_at
        };
        let tags_json = serde_json::to_string(&contact.tags).map_err(|_| AppError::Parse)?;
        tx.execute(
            "INSERT INTO contacts (name,email,phone,company,title,tags,notes,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                contact.name,
                contact.email.as_deref(),
                contact.phone.as_deref(),
                contact.company.as_deref(),
                contact.title.as_deref(),
                tags_json,
                contact.notes.as_deref(),
                created_at,
                updated_at,
            ],
        )
        .map_err(|_| AppError::Database)?;
        imported += 1;
    }

    for interaction in bundle.interactions {
        tx.execute(
            "INSERT INTO interactions (contact_id,kind,summary,occurred_at,created_at) VALUES (?1,?2,?3,?4,?5)",
            params![
                interaction.contact_id,
                interaction.kind,
                interaction.summary,
                interaction.occurred_at,
                interaction.created_at,
            ],
        )
        .map_err(|_| AppError::Database)?;
    }

    tx.commit().map_err(|_| AppError::Database)?;
    Ok(imported)
}

fn import_csv(conn: &Connection, content: &str) -> AppResult<usize> {
    let mut lines = content.lines();
    let header = lines.next().ok_or(AppError::Parse)?;

    if !header.contains("name") {
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
        if cols.len() < 7 {
            continue;
        }

        let now = now_iso();
        tx.execute(
            "INSERT INTO contacts (name,email,phone,company,title,tags,notes,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                cols[0],
                empty_to_none(&cols[1]),
                empty_to_none(&cols[2]),
                empty_to_none(&cols[3]),
                empty_to_none(&cols[4]),
                serde_json::to_string(&parse_tags(Some(&cols[5]))).map_err(|_| AppError::Parse)?,
                empty_to_none(&cols[6]),
                now,
                now,
            ],
        )
        .map_err(|_| AppError::Database)?;
        imported += 1;
    }

    tx.commit().map_err(|_| AppError::Database)?;
    Ok(imported)
}

fn contacts_to_csv(contacts: &[ContactItem]) -> String {
    let mut out = String::from("name,email,phone,company,title,tags,notes\n");
    for c in contacts {
        let fields = [
            csv_escape(&c.name),
            csv_escape(c.email.as_deref().unwrap_or("")),
            csv_escape(c.phone.as_deref().unwrap_or("")),
            csv_escape(c.company.as_deref().unwrap_or("")),
            csv_escape(c.title.as_deref().unwrap_or("")),
            csv_escape(&c.tags.join("|")),
            csv_escape(c.notes.as_deref().unwrap_or("")),
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
    let mut current = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        match chars[i] {
            '"' => {
                if in_quotes && i + 1 < chars.len() && chars[i + 1] == '"' {
                    current.push('"');
                    i += 1;
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                out.push(current.clone());
                current.clear();
            }
            ch => current.push(ch),
        }
        i += 1;
    }
    out.push(current);
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
