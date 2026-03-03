use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use chrono::{SecondsFormat, TimeZone, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use futures::future::join_all;
use reqwest::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "dee-mentions",
    version,
    about = "Brand mention monitor CLI",
    long_about = "dee-mentions - Search mentions across public sources and manage watch queries.",
    after_help = "EXAMPLES:\n  dee-mentions check dee.ink --sources hn,reddit --limit 5 --json\n  dee-mentions watch add dee.ink --tag brand --json\n  dee-mentions watch list --json\n  dee-mentions run --all --limit 3 --json\n  dee-mentions watch remove 1 --json"
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
    /// Check mentions for one query
    Check(CheckArgs),
    /// Run checks for saved watch queries
    Run(RunArgs),
    /// Manage watch queries
    Watch(WatchArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Hash)]
enum Source {
    Hn,
    Reddit,
}

#[derive(Debug, Args)]
struct CheckArgs {
    query: String,

    /// Sources to query
    #[arg(long, value_delimiter = ',', default_value = "hn,reddit")]
    sources: Vec<Source>,

    /// Max mentions per source
    #[arg(long, default_value_t = 10)]
    limit: usize,
}

#[derive(Debug, Args)]
struct RunArgs {
    /// Run one watch by id
    #[arg(long)]
    id: Option<i64>,

    /// Run all watches
    #[arg(long)]
    all: bool,

    /// Override default per-watch source list
    #[arg(long, value_delimiter = ',')]
    sources: Vec<Source>,

    /// Max mentions per source
    #[arg(long, default_value_t = 10)]
    limit: usize,
}

#[derive(Debug, Args)]
struct WatchArgs {
    #[command(subcommand)]
    command: WatchCommand,
}

#[derive(Debug, Subcommand)]
enum WatchCommand {
    /// Add watch query
    Add(WatchAddArgs),
    /// List watch queries
    List,
    /// Remove watch query by id
    Remove(WatchRemoveArgs),
}

#[derive(Debug, Args)]
struct WatchAddArgs {
    query: String,

    #[arg(long)]
    tag: Option<String>,

    #[arg(long, value_delimiter = ',', default_value = "hn,reddit")]
    sources: Vec<Source>,
}

#[derive(Debug, Args)]
struct WatchRemoveArgs {
    id: i64,
}

#[derive(Debug, Clone, Serialize)]
struct MentionItem {
    source: String,
    query: String,
    title: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
struct WatchItem {
    id: i64,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    sources: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct WatchItemOut {
    id: i64,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    sources: Vec<String>,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    ok: bool,
    count: usize,
    items: Vec<T>,
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

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Data directory not found")]
    DataDirMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Watch not found")]
    NotFound,
    #[error("All source requests failed")]
    RequestFailed,
    #[error("Database operation failed")]
    Database,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::DataDirMissing => "CONFIG_MISSING",
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::NotFound => "NOT_FOUND",
            Self::RequestFailed => "REQUEST_FAILED",
            Self::Database => "DATABASE_ERROR",
        }
    }
}

#[derive(Debug, Deserialize)]
struct HnSearchResponse {
    #[serde(default)]
    hits: Vec<HnHit>,
}

#[derive(Debug, Deserialize)]
struct HnHit {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    story_url: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    points: Option<i64>,
    #[serde(default)]
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RedditRoot {
    data: RedditData,
}

#[derive(Debug, Deserialize)]
struct RedditData {
    #[serde(default)]
    children: Vec<RedditChild>,
}

#[derive(Debug, Deserialize)]
struct RedditChild {
    data: RedditPost,
}

#[derive(Debug, Deserialize)]
struct RedditPost {
    #[serde(default)]
    title: String,
    #[serde(default)]
    permalink: String,
    #[serde(default)]
    selftext: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    score: i64,
    #[serde(default)]
    created_utc: f64,
}

#[derive(Debug)]
struct SourceFailure;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let cli = parse_cli();

    if let Err(err) = run(&cli).await {
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

async fn run(cli: &Cli) -> Result<(), AppError> {
    validate_args(&cli.command)?;

    let db_path = db_path()?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|_| AppError::Database)?;
    }

    if cli.global.verbose {
        eprintln!("[dee-mentions] db_path={}", db_path.display());
    }

    let conn = Connection::open(db_path).map_err(|_| AppError::Database)?;
    init_db(&conn)?;

    match &cli.command {
        Commands::Check(args) => {
            let items =
                fetch_mentions(&args.query, &args.sources, args.limit, cli.global.verbose).await?;
            print_mentions(items, &cli.global)
        }
        Commands::Run(args) => {
            let watches = select_watches(&conn, args)?;
            let mut items = Vec::new();

            for watch in watches {
                let source_list = if args.sources.is_empty() {
                    parse_sources_csv(&watch.sources)
                } else {
                    args.sources.clone()
                };

                let mut found =
                    fetch_mentions(&watch.query, &source_list, args.limit, cli.global.verbose)
                        .await?;
                items.append(&mut found);
            }

            items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            print_mentions(items, &cli.global)
        }
        Commands::Watch(args) => match &args.command {
            WatchCommand::Add(cmd) => {
                let source_csv = sources_to_csv(&cmd.sources);
                conn.execute(
                    "INSERT INTO watches (query, tag, sources, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![cmd.query, cmd.tag.as_deref(), source_csv, now_iso()],
                )
                .map_err(|_| AppError::Database)?;
                print_action("Watch added", Some(conn.last_insert_rowid()), &cli.global)
            }
            WatchCommand::List => {
                let mut stmt = conn
                    .prepare("SELECT id,query,tag,sources,created_at FROM watches ORDER BY id ASC")
                    .map_err(|_| AppError::Database)?;
                let rows = stmt
                    .query_map([], |row| {
                        let sources_raw: String = row.get(3)?;
                        Ok(WatchItem {
                            id: row.get(0)?,
                            query: row.get(1)?,
                            tag: row.get(2)?,
                            sources: sources_raw,
                            created_at: row.get(4)?,
                        })
                    })
                    .map_err(|_| AppError::Database)?;

                let mut items = Vec::new();
                for row in rows {
                    items.push(row.map_err(|_| AppError::Database)?);
                }

                if cli.global.json {
                    let out_items: Vec<WatchItemOut> = items
                        .iter()
                        .map(|item| WatchItemOut {
                            id: item.id,
                            query: item.query.clone(),
                            tag: item.tag.clone(),
                            sources: parse_sources_csv(&item.sources)
                                .into_iter()
                                .map(source_to_string)
                                .map(str::to_string)
                                .collect(),
                            created_at: item.created_at.clone(),
                        })
                        .collect();
                    print_json(&ListResponse {
                        ok: true,
                        count: out_items.len(),
                        items: out_items,
                    });
                } else if cli.global.quiet {
                    println!("{}", items.len());
                } else if items.is_empty() {
                    println!("no watches found");
                } else {
                    for item in items {
                        println!("#{} {} [{}]", item.id, item.query, item.sources);
                    }
                }
                Ok(())
            }
            WatchCommand::Remove(cmd) => {
                let deleted = conn
                    .execute("DELETE FROM watches WHERE id = ?1", params![cmd.id])
                    .map_err(|_| AppError::Database)?;

                if deleted == 0 {
                    return Err(AppError::NotFound);
                }

                print_action("Watch removed", Some(cmd.id), &cli.global)
            }
        },
    }
}

fn validate_args(command: &Commands) -> Result<(), AppError> {
    match command {
        Commands::Check(args) => {
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
        Commands::Run(args) => {
            if !args.all && args.id.is_none() {
                return Err(AppError::InvalidArgument(
                    "use --all or --id <watch-id>".to_string(),
                ));
            }
            if args.limit == 0 {
                return Err(AppError::InvalidArgument(
                    "limit must be greater than 0".to_string(),
                ));
            }
            Ok(())
        }
        Commands::Watch(args) => match &args.command {
            WatchCommand::Add(cmd) => {
                if cmd.query.trim().is_empty() {
                    return Err(AppError::InvalidArgument(
                        "query must not be empty".to_string(),
                    ));
                }
                if cmd.sources.is_empty() {
                    return Err(AppError::InvalidArgument(
                        "at least one source is required".to_string(),
                    ));
                }
                Ok(())
            }
            WatchCommand::List => Ok(()),
            WatchCommand::Remove(cmd) => {
                if cmd.id <= 0 {
                    return Err(AppError::InvalidArgument(
                        "id must be greater than 0".to_string(),
                    ));
                }
                Ok(())
            }
        },
    }
}

fn db_path() -> Result<PathBuf, AppError> {
    let base = dirs::data_dir().ok_or(AppError::DataDirMissing)?;
    Ok(base.join("dee-mentions").join("mentions.db"))
}

fn init_db(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS watches (
          id         INTEGER PRIMARY KEY AUTOINCREMENT,
          query      TEXT    NOT NULL,
          tag        TEXT,
          sources    TEXT    NOT NULL,
          created_at TEXT    NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_watches_query ON watches(query);
        "#,
    )
    .map_err(|_| AppError::Database)
}

fn select_watches(conn: &Connection, args: &RunArgs) -> Result<Vec<WatchItem>, AppError> {
    let mut items = Vec::new();

    if args.all {
        let mut stmt = conn
            .prepare("SELECT id,query,tag,sources,created_at FROM watches ORDER BY id ASC")
            .map_err(|_| AppError::Database)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(WatchItem {
                    id: row.get(0)?,
                    query: row.get(1)?,
                    tag: row.get(2)?,
                    sources: row.get::<_, String>(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|_| AppError::Database)?;

        for row in rows {
            items.push(row.map_err(|_| AppError::Database)?);
        }
    } else if let Some(id) = args.id {
        let mut stmt = conn
            .prepare("SELECT id,query,tag,sources,created_at FROM watches WHERE id = ?1")
            .map_err(|_| AppError::Database)?;
        let found = stmt
            .query_row(params![id], |row| {
                Ok(WatchItem {
                    id: row.get(0)?,
                    query: row.get(1)?,
                    tag: row.get(2)?,
                    sources: row.get::<_, String>(3)?,
                    created_at: row.get(4)?,
                })
            })
            .optional()
            .map_err(|_| AppError::Database)?;

        if let Some(item) = found {
            items.push(item);
        }
    }

    if items.is_empty() {
        return Err(AppError::NotFound);
    }

    Ok(items)
}

async fn fetch_mentions(
    query: &str,
    sources: &[Source],
    limit: usize,
    verbose: bool,
) -> Result<Vec<MentionItem>, AppError> {
    let client = Client::builder()
        .user_agent("dee-mentions/0.1")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| AppError::RequestFailed)?;

    let unique_sources: HashSet<Source> = sources.iter().copied().collect();
    if unique_sources.is_empty() {
        return Err(AppError::InvalidArgument(
            "at least one source is required".to_string(),
        ));
    }

    let mut tasks = Vec::new();

    for source in unique_sources {
        let c = client.clone();
        let q = query.to_string();
        let task = async move {
            match source {
                Source::Hn => fetch_hn_mentions(&c, &q, limit).await,
                Source::Reddit => fetch_reddit_mentions(&c, &q, limit).await,
            }
        };
        tasks.push(task);
    }

    let results = join_all(tasks).await;

    let mut all_mentions = Vec::new();
    let mut success = 0usize;

    for result in results {
        match result {
            Ok(mut items) => {
                success += 1;
                all_mentions.append(&mut items);
            }
            Err(_) => {
                if verbose {
                    eprintln!("[dee-mentions] source request failed");
                }
            }
        }
    }

    if success == 0 {
        return Err(AppError::RequestFailed);
    }

    all_mentions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(all_mentions)
}

async fn fetch_hn_mentions(
    client: &Client,
    query: &str,
    limit: usize,
) -> Result<Vec<MentionItem>, SourceFailure> {
    let base = std::env::var("DEE_MENTIONS_HN_BASE")
        .unwrap_or_else(|_| "https://hn.algolia.com".to_string());
    let url = format!(
        "{}/api/v1/search?query={}&tags=story&hitsPerPage={}",
        base,
        urlencoding::encode(query),
        limit
    );

    let response = client.get(url).send().await.map_err(|_| SourceFailure)?;
    if !response.status().is_success() {
        return Err(SourceFailure);
    }

    let payload: HnSearchResponse = response.json().await.map_err(|_| SourceFailure)?;

    let mut items = Vec::new();
    for hit in payload.hits {
        let title = hit.title.unwrap_or_default();
        if title.is_empty() {
            continue;
        }
        let url = hit.url.or(hit.story_url).unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        items.push(MentionItem {
            source: "hn".to_string(),
            query: query.to_string(),
            title,
            url,
            snippet: None,
            author: hit.author,
            created_at: hit.created_at.unwrap_or_else(now_iso),
            score: hit.points,
        });
    }

    Ok(items)
}

async fn fetch_reddit_mentions(
    client: &Client,
    query: &str,
    limit: usize,
) -> Result<Vec<MentionItem>, SourceFailure> {
    let base = std::env::var("DEE_MENTIONS_REDDIT_BASE")
        .unwrap_or_else(|_| "https://www.reddit.com".to_string());
    let url = format!(
        "{}/search.json?q={}&limit={}&sort=new&t=week",
        base,
        urlencoding::encode(query),
        limit
    );

    let response = client
        .get(url)
        .header("User-Agent", "dee-mentions/0.1")
        .send()
        .await
        .map_err(|_| SourceFailure)?;
    if !response.status().is_success() {
        return Err(SourceFailure);
    }

    let payload: RedditRoot = response.json().await.map_err(|_| SourceFailure)?;

    let mut items = Vec::new();
    for child in payload.data.children {
        let post = child.data;
        if post.title.is_empty() {
            continue;
        }
        let url = format!("https://reddit.com{}", post.permalink);
        let created = created_utc_to_rfc3339(post.created_utc);

        items.push(MentionItem {
            source: "reddit".to_string(),
            query: query.to_string(),
            title: post.title,
            url,
            snippet: if post.selftext.is_empty() {
                None
            } else {
                Some(post.selftext.chars().take(240).collect())
            },
            author: if post.author.is_empty() {
                None
            } else {
                Some(post.author)
            },
            created_at: created,
            score: Some(post.score),
        });
    }

    Ok(items)
}

fn created_utc_to_rfc3339(ts: f64) -> String {
    let sec = ts as i64;
    Utc.timestamp_opt(sec, 0)
        .single()
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_else(now_iso)
}

fn source_to_string(source: Source) -> &'static str {
    match source {
        Source::Hn => "hn",
        Source::Reddit => "reddit",
    }
}

fn sources_to_csv(sources: &[Source]) -> String {
    let mut uniq = HashSet::new();
    let mut out = Vec::new();
    for source in sources {
        if uniq.insert(*source) {
            out.push(source_to_string(*source).to_string());
        }
    }
    out.join(",")
}

fn parse_sources_csv(raw: &str) -> Vec<Source> {
    raw.split(',')
        .map(|s| s.trim().to_lowercase())
        .filter_map(|s| match s.as_str() {
            "hn" => Some(Source::Hn),
            "reddit" => Some(Source::Reddit),
            _ => None,
        })
        .collect()
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn print_mentions(items: Vec<MentionItem>, global: &GlobalFlags) -> Result<(), AppError> {
    if global.json {
        print_json(&ListResponse {
            ok: true,
            count: items.len(),
            items,
        });
    } else if global.quiet {
        println!("{}", items.len());
    } else if items.is_empty() {
        println!("no mentions found");
    } else {
        for item in items {
            println!("[{}] {}", item.source, item.title);
        }
    }
    Ok(())
}

fn print_action(message: &str, id: Option<i64>, global: &GlobalFlags) -> Result<(), AppError> {
    if global.json {
        print_json(&ActionResponse {
            ok: true,
            message: message.to_string(),
            id,
        });
    } else if global.quiet {
        println!("{}", id.unwrap_or(1));
    } else {
        println!("{message}");
    }
    Ok(())
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
