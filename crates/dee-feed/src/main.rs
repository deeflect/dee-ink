use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand, ValueEnum};
use feed_rs::parser;
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{Migrations, M};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

const TOOL: &str = "dee-feed";

#[derive(Parser, Debug)]
#[command(name = "dee-feed")]
#[command(version)]
#[command(about = "dee-feed - RSS/Atom feed reader CLI")]
#[command(
    after_help = "EXAMPLES:\n  dee-feed add https://example.com/feed.xml --name \"Example\"\n  dee-feed list --json\n  dee-feed fetch --limit 20 --json\n  dee-feed read 1 --json\n  dee-feed export --format opml"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Add(AddArgs),
    List(GlobalFlags),
    Remove(RemoveArgs),
    Fetch(FetchArgs),
    Read(ReadArgs),
    MarkRead(MarkReadArgs),
    Export(ExportArgs),
    Import(ImportArgs),
    Config(ConfigArgs),
}

#[derive(Args, Debug, Clone)]
struct GlobalFlags {
    #[arg(short = 'j', long)]
    json: bool,
    #[arg(short = 'q', long)]
    quiet: bool,
    #[arg(short = 'v', long)]
    verbose: bool,
}

#[derive(Args, Debug)]
struct AddArgs {
    url: String,
    #[arg(long)]
    name: Option<String>,
    #[command(flatten)]
    flags: GlobalFlags,
}

#[derive(Args, Debug)]
struct RemoveArgs {
    name_or_id: String,
    #[command(flatten)]
    flags: GlobalFlags,
}

#[derive(Args, Debug)]
struct FetchArgs {
    name_or_id: Option<String>,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long)]
    unread: bool,
    #[command(flatten)]
    flags: GlobalFlags,
}

#[derive(Args, Debug)]
struct ReadArgs {
    item_id: i64,
    #[command(flatten)]
    flags: GlobalFlags,
}

#[derive(Args, Debug)]
struct MarkReadArgs {
    name_or_id: String,
    #[arg(long, default_value_t = false)]
    all: bool,
    #[command(flatten)]
    flags: GlobalFlags,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum ExportFormat {
    Opml,
    Json,
}

#[derive(Args, Debug)]
struct ExportArgs {
    #[arg(long, value_enum, default_value_t = ExportFormat::Json)]
    format: ExportFormat,
    #[command(flatten)]
    flags: GlobalFlags,
}

#[derive(Args, Debug)]
struct ImportArgs {
    file: PathBuf,
    #[command(flatten)]
    flags: GlobalFlags,
}

#[derive(Args, Debug)]
struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Subcommand, Debug)]
enum ConfigCommand {
    Show(GlobalFlags),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct FeedDef {
    id: i64,
    name: String,
    url: String,
    created_at: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct FeedConfig {
    feeds: Vec<FeedDef>,
}

#[derive(Serialize, Debug)]
struct FeedItem {
    id: i64,
    feed: String,
    title: String,
    url: String,
    published: String,
    read: bool,
    summary: String,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        let json_mode = std::env::args().any(|arg| arg == "--json" || arg == "-j");
        if json_mode {
            println!(
                "{}",
                json!({"ok": false, "error": err.to_string(), "code": "RUNTIME_ERROR"})
            );
        } else {
            eprintln!("error: {err}");
        }
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut cfg = load_feeds()?;
    let mut conn = open_db()?;

    match cli.command {
        Commands::Add(args) => cmd_add(&mut cfg, args),
        Commands::List(flags) => cmd_list(&cfg, flags),
        Commands::Remove(args) => cmd_remove(&mut cfg, args),
        Commands::Fetch(args) => cmd_fetch(&cfg, &mut conn, args).await,
        Commands::Read(args) => cmd_read(&cfg, &mut conn, args),
        Commands::MarkRead(args) => cmd_mark_read(&cfg, &mut conn, args),
        Commands::Export(args) => cmd_export(&cfg, args),
        Commands::Import(args) => cmd_import(&mut cfg, args),
        Commands::Config(args) => cmd_config(args),
    }
}

fn cmd_add(cfg: &mut FeedConfig, args: AddArgs) -> Result<()> {
    let next_id = cfg.feeds.iter().map(|f| f.id).max().unwrap_or(0) + 1;
    if cfg.feeds.iter().any(|f| f.url == args.url) {
        return Err(anyhow!("Feed already exists: {}", args.url));
    }
    let name = args.name.unwrap_or_else(|| format!("feed-{}", next_id));
    let item = FeedDef {
        id: next_id,
        name,
        url: args.url,
        created_at: Utc::now().to_rfc3339(),
    };
    cfg.feeds.push(item.clone());
    save_feeds(cfg)?;
    output_q(
        &args.flags,
        json!({"ok": true, "message": "Feed added", "id": item.id, "item": item}),
        &format!("Added feed #{}", next_id),
        &format!("{}", next_id),
    );
    Ok(())
}

fn cmd_list(cfg: &FeedConfig, flags: GlobalFlags) -> Result<()> {
    if flags.json {
        println!(
            "{}",
            json!({"ok": true, "count": cfg.feeds.len(), "items": cfg.feeds})
        );
    } else if flags.quiet {
        for f in &cfg.feeds {
            println!("{}", f.id);
        }
    } else {
        println!("{} feeds", cfg.feeds.len());
        for f in &cfg.feeds {
            println!("  {} {} ({})", f.id, f.name, f.url);
        }
    }
    Ok(())
}

fn cmd_remove(cfg: &mut FeedConfig, args: RemoveArgs) -> Result<()> {
    let found = resolve_feed(cfg, &args.name_or_id)?;
    cfg.feeds.retain(|f| f.id != found.id);
    save_feeds(cfg)?;
    output_q(
        &args.flags,
        json!({"ok": true, "message": "Feed removed", "id": found.id}),
        &format!("Removed {}", found.name),
        &format!("{}", found.id),
    );
    Ok(())
}

async fn cmd_fetch(cfg: &FeedConfig, conn: &mut Connection, args: FetchArgs) -> Result<()> {
    let scoped_feed_id: Option<i64>;
    let chosen = if let Some(target) = args.name_or_id.as_deref() {
        let feed = resolve_feed(cfg, target)?;
        scoped_feed_id = Some(feed.id);
        vec![feed]
    } else {
        scoped_feed_id = None;
        cfg.feeds.clone()
    };

    // Sync cache before inserts so JOIN works correctly
    sync_feeds_cache(conn, cfg)?;

    let client = reqwest::Client::new();
    for feed in &chosen {
        match fetch_and_store_feed(&client, conn, feed).await {
            Ok(()) => {}
            Err(e) => {
                if args.flags.verbose {
                    eprintln!("warning: feed {} failed: {e}", feed.url);
                }
                // isolation: continue with remaining feeds
            }
        }
    }

    // Build query with optional feed_id and unread scopes
    let mut conditions = Vec::new();
    if args.unread {
        conditions.push("i.read = 0".to_string());
    }
    if let Some(fid) = scoped_feed_id {
        conditions.push(format!("i.feed_id = {fid}"));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT i.id, f.name, i.title, i.url, i.published, i.read, i.summary \
         FROM items i JOIN feeds_cache f ON f.id=i.feed_id{where_clause} \
         ORDER BY i.published DESC LIMIT ?1"
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![args.limit as i64], |row| {
        Ok(FeedItem {
            id: row.get(0)?,
            feed: row.get(1)?,
            title: row.get(2)?,
            url: row.get(3)?,
            published: normalize_iso(row.get::<_, String>(4)?),
            read: row.get::<_, i64>(5)? == 1,
            summary: row.get(6)?,
        })
    })?;
    let items: Vec<FeedItem> = rows.collect::<rusqlite::Result<Vec<_>>>()?;

    if args.flags.json {
        println!(
            "{}",
            json!({"ok": true, "count": items.len(), "items": items})
        );
    } else if args.flags.quiet {
        for item in &items {
            println!("{}", item.id);
        }
    } else {
        println!("Fetched {} items", items.len());
        for item in &items {
            println!("  [{}] {} ({})", item.id, item.title, item.published);
        }
    }
    Ok(())
}

async fn fetch_and_store_feed(
    client: &reqwest::Client,
    conn: &mut Connection,
    feed: &FeedDef,
) -> Result<()> {
    let body = client
        .get(&feed.url)
        .send()
        .await
        .with_context(|| format!("Failed fetching {}", feed.url))?
        .error_for_status()
        .with_context(|| format!("Bad status from {}", feed.url))?
        .bytes()
        .await
        .context("Failed reading response body")?;

    let parsed =
        parser::parse(&body[..]).with_context(|| format!("Invalid feed XML: {}", feed.url))?;

    for entry in parsed.entries {
        let ext_id = entry.id;
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_else(|| "Untitled".to_string());
        let link = entry
            .links
            .first()
            .map(|l| l.href.clone())
            .unwrap_or_default();
        let summary = entry
            .summary
            .as_ref()
            .map(|s| s.content.clone())
            .unwrap_or_default();
        let published = entry
            .published
            .or(entry.updated)
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        conn.execute(
            "INSERT OR IGNORE INTO items (feed_id, ext_id, title, url, summary, published, read) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            params![feed.id, ext_id, title, link, summary, published],
        )?;
    }
    Ok(())
}

fn cmd_read(cfg: &FeedConfig, conn: &mut Connection, args: ReadArgs) -> Result<()> {
    sync_feeds_cache(conn, cfg)?;
    let mut stmt = conn.prepare(
        "SELECT i.id, COALESCE(f.name, ''), i.title, i.url, i.published, i.read, i.summary \
         FROM items i LEFT JOIN feeds_cache f ON f.id=i.feed_id WHERE i.id=?1",
    )?;
    let item: Option<FeedItem> = stmt
        .query_row(params![args.item_id], |row| {
            Ok(FeedItem {
                id: row.get(0)?,
                feed: row.get(1)?,
                title: row.get(2)?,
                url: row.get(3)?,
                published: normalize_iso(row.get::<_, String>(4)?),
                read: row.get::<_, i64>(5)? == 1,
                summary: row.get(6)?,
            })
        })
        .optional()?;

    let mut item = item.ok_or_else(|| anyhow!("Item not found: {}", args.item_id))?;
    conn.execute("UPDATE items SET read=1 WHERE id=?1", params![args.item_id])?;
    item.read = true;

    output(
        &args.flags,
        json!({"ok": true, "item": item}),
        format!("{}", args.item_id),
    );
    Ok(())
}

fn cmd_mark_read(cfg: &FeedConfig, conn: &mut Connection, args: MarkReadArgs) -> Result<()> {
    if !args.all {
        return Err(anyhow!("Missing required argument: --all"));
    }
    let feed = resolve_feed(cfg, &args.name_or_id)?;
    let count = conn.execute("UPDATE items SET read=1 WHERE feed_id=?1", params![feed.id])?;
    output_q(
        &args.flags,
        json!({"ok": true, "message": "Marked items read", "count": count}),
        &format!("Marked {} as read", count),
        &format!("{}", count),
    );
    Ok(())
}

fn cmd_export(cfg: &FeedConfig, args: ExportArgs) -> Result<()> {
    match args.format {
        ExportFormat::Json => {
            output(
                &args.flags,
                json!({"ok": true, "count": cfg.feeds.len(), "items": cfg.feeds}),
                "Exported feeds".to_string(),
            );
        }
        ExportFormat::Opml => {
            let body = cfg
                .feeds
                .iter()
                .map(|f| {
                    format!(
                        "    <outline text=\"{}\" title=\"{}\" type=\"rss\" xmlUrl=\"{}\" />",
                        xml_escape(&f.name),
                        xml_escape(&f.name),
                        xml_escape(&f.url)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            let opml = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\">\n  <head><title>dee-feed export</title></head>\n  <body>\n{}\n  </body>\n</opml>", body);
            if args.flags.json {
                println!(
                    "{}",
                    json!({"ok": true, "count": cfg.feeds.len(), "opml": opml})
                );
            } else {
                println!("{opml}");
            }
        }
    }
    Ok(())
}

fn cmd_import(cfg: &mut FeedConfig, args: ImportArgs) -> Result<()> {
    let data = fs::read_to_string(&args.file)
        .with_context(|| format!("Could not read file {}", args.file.display()))?;
    let mut existing: HashSet<String> = cfg.feeds.iter().map(|f| f.url.clone()).collect();
    let mut added = 0_i64;
    for line in data.lines() {
        if let Some(url) = parse_attr(line, "xmlUrl") {
            if existing.contains(&url) {
                continue;
            }
            let next_id = cfg.feeds.iter().map(|f| f.id).max().unwrap_or(0) + 1;
            let name = parse_attr(line, "title")
                .or_else(|| parse_attr(line, "text"))
                .unwrap_or_else(|| format!("feed-{}", next_id));
            cfg.feeds.push(FeedDef {
                id: next_id,
                name,
                url: url.clone(),
                created_at: Utc::now().to_rfc3339(),
            });
            existing.insert(url);
            added += 1;
        }
    }
    save_feeds(cfg)?;
    output(
        &args.flags,
        json!({"ok": true, "message": "Import complete", "count": added}),
        format!("Imported {} feeds", added),
    );
    Ok(())
}

fn cmd_config(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Show(flags) => {
            let cfg_path = config_path();
            if !cfg_path.exists() {
                ensure_dirs()?;
                fs::write(&cfg_path, "[general]\ndefault_format = \"table\"\n")?;
            }
            let content = fs::read_to_string(&cfg_path)?;
            if flags.json {
                println!(
                    "{}",
                    json!({"ok": true, "item": {"path": cfg_path.display().to_string(), "content": content}})
                );
            } else {
                println!("{}", content.trim_end());
            }
            Ok(())
        }
    }
}

fn resolve_feed(cfg: &FeedConfig, name_or_id: &str) -> Result<FeedDef> {
    if let Ok(id) = name_or_id.parse::<i64>() {
        if let Some(found) = cfg.feeds.iter().find(|f| f.id == id) {
            return Ok(found.clone());
        }
    }
    let needle = name_or_id.to_lowercase();
    let exact = cfg
        .feeds
        .iter()
        .find(|f| f.name.to_lowercase() == needle)
        .cloned();
    if let Some(found) = exact {
        return Ok(found);
    }
    let fuzzy = cfg
        .feeds
        .iter()
        .find(|f| f.name.to_lowercase().contains(&needle))
        .cloned();
    fuzzy.ok_or_else(|| anyhow!("Feed not found: {name_or_id}"))
}

fn output(flags: &GlobalFlags, payload: Value, text: String) {
    output_q(flags, payload, &text, &text);
}

fn output_q(flags: &GlobalFlags, payload: Value, text: &str, quiet_text: &str) {
    if flags.json {
        println!("{payload}");
    } else if flags.quiet {
        println!("{quiet_text}");
    } else {
        println!("{text}");
    }
    if flags.verbose {
        eprintln!("debug: completed");
    }
}

fn ensure_dirs() -> Result<()> {
    let cfg_parent = config_dir()?;
    let data_parent = data_dir()?;
    fs::create_dir_all(cfg_parent)?;
    fs::create_dir_all(data_parent)?;
    Ok(())
}

fn config_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|p| p.join(TOOL))
        .ok_or_else(|| anyhow!("Could not resolve config directory"))
}

fn data_dir() -> Result<PathBuf> {
    dirs::data_dir()
        .map(|p| p.join(TOOL))
        .ok_or_else(|| anyhow!("Could not resolve data directory"))
}

fn feeds_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("feeds.toml"))
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(TOOL)
        .join("config.toml")
}

fn db_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("feed.db"))
}

fn load_feeds() -> Result<FeedConfig> {
    ensure_dirs()?;
    let path = feeds_path()?;
    if !path.exists() {
        return Ok(FeedConfig::default());
    }
    let content = fs::read_to_string(path)?;
    let parsed: FeedConfig = toml::from_str(&content)?;
    Ok(parsed)
}

fn save_feeds(cfg: &FeedConfig) -> Result<()> {
    ensure_dirs()?;
    let path = feeds_path()?;
    let toml_data = toml::to_string_pretty(cfg)?;
    fs::write(path, toml_data)?;
    Ok(())
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("../migrations/001_initial.sql"))])
}

fn open_db() -> Result<Connection> {
    ensure_dirs()?;
    let path = db_path()?;
    let mut conn = Connection::open(path)?;
    migrations().to_latest(&mut conn)?;
    Ok(conn)
}

fn sync_feeds_cache(conn: &mut Connection, cfg: &FeedConfig) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM feeds_cache", [])?;
    for f in &cfg.feeds {
        tx.execute(
            "INSERT INTO feeds_cache (id, name, url) VALUES (?1, ?2, ?3)",
            params![f.id, f.name, f.url],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn parse_attr(line: &str, name: &str) -> Option<String> {
    let token = format!("{name}=\"");
    let start = line.find(&token)? + token.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn normalize_iso(input: String) -> String {
    DateTime::parse_from_rfc3339(&input)
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339())
        .unwrap_or(input)
}

fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
}
