use std::fs;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "dee-parking",
    version,
    about = "Find parking spots near a location",
    long_about = "dee-parking - Query parking places with agent-friendly JSON output.",
    after_help = "EXAMPLES:\n  dee-parking config set google.api-key <KEY>\n  dee-parking search \"Downtown Austin\" --limit 10 --json\n  dee-parking search \"Mission District SF\" --query \"covered parking near Mission District SF\" --json"
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
    Config(ConfigArgs),
}

#[derive(Debug, Args)]
struct SearchArgs {
    location: String,

    #[arg(long)]
    query: Option<String>,

    #[arg(long, default_value_t = 20)]
    limit: usize,
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
    api_key: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
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
struct ParkingItem {
    name: String,
    address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rating_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    business_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    open_now: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PlacesResponse {
    #[serde(default)]
    status: String,
    #[serde(default)]
    results: Vec<PlaceResult>,
}

#[derive(Debug, Deserialize)]
struct PlaceResult {
    #[serde(default)]
    name: String,
    #[serde(default)]
    formatted_address: String,
    #[serde(default)]
    rating: Option<f64>,
    #[serde(default)]
    user_ratings_total: Option<i64>,
    #[serde(default)]
    business_status: Option<String>,
    #[serde(default)]
    opening_hours: Option<OpeningHours>,
}

#[derive(Debug, Deserialize)]
struct OpeningHours {
    #[serde(default)]
    open_now: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Configuration directory not found")]
    ConfigMissing,
    #[error("Missing Google API key. Set google.api-key via config set")]
    AuthMissing,
    #[error("Unknown config key: {0}")]
    InvalidConfigKey(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("HTTP request failed")]
    RequestFailed,
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Response parse failed")]
    ParseFailed,
    #[error("JSON serialization failed")]
    Serialize,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::ConfigMissing => "CONFIG_MISSING",
            Self::AuthMissing => "AUTH_MISSING",
            Self::InvalidConfigKey(_) | Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::RequestFailed => "REQUEST_FAILED",
            Self::ApiError(_) => "API_ERROR",
            Self::ParseFailed => "PARSE_FAILED",
            Self::Serialize => "SERIALIZE",
        }
    }
}

type AppResult<T> = Result<T, AppError>;

fn main() {
    let cli = parse_cli();

    if let Err(err) = dispatch(&cli) {
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

fn dispatch(cli: &Cli) -> AppResult<()> {
    match &cli.command {
        Commands::Search(args) => cmd_search(args, &cli.global),
        Commands::Config(args) => cmd_config(args, &cli.global),
    }
}

fn cmd_search(args: &SearchArgs, global: &GlobalArgs) -> AppResult<()> {
    validate_text(&args.location, "location")?;
    if !(1..=50).contains(&args.limit) {
        return Err(AppError::InvalidArgument(
            "limit must be between 1 and 50".to_string(),
        ));
    }

    let config = load_config_or_default()?;
    let api_key = config.api_key.ok_or(AppError::AuthMissing)?;
    let base_url = config.base_url.unwrap_or_else(|| {
        "https://maps.googleapis.com/maps/api/place/textsearch/json".to_string()
    });

    let query = args
        .query
        .clone()
        .unwrap_or_else(|| format!("parking near {}", args.location.trim()));

    let url = format!(
        "{}?query={}&key={}",
        base_url,
        urlencoding::encode(&query),
        urlencoding::encode(api_key.trim())
    );

    let client = Client::new();
    let response = client
        .get(url)
        .send()
        .map_err(|_| AppError::RequestFailed)?;

    let status = response.status();
    let body = response.text().map_err(|_| AppError::RequestFailed)?;

    if !status.is_success() {
        return Err(AppError::ApiError(format!("HTTP {}", status.as_u16())));
    }

    let parsed: PlacesResponse = serde_json::from_str(&body).map_err(|_| AppError::ParseFailed)?;

    if parsed.status != "OK" && parsed.status != "ZERO_RESULTS" {
        return Err(AppError::ApiError(parsed.status));
    }

    let items = parsed
        .results
        .into_iter()
        .take(args.limit)
        .map(|entry| ParkingItem {
            name: entry.name,
            address: entry.formatted_address,
            rating: entry.rating,
            rating_count: entry.user_ratings_total,
            business_status: entry.business_status,
            open_now: entry.opening_hours.and_then(|hours| hours.open_now),
        })
        .collect::<Vec<_>>();

    if global.json {
        print_json(&OkList {
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
        println!("No parking spots found.");
    } else {
        for item in &items {
            println!("{} | {}", item.name, item.address);
        }
    }

    Ok(())
}

fn validate_text(value: &str, field: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{field} cannot be empty"
        )));
    }
    Ok(())
}

fn cmd_config(args: &ConfigArgs, global: &GlobalArgs) -> AppResult<()> {
    match &args.command {
        ConfigCommand::Set(set) => cmd_config_set(set, global),
        ConfigCommand::Show(flags) => cmd_config_show(flags.json || global.json),
        ConfigCommand::Path => cmd_config_path(global.json),
    }
}

fn cmd_config_set(args: &ConfigSetArgs, global: &GlobalArgs) -> AppResult<()> {
    let mut config = load_config_or_default()?;

    match args.key.as_str() {
        "google.api-key" => {
            ensure_not_empty(&args.value)?;
            config.api_key = Some(args.value.clone());
        }
        "google.base-url" => {
            ensure_not_empty(&args.value)?;
            config.base_url = Some(args.value.clone());
        }
        other => return Err(AppError::InvalidConfigKey(other.to_string())),
    }

    save_config(&config)?;

    if global.json || args.output.json {
        print_json(&OkMessage {
            ok: true,
            message: "Config updated".to_string(),
        });
    } else if global.quiet {
        println!("ok");
    } else {
        println!("Config updated");
    }

    Ok(())
}

fn ensure_not_empty(value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "value cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn cmd_config_show(json: bool) -> AppResult<()> {
    let config = load_config_or_default()?;

    if json {
        print_json(&OkItem {
            ok: true,
            item: config,
        });
    } else {
        println!("api_key={}", config.api_key.unwrap_or_default());
        println!("base_url={}", config.base_url.unwrap_or_default());
    }

    Ok(())
}

fn cmd_config_path(json: bool) -> AppResult<()> {
    let path = config_path()?;
    let rendered = path.display().to_string();

    if json {
        print_json(&OkList {
            ok: true,
            count: 1,
            items: vec![rendered],
        });
    } else {
        println!("{rendered}");
    }

    Ok(())
}

fn config_path() -> AppResult<PathBuf> {
    let base = dirs::config_dir().ok_or(AppError::ConfigMissing)?;
    Ok(base.join("dee-parking").join("config.toml"))
}

fn load_config_or_default() -> AppResult<AppConfig> {
    match load_config() {
        Ok(config) => Ok(config),
        Err(AppError::InvalidArgument(_)) => Ok(AppConfig::default()),
        Err(other) => Err(other),
    }
}

fn load_config() -> AppResult<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Err(AppError::InvalidArgument(
            "config file not found".to_string(),
        ));
    }

    let raw = fs::read_to_string(path)
        .map_err(|_| AppError::InvalidArgument("failed to read config".to_string()))?;
    toml::from_str(&raw).map_err(|_| AppError::InvalidArgument("invalid config TOML".to_string()))
}

fn save_config(config: &AppConfig) -> AppResult<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|_| AppError::InvalidArgument("failed to create config dir".to_string()))?;
    }

    let raw = toml::to_string(config)
        .map_err(|_| AppError::InvalidArgument("failed to encode config".to_string()))?;
    fs::write(path, raw)
        .map_err(|_| AppError::InvalidArgument("failed to write config".to_string()))
}

fn print_json<T: Serialize>(value: &T) {
    if let Ok(rendered) = serde_json::to_string(value) {
        println!("{rendered}");
    } else {
        let payload = ErrorJson {
            ok: false,
            error: "JSON serialization failed".to_string(),
            code: AppError::Serialize.code().to_string(),
        };
        if let Ok(rendered) = serde_json::to_string(&payload) {
            println!("{rendered}");
        } else {
            println!(
                "{{\"ok\":false,\"error\":\"JSON serialization failed\",\"code\":\"SERIALIZE\"}}"
            );
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
