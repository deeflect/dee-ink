use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

const EVENTBRITE_BASE: &str = "https://www.eventbriteapi.com/v3";

#[derive(Debug, Parser)]
#[command(
    name = "dee-events",
    version,
    about = "Local events search CLI",
    after_help = "EXAMPLES:\n  dee-events search \"San Francisco\" --query tech --limit 10 --json\n  dee-events show 1234567890 --json\n  dee-events config set eventbrite.token <TOKEN>"
)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Args)]
struct GlobalArgs {
    #[arg(short = 'j', long, global = true)]
    json: bool,
    #[arg(short = 'q', long, global = true)]
    quiet: bool,
    #[arg(short = 'v', long, global = true)]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Search(SearchArgs),
    Show(ShowArgs),
    Config(ConfigArgs),
}

#[derive(Debug, Args)]
struct SearchArgs {
    city: String,
    #[arg(long)]
    query: Option<String>,
    #[arg(long)]
    date: Option<String>,
    #[arg(long)]
    category: Option<String>,
    #[arg(long, default_value_t = 20)]
    limit: usize,
}

#[derive(Debug, Args)]
struct ShowArgs {
    event_id: String,
}

#[derive(Debug, Args)]
struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Set(ConfigSetArgs),
    Show(ShowFlags),
    Path,
}

#[derive(Debug, Args)]
struct ConfigSetArgs {
    key: String,
    value: String,
    #[command(flatten)]
    output: ShowFlags,
}

#[derive(Debug, Args)]
struct ShowFlags {
    #[arg(short = 'j', long)]
    json: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default)]
    token: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Configuration directory not found")]
    ConfigMissing,
    #[error("Missing Eventbrite token. Set eventbrite.token via config set")]
    AuthMissing,
    #[error("Unknown config key: {0}")]
    InvalidConfigKey(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("HTTP request failed")]
    RequestFailed,
    #[error("Eventbrite API returned an error")]
    ApiError,
    #[error("No item found")]
    NotFound,
    #[error("Response parse failed")]
    ParseFailed,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::ConfigMissing => "CONFIG_MISSING",
            Self::AuthMissing => "AUTH_MISSING",
            Self::InvalidConfigKey(_) | Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::RequestFailed => "REQUEST_FAILED",
            Self::ApiError => "API_ERROR",
            Self::NotFound => "NOT_FOUND",
            Self::ParseFailed => "PARSE_FAILED",
        }
    }
}

#[derive(Debug, Serialize)]
struct OkList<T> {
    ok: bool,
    count: usize,
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct OkItem<T> {
    ok: bool,
    item: T,
}

#[derive(Debug, Serialize)]
struct OkMessage {
    ok: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorJson {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, Serialize)]
struct EventItem {
    id: String,
    name: String,
    description: String,
    start: String,
    end: String,
    status: String,
    url: String,
    city: String,
    venue: String,
}

#[derive(Debug, Deserialize)]
struct EventSearchResponse {
    events: Vec<EventNode>,
}

#[derive(Debug, Deserialize)]
struct EventNode {
    id: String,
    #[serde(default)]
    name: TextNode,
    #[serde(default)]
    description: TextNode,
    #[serde(default)]
    start: DateNode,
    #[serde(default)]
    end: DateNode,
    #[serde(default)]
    status: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    venue: VenueNode,
}

#[derive(Debug, Deserialize, Default)]
struct TextNode {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize, Default)]
struct DateNode {
    #[serde(default)]
    utc: String,
}

#[derive(Debug, Deserialize, Default)]
struct VenueNode {
    #[serde(default)]
    name: String,
    #[serde(default)]
    address: AddressNode,
}

#[derive(Debug, Deserialize, Default)]
struct AddressNode {
    #[serde(default)]
    localized_area_display: String,
}

fn main() {
    let cli = Cli::parse();

    let result = dispatch(&cli);
    if let Err(err) = result {
        if cli.global.json {
            print_json(&ErrorJson {
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

fn dispatch(cli: &Cli) -> Result<(), AppError> {
    match &cli.command {
        Commands::Search(args) => cmd_search(args, &cli.global),
        Commands::Show(args) => cmd_show(args, &cli.global),
        Commands::Config(args) => cmd_config(args),
    }
}

fn cmd_search(args: &SearchArgs, out: &GlobalArgs) -> Result<(), AppError> {
    if args.limit == 0 || args.limit > 50 {
        return Err(AppError::InvalidArgument(
            "--limit must be between 1 and 50".to_string(),
        ));
    }

    let mut url = format!(
        "{}/events/search/?location.address={}&expand=venue&page=1",
        EVENTBRITE_BASE,
        urlencoding::encode(&args.city)
    );

    if let Some(query) = &args.query {
        url.push_str("&q=");
        url.push_str(&urlencoding::encode(query));
    }
    if let Some(date) = &args.date {
        url.push_str("&start_date.keyword=");
        url.push_str(&urlencoding::encode(date));
    }
    if let Some(category) = &args.category {
        url.push_str("&categories=");
        url.push_str(&urlencoding::encode(category));
    }

    let mut body: EventSearchResponse = get_json(&url, out.verbose)?;
    body.events.truncate(args.limit);

    let items: Vec<EventItem> = body.events.into_iter().map(map_event).collect();

    if out.json {
        print_json(&OkList {
            ok: true,
            count: items.len(),
            items,
        });
    } else if out.quiet {
        println!("{}", items.len());
    } else {
        for item in items {
            println!("{} ({})", item.name, item.id);
            println!("  {}", item.start);
            if !item.city.is_empty() {
                println!("  {}", item.city);
            }
        }
    }

    Ok(())
}

fn cmd_show(args: &ShowArgs, out: &GlobalArgs) -> Result<(), AppError> {
    let url = format!("{}/events/{}/?expand=venue", EVENTBRITE_BASE, args.event_id);

    let row: EventNode = get_json(&url, out.verbose)?;
    let item = map_event(row);

    if out.json {
        print_json(&OkItem { ok: true, item });
    } else if out.quiet {
        println!("{}", item.id);
    } else {
        println!("{} ({})", item.name, item.id);
        println!("start: {}", item.start);
        if !item.end.is_empty() {
            println!("end: {}", item.end);
        }
        if !item.venue.is_empty() {
            println!("venue: {}", item.venue);
        }
        if !item.city.is_empty() {
            println!("city: {}", item.city);
        }
        if !item.url.is_empty() {
            println!("url: {}", item.url);
        }
    }

    Ok(())
}

fn get_json<T: for<'de> Deserialize<'de>>(url: &str, verbose: bool) -> Result<T, AppError> {
    let cfg = load_config().map_err(|_| AppError::ConfigMissing)?;
    let token = cfg
        .token
        .filter(|x| !x.trim().is_empty())
        .ok_or(AppError::AuthMissing)?;

    if verbose {
        eprintln!("debug: GET {url}");
    }

    let client = Client::builder()
        .user_agent("dee-events/0.1.0 (https://dee.ink)")
        .build()
        .map_err(|_| AppError::RequestFailed)?;

    let response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .map_err(|_| AppError::RequestFailed)?;

    if response.status().as_u16() == 404 {
        return Err(AppError::NotFound);
    }
    if !response.status().is_success() {
        return Err(AppError::ApiError);
    }

    response.json().map_err(|_| AppError::ParseFailed)
}

fn map_event(row: EventNode) -> EventItem {
    EventItem {
        id: row.id,
        name: row.name.text,
        description: row.description.text,
        start: row.start.utc,
        end: row.end.utc,
        status: row.status,
        url: row.url,
        city: row.venue.address.localized_area_display,
        venue: row.venue.name,
    }
}

fn cmd_config(args: &ConfigArgs) -> Result<(), AppError> {
    match &args.command {
        ConfigCommand::Set(input) => {
            let mut cfg = load_config().unwrap_or_default();
            match input.key.as_str() {
                "eventbrite.token" | "token" => cfg.token = Some(input.value.clone()),
                other => return Err(AppError::InvalidConfigKey(other.to_string())),
            }
            save_config(&cfg).map_err(|_| AppError::ConfigMissing)?;

            if input.output.json {
                print_json(&OkMessage {
                    ok: true,
                    message: "Config updated".to_string(),
                });
            } else {
                println!("Config updated");
            }
            Ok(())
        }
        ConfigCommand::Show(flags) => {
            let cfg = load_config().unwrap_or_default();
            if flags.json {
                print_json(&OkItem {
                    ok: true,
                    item: cfg,
                });
            } else {
                let state = cfg.token.as_deref().map(|_| "set").unwrap_or("missing");
                println!("token: {state}");
            }
            Ok(())
        }
        ConfigCommand::Path => {
            println!("{}", config_path().display());
            Ok(())
        }
    }
}

fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("dee-events");
    path.push("config.toml");
    path
}

fn load_config() -> Result<AppConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed reading config at {}", path.display()))?;
    toml::from_str(&content).context("failed parsing config")
}

fn save_config(cfg: &AppConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, toml::to_string_pretty(cfg)?)?;
    Ok(())
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => println!("{text}"),
        Err(_) => {
            println!(
                "{{\"ok\":false,\"error\":\"serialization failed\",\"code\":\"INTERNAL_ERROR\"}}"
            );
        }
    }
}
