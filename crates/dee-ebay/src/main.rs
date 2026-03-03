use std::fs;
use std::path::PathBuf;

use base64::Engine as _;
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "dee-ebay",
    version,
    about = "Search eBay listings from the terminal",
    long_about = "dee-ebay - Query eBay Browse API with agent-friendly JSON output.",
    after_help = "EXAMPLES:\n  dee-ebay config set ebay.client-id <ID>\n  dee-ebay config set ebay.client-secret <SECRET>\n  dee-ebay search \"mechanical keyboard\" --limit 10 --json\n  dee-ebay search \"nintendo switch\" --sort newly-listed --json"
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

#[derive(Debug, Clone, ValueEnum)]
enum SortArg {
    #[value(name = "best-match")]
    BestMatch,
    #[value(name = "newly-listed")]
    NewlyListed,
    #[value(name = "ending-soonest")]
    EndingSoonest,
}

impl SortArg {
    fn as_api_value(&self) -> &'static str {
        match self {
            Self::BestMatch => "bestMatch",
            Self::NewlyListed => "newlyListed",
            Self::EndingSoonest => "endingSoonest",
        }
    }
}

#[derive(Debug, Args)]
struct SearchArgs {
    query: String,

    #[arg(long, default_value_t = 20)]
    limit: usize,

    #[arg(long, value_enum)]
    sort: Option<SortArg>,
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
    client_id: Option<String>,
    #[serde(default)]
    client_secret: Option<String>,
    #[serde(default)]
    sandbox: Option<bool>,
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
struct SearchItem {
    id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct BrowseResponse {
    #[serde(default)]
    item_summaries: Vec<BrowseItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrowseItem {
    item_id: String,
    title: String,
    #[serde(default)]
    condition: Option<String>,
    #[serde(default)]
    item_web_url: Option<String>,
    #[serde(default)]
    price: Option<BrowsePrice>,
    #[serde(default)]
    seller: Option<BrowseSeller>,
}

#[derive(Debug, Deserialize)]
struct BrowsePrice {
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    currency: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BrowseSeller {
    #[serde(default)]
    username: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Configuration directory not found")]
    ConfigMissing,
    #[error("Missing eBay credentials. Set ebay.client-id and ebay.client-secret")]
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
    if args.query.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "query cannot be empty".to_string(),
        ));
    }
    if args.limit == 0 || args.limit > 200 {
        return Err(AppError::InvalidArgument(
            "limit must be between 1 and 200".to_string(),
        ));
    }

    let config = load_config_or_default()?;
    let client_id = config.client_id.ok_or(AppError::AuthMissing)?;
    let client_secret = config.client_secret.ok_or(AppError::AuthMissing)?;
    let sandbox = config.sandbox.unwrap_or(false);

    let auth_base = if sandbox {
        "https://api.sandbox.ebay.com"
    } else {
        "https://api.ebay.com"
    };

    let token = fetch_access_token(auth_base, &client_id, &client_secret)?;

    let mut url = format!(
        "{auth_base}/buy/browse/v1/item_summary/search?q={}&limit={}",
        urlencoding::encode(args.query.trim()),
        args.limit
    );

    if let Some(sort) = &args.sort {
        url.push_str("&sort=");
        url.push_str(sort.as_api_value());
    }

    let client = Client::new();
    let response = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .send()
        .map_err(|_| AppError::RequestFailed)?;

    let status = response.status();
    let body = response.text().map_err(|_| AppError::RequestFailed)?;

    if !status.is_success() {
        return Err(AppError::ApiError(format!("HTTP {}", status.as_u16())));
    }

    let parsed: BrowseResponse = serde_json::from_str(&body).map_err(|_| AppError::ParseFailed)?;
    let items = parsed
        .item_summaries
        .into_iter()
        .map(|item| SearchItem {
            id: item.item_id,
            title: item.title,
            condition: item.condition,
            price: item
                .price
                .as_ref()
                .and_then(|price| price.value.as_deref())
                .and_then(|value| value.parse::<f64>().ok()),
            currency: item.price.and_then(|price| price.currency),
            seller: item.seller.and_then(|seller| seller.username),
            url: item.item_web_url,
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
        println!("No items found.");
    } else {
        for item in &items {
            let price = item
                .price
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "-".to_string());
            let currency = item.currency.as_deref().unwrap_or("-");
            println!("{} | {} | {} {}", item.id, item.title, price, currency);
        }
    }

    Ok(())
}

fn fetch_access_token(base: &str, client_id: &str, client_secret: &str) -> AppResult<String> {
    let endpoint = format!("{base}/identity/v1/oauth2/token");
    let basic = base64::engine::general_purpose::STANDARD.encode(format!(
        "{}:{}",
        client_id.trim(),
        client_secret.trim()
    ));

    let client = Client::new();
    let response = client
        .post(endpoint)
        .header(AUTHORIZATION, format!("Basic {basic}"))
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body("grant_type=client_credentials&scope=https%3A%2F%2Fapi.ebay.com%2Foauth%2Fapi_scope")
        .send()
        .map_err(|_| AppError::RequestFailed)?;

    let status = response.status();
    let body = response.text().map_err(|_| AppError::RequestFailed)?;

    if !status.is_success() {
        return Err(AppError::ApiError(format!(
            "token request failed with HTTP {}",
            status.as_u16()
        )));
    }

    let token: TokenResponse = serde_json::from_str(&body).map_err(|_| AppError::ParseFailed)?;
    if token.access_token.trim().is_empty() {
        return Err(AppError::ParseFailed);
    }

    Ok(token.access_token)
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
        "ebay.client-id" => {
            if args.value.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "value cannot be empty".to_string(),
                ));
            }
            config.client_id = Some(args.value.clone());
        }
        "ebay.client-secret" => {
            if args.value.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "value cannot be empty".to_string(),
                ));
            }
            config.client_secret = Some(args.value.clone());
        }
        "ebay.sandbox" => {
            let parsed = args.value.trim().parse::<bool>().map_err(|_| {
                AppError::InvalidArgument("sandbox must be true or false".to_string())
            })?;
            config.sandbox = Some(parsed);
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

fn cmd_config_show(json: bool) -> AppResult<()> {
    let config = load_config_or_default()?;

    if json {
        print_json(&OkItem {
            ok: true,
            item: config,
        });
    } else {
        println!("client_id={}", config.client_id.unwrap_or_default());
        println!("client_secret={}", config.client_secret.unwrap_or_default());
        println!("sandbox={}", config.sandbox.unwrap_or(false));
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
    Ok(base.join("dee-ebay").join("config.toml"))
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
