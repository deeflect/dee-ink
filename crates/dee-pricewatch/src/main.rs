use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use clap::{Args, Parser, Subcommand};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::Url;
use rusqlite::{params, Connection, OptionalExtension};
use scraper::{Html, Selector};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "dee-pricewatch",
    version,
    about = "Track product prices across webpages",
    long_about = "dee-pricewatch - Monitor webpage prices locally with consistent JSON output.",
    after_help = "EXAMPLES:\n  dee-pricewatch add \"https://example.com/product\" --target-price 19.99\n  dee-pricewatch list --json\n  dee-pricewatch check --json\n  dee-pricewatch check 2 --json\n  dee-pricewatch delete 2 --json"
)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Args)]
struct GlobalArgs {
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
    /// Add a URL to watch
    Add(AddArgs),
    /// List watches
    List,
    /// Check current prices (all watches by default)
    Check(CheckArgs),
    /// Delete a watch
    Delete(WatchArg),
}

#[derive(Debug, Args)]
struct AddArgs {
    url: String,

    #[arg(long)]
    label: Option<String>,

    #[arg(long)]
    target_price: Option<f64>,

    /// CSS selector for price text extraction
    #[arg(long)]
    selector: Option<String>,

    /// Currency fallback when page has no symbol
    #[arg(long, default_value = "USD")]
    currency: String,

    /// Seed last known price without fetching
    #[arg(long)]
    initial_price: Option<f64>,
}

#[derive(Debug, Args)]
struct CheckArgs {
    /// Optional watch id or label; defaults to all
    watch: Option<String>,

    #[arg(long, default_value_t = 20)]
    timeout_secs: u64,
}

#[derive(Debug, Clone, Args)]
struct WatchArg {
    watch: String,
}

#[derive(Debug, Serialize)]
struct OkList<T> {
    ok: bool,
    count: usize,
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct OkMessage {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
}

#[derive(Debug, Serialize)]
struct ErrorJson {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, Clone)]
struct WatchRecord {
    id: i64,
    url: String,
    label: String,
    target_price: Option<f64>,
    selector: Option<String>,
    last_price: Option<f64>,
    last_currency: Option<String>,
    last_checked_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct WatchItem {
    id: i64,
    url: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_checked_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct CheckItem {
    id: i64,
    url: String,
    label: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    previous_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dropped: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_hit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    checked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Data directory not found")]
    DataDirMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Watch not found")]
    NotFound,
    #[error("Database operation failed")]
    Database,
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Price parse failed: {0}")]
    ParseFailed(String),
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
            Self::RequestFailed(_) => "REQUEST_FAILED",
            Self::ParseFailed(_) => "PARSE_FAILED",
            Self::Serialize => "SERIALIZE",
        }
    }
}

type AppResult<T> = Result<T, AppError>;

fn main() {
    let cli = parse_cli();

    if let Err(err) = dispatch(&cli) {
        if cli.global.json {
            let payload = ErrorJson {
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

fn dispatch(cli: &Cli) -> AppResult<()> {
    let db_path = db_path()?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|_| AppError::Database)?;
    }

    if cli.global.verbose {
        eprintln!("[dee-pricewatch] db_path={}", db_path.display());
    }

    let conn = Connection::open(db_path).map_err(|_| AppError::Database)?;
    initialize_db(&conn)?;

    match &cli.command {
        Commands::Add(args) => cmd_add(&conn, args, &cli.global),
        Commands::List => cmd_list(&conn, &cli.global),
        Commands::Check(args) => cmd_check(&conn, args, &cli.global),
        Commands::Delete(args) => cmd_delete(&conn, args, &cli.global),
    }
}

fn cmd_add(conn: &Connection, args: &AddArgs, global: &GlobalArgs) -> AppResult<()> {
    validate_url(&args.url)?;
    validate_positive_price(args.target_price, "target-price")?;
    validate_positive_price(args.initial_price, "initial-price")?;

    let currency = normalize_currency(&args.currency)?;
    let label = derive_label(args.label.as_deref(), &args.url)?;
    let now = now_timestamp();

    conn.execute(
        "INSERT INTO watches
         (url, label, target_price, selector, last_price, last_currency, last_checked_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            args.url,
            label,
            args.target_price,
            args.selector,
            args.initial_price,
            args.initial_price.map(|_| currency.clone()),
            args.initial_price.map(|_| now.clone()),
            now,
            now,
        ],
    )
    .map_err(|_| AppError::Database)?;

    let id = conn.last_insert_rowid();
    let message = "Watch added".to_string();

    if global.json {
        write_json(&OkMessage {
            ok: true,
            message,
            id: Some(id),
        })
    } else if global.quiet {
        println!("{id}");
        Ok(())
    } else {
        println!("Added watch #{id}: {}", args.url);
        Ok(())
    }
}

fn cmd_list(conn: &Connection, global: &GlobalArgs) -> AppResult<()> {
    let watches = list_watches(conn)?;
    let items = watches
        .into_iter()
        .map(|watch| WatchItem {
            id: watch.id,
            url: watch.url,
            label: watch.label,
            target_price: watch.target_price,
            selector: watch.selector,
            last_price: watch.last_price,
            last_currency: watch.last_currency,
            last_checked_at: watch.last_checked_at,
            created_at: watch.created_at,
            updated_at: watch.updated_at,
        })
        .collect::<Vec<_>>();

    if global.json {
        write_json(&OkList {
            ok: true,
            count: items.len(),
            items,
        })
    } else if global.quiet {
        println!("{}", items.len());
        Ok(())
    } else if items.is_empty() {
        println!("No watches yet.");
        Ok(())
    } else {
        for item in &items {
            let last = item
                .last_price
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "-".to_string());
            let currency = item.last_currency.as_deref().unwrap_or("-");
            println!(
                "#{} {} [{}] {} {}",
                item.id, item.label, item.url, last, currency
            );
        }
        Ok(())
    }
}

fn cmd_check(conn: &Connection, args: &CheckArgs, global: &GlobalArgs) -> AppResult<()> {
    let watches = match args.watch.as_deref() {
        Some(query) => vec![resolve_watch(conn, query)?],
        None => list_watches(conn)?,
    };

    let client = build_http_client(args.timeout_secs)?;
    let mut items = Vec::with_capacity(watches.len());

    for watch in watches {
        match check_one_watch(conn, &client, &watch) {
            Ok(item) => items.push(item),
            Err(err) => items.push(CheckItem {
                id: watch.id,
                url: watch.url,
                label: watch.label,
                ok: false,
                price: None,
                currency: None,
                previous_price: watch.last_price,
                dropped: None,
                target_price: watch.target_price,
                target_hit: None,
                checked_at: None,
                error: Some(err.to_string()),
                code: Some(err.code().to_string()),
            }),
        }
    }

    if global.json {
        write_json(&OkList {
            ok: true,
            count: items.len(),
            items,
        })
    } else if global.quiet {
        let success_count = items.iter().filter(|item| item.ok).count();
        println!("{success_count}");
        Ok(())
    } else if items.is_empty() {
        println!("No watches to check.");
        Ok(())
    } else {
        for item in &items {
            if item.ok {
                let price = item.price.unwrap_or_default();
                let currency = item.currency.as_deref().unwrap_or("USD");
                let dropped = item.dropped.unwrap_or(false);
                let marker = if dropped { "drop" } else { "steady" };
                println!(
                    "#{} {} {price:.2} {currency} ({marker})",
                    item.id, item.label
                );
            } else {
                let error = item.error.as_deref().unwrap_or("unknown error");
                println!("#{} {} error: {}", item.id, item.label, error);
            }
        }
        Ok(())
    }
}

fn cmd_delete(conn: &Connection, args: &WatchArg, global: &GlobalArgs) -> AppResult<()> {
    let watch = resolve_watch(conn, &args.watch)?;

    conn.execute("DELETE FROM watches WHERE id = ?1", params![watch.id])
        .map_err(|_| AppError::Database)?;

    let message = "Watch deleted".to_string();

    if global.json {
        write_json(&OkMessage {
            ok: true,
            message,
            id: Some(watch.id),
        })
    } else if global.quiet {
        println!("{}", watch.id);
        Ok(())
    } else {
        println!("Deleted watch #{}: {}", watch.id, watch.label);
        Ok(())
    }
}

fn build_http_client(timeout_secs: u64) -> AppResult<Client> {
    let timeout = if timeout_secs == 0 { 20 } else { timeout_secs };
    Client::builder()
        .timeout(Duration::from_secs(timeout))
        .user_agent("dee-pricewatch/0.1")
        .build()
        .map_err(|err| AppError::RequestFailed(err.to_string()))
}

fn check_one_watch(
    conn: &Connection,
    client: &Client,
    watch: &WatchRecord,
) -> AppResult<CheckItem> {
    let html = fetch_page(client, &watch.url)?;
    let currency_fallback = watch.last_currency.as_deref().unwrap_or("USD").to_string();

    let (price, currency) = extract_price(&html, watch.selector.as_deref(), &currency_fallback)?;
    let checked_at = now_timestamp();
    let previous_price = watch.last_price;
    let dropped = previous_price.map(|previous| price < previous);
    let target_hit = watch.target_price.map(|target| price <= target);

    conn.execute(
        "UPDATE watches
         SET last_price = ?1,
             last_currency = ?2,
             last_checked_at = ?3,
             updated_at = ?4
         WHERE id = ?5",
        params![price, currency, checked_at, checked_at, watch.id],
    )
    .map_err(|_| AppError::Database)?;

    conn.execute(
        "INSERT INTO checks (watch_id, price, currency, checked_at, dropped)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            watch.id,
            price,
            currency,
            checked_at,
            i64::from(dropped.unwrap_or(false))
        ],
    )
    .map_err(|_| AppError::Database)?;

    Ok(CheckItem {
        id: watch.id,
        url: watch.url.clone(),
        label: watch.label.clone(),
        ok: true,
        price: Some(price),
        currency: Some(currency),
        previous_price,
        dropped,
        target_price: watch.target_price,
        target_hit,
        checked_at: Some(checked_at),
        error: None,
        code: None,
    })
}

fn fetch_page(client: &Client, url: &str) -> AppResult<String> {
    let response = client
        .get(url)
        .send()
        .map_err(|err| AppError::RequestFailed(err.to_string()))?;

    if !response.status().is_success() {
        return Err(AppError::RequestFailed(format!(
            "HTTP {} for {url}",
            response.status().as_u16()
        )));
    }

    response
        .text()
        .map_err(|err| AppError::RequestFailed(err.to_string()))
}

fn extract_price(
    html: &str,
    selector: Option<&str>,
    fallback_currency: &str,
) -> AppResult<(f64, String)> {
    let text = match selector {
        Some(raw_selector) => {
            let doc = Html::parse_document(html);
            let parsed_selector = Selector::parse(raw_selector).map_err(|_| {
                AppError::InvalidArgument(format!("invalid selector '{raw_selector}'"))
            })?;
            let selected = doc.select(&parsed_selector).next().ok_or_else(|| {
                AppError::ParseFailed("selector did not match any element".to_string())
            })?;
            selected.text().collect::<Vec<_>>().join(" ")
        }
        None => html.to_string(),
    };

    let symbol_pattern =
        Regex::new(r"(?i)([$€£])\s*([0-9]{1,3}(?:,[0-9]{3})*(?:\.[0-9]{2})|[0-9]+(?:\.[0-9]{2}))")
            .map_err(|_| AppError::ParseFailed("price regex failed to compile".to_string()))?;

    if let Some(caps) = symbol_pattern.captures(&text) {
        let symbol = caps
            .get(1)
            .map(|value| value.as_str())
            .ok_or_else(|| AppError::ParseFailed("currency symbol missing".to_string()))?;
        let raw_number = caps
            .get(2)
            .map(|value| value.as_str())
            .ok_or_else(|| AppError::ParseFailed("price number missing".to_string()))?;
        let price = parse_number(raw_number)?;
        let currency = symbol_to_currency(symbol).to_string();
        return Ok((price, currency));
    }

    let decimal_pattern = Regex::new(r"\b([0-9]{1,3}(?:,[0-9]{3})*\.[0-9]{2})\b")
        .map_err(|_| AppError::ParseFailed("price regex failed to compile".to_string()))?;

    if let Some(caps) = decimal_pattern.captures(&text) {
        let raw_number = caps
            .get(1)
            .map(|value| value.as_str())
            .ok_or_else(|| AppError::ParseFailed("price number missing".to_string()))?;
        let price = parse_number(raw_number)?;
        let currency = normalize_currency(fallback_currency)?;
        return Ok((price, currency));
    }

    Err(AppError::ParseFailed(
        "no recognizable price token found".to_string(),
    ))
}

fn parse_number(raw: &str) -> AppResult<f64> {
    let normalized = raw.replace(',', "");
    let value = normalized
        .parse::<f64>()
        .map_err(|_| AppError::ParseFailed(format!("invalid numeric price '{raw}'")))?;

    if value <= 0.0 {
        return Err(AppError::ParseFailed("price must be positive".to_string()));
    }

    Ok(value)
}

fn symbol_to_currency(symbol: &str) -> &'static str {
    match symbol {
        "$" => "USD",
        "€" => "EUR",
        "£" => "GBP",
        _ => "USD",
    }
}

fn validate_url(url: &str) -> AppResult<()> {
    let parsed =
        Url::parse(url).map_err(|_| AppError::InvalidArgument(format!("invalid url '{url}'")))?;

    match parsed.scheme() {
        "http" | "https" => Ok(()),
        _ => Err(AppError::InvalidArgument(
            "url must use http or https".to_string(),
        )),
    }
}

fn validate_positive_price(value: Option<f64>, name: &str) -> AppResult<()> {
    if let Some(price) = value {
        if price <= 0.0 {
            return Err(AppError::InvalidArgument(format!("{name} must be > 0")));
        }
    }
    Ok(())
}

fn normalize_currency(raw: &str) -> AppResult<String> {
    let value = raw.trim().to_ascii_uppercase();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(
            "currency cannot be empty".to_string(),
        ));
    }
    if value.len() > 8 {
        return Err(AppError::InvalidArgument(
            "currency is too long".to_string(),
        ));
    }
    Ok(value)
}

fn derive_label(explicit: Option<&str>, url: &str) -> AppResult<String> {
    if let Some(value) = explicit {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let parsed =
        Url::parse(url).map_err(|_| AppError::InvalidArgument(format!("invalid url '{url}'")))?;

    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::InvalidArgument("url host missing".to_string()))?;

    Ok(host.to_string())
}

fn resolve_watch(conn: &Connection, query: &str) -> AppResult<WatchRecord> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(
            "watch must be an id or label".to_string(),
        ));
    }

    if let Ok(id) = trimmed.parse::<i64>() {
        if let Some(record) = query_watch_by_id(conn, id)? {
            return Ok(record);
        }
    }

    if let Some(record) = query_watch_exact_label(conn, trimmed)? {
        return Ok(record);
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, url, label, target_price, selector, last_price, last_currency,
                    last_checked_at, created_at, updated_at
             FROM watches
             WHERE lower(label) LIKE '%' || lower(?1) || '%'
             ORDER BY id
             LIMIT 2",
        )
        .map_err(|_| AppError::Database)?;

    let rows = stmt
        .query_map(params![trimmed], map_watch_row)
        .map_err(|_| AppError::Database)?;

    let matches = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::Database)?;

    match matches.len() {
        0 => Err(AppError::NotFound),
        1 => Ok(matches[0].clone()),
        _ => Err(AppError::InvalidArgument(
            "watch query matched multiple labels; use id or exact label".to_string(),
        )),
    }
}

fn query_watch_by_id(conn: &Connection, id: i64) -> AppResult<Option<WatchRecord>> {
    conn.query_row(
        "SELECT id, url, label, target_price, selector, last_price, last_currency,
                last_checked_at, created_at, updated_at
         FROM watches WHERE id = ?1",
        params![id],
        map_watch_row,
    )
    .optional()
    .map_err(|_| AppError::Database)
}

fn query_watch_exact_label(conn: &Connection, label: &str) -> AppResult<Option<WatchRecord>> {
    conn.query_row(
        "SELECT id, url, label, target_price, selector, last_price, last_currency,
                last_checked_at, created_at, updated_at
         FROM watches WHERE lower(label) = lower(?1)",
        params![label],
        map_watch_row,
    )
    .optional()
    .map_err(|_| AppError::Database)
}

fn list_watches(conn: &Connection) -> AppResult<Vec<WatchRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, url, label, target_price, selector, last_price, last_currency,
                    last_checked_at, created_at, updated_at
             FROM watches ORDER BY id",
        )
        .map_err(|_| AppError::Database)?;

    let rows = stmt
        .query_map([], map_watch_row)
        .map_err(|_| AppError::Database)?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::Database)
}

fn map_watch_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WatchRecord> {
    Ok(WatchRecord {
        id: row.get(0)?,
        url: row.get(1)?,
        label: row.get(2)?,
        target_price: row.get(3)?,
        selector: row.get(4)?,
        last_price: row.get(5)?,
        last_currency: row.get(6)?,
        last_checked_at: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn db_path() -> AppResult<PathBuf> {
    let base = dirs::data_dir().ok_or(AppError::DataDirMissing)?;
    Ok(base.join("dee-pricewatch").join("pricewatch.db"))
}

fn initialize_db(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS watches (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             url TEXT NOT NULL,
             label TEXT NOT NULL,
             target_price REAL,
             selector TEXT,
             last_price REAL,
             last_currency TEXT,
             last_checked_at TEXT,
             created_at TEXT NOT NULL,
             updated_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS checks (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             watch_id INTEGER NOT NULL,
             price REAL NOT NULL,
             currency TEXT NOT NULL,
             checked_at TEXT NOT NULL,
             dropped INTEGER NOT NULL,
             FOREIGN KEY(watch_id) REFERENCES watches(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_checks_watch_id_checked_at
             ON checks(watch_id, checked_at DESC);",
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
