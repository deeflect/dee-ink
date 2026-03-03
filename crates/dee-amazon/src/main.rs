use std::fs;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::Url;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "dee-amazon",
    version,
    about = "Search Amazon products",
    long_about = "dee-amazon - Search Amazon listings with agent-friendly JSON output.",
    after_help = "EXAMPLES:\n  dee-amazon search \"mechanical keyboard\" --limit 10 --json\n  dee-amazon config set amazon.user-agent \"dee-amazon/0.1\"\n  dee-amazon config set amazon.base-url https://www.amazon.com/s\n  dee-amazon config show --json"
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
    query: String,

    #[arg(long, default_value_t = 20)]
    limit: usize,

    #[arg(long)]
    base_url: Option<String>,
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
    user_agent: Option<String>,
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
struct ProductItem {
    id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Configuration directory not found")]
    ConfigMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("HTTP request failed")]
    RequestFailed,
    #[error("Upstream API error: {0}")]
    ApiError(String),
    #[error("Parse failed")]
    ParseFailed,
    #[error("JSON serialization failed")]
    Serialize,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::ConfigMissing => "CONFIG_MISSING",
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
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
    validate_query(&args.query)?;
    if !(1..=100).contains(&args.limit) {
        return Err(AppError::InvalidArgument(
            "limit must be between 1 and 100".to_string(),
        ));
    }

    let config = load_config_or_default()?;
    let base_url = args
        .base_url
        .clone()
        .or(config.base_url)
        .unwrap_or_else(|| "https://www.amazon.com/s".to_string());

    validate_http_url(&base_url)?;

    let user_agent = config
        .user_agent
        .unwrap_or_else(|| "dee-amazon/0.1 (+https://dee.ink)".to_string());

    let url = format!(
        "{}?k={}",
        base_url.trim_end_matches('/'),
        urlencoding::encode(args.query.trim())
    );

    let client = Client::new();
    let response = client
        .get(url)
        .header("User-Agent", user_agent)
        .send()
        .map_err(|_| AppError::RequestFailed)?;

    let status = response.status();
    if !status.is_success() {
        return Err(AppError::ApiError(format!("HTTP {}", status.as_u16())));
    }

    let html = response.text().map_err(|_| AppError::RequestFailed)?;
    let items = parse_products(&html, args.limit)?;

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
        println!("No products found.");
    } else {
        for item in &items {
            let price = item
                .price
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "-".to_string());
            println!("{} | {} | {}", item.id, item.title, price);
        }
    }

    Ok(())
}

fn parse_products(html: &str, limit: usize) -> AppResult<Vec<ProductItem>> {
    let doc = Html::parse_document(html);

    let card_sel = Selector::parse("div[data-component-type='s-search-result']")
        .map_err(|_| AppError::ParseFailed)?;
    let title_sel = Selector::parse("h2 span").map_err(|_| AppError::ParseFailed)?;
    let link_sel = Selector::parse("h2 a").map_err(|_| AppError::ParseFailed)?;
    let price_sel = Selector::parse(".a-price .a-offscreen").map_err(|_| AppError::ParseFailed)?;
    let rating_sel = Selector::parse("span.a-icon-alt").map_err(|_| AppError::ParseFailed)?;
    let review_sel =
        Selector::parse("span.a-size-base.s-underline-text").map_err(|_| AppError::ParseFailed)?;

    let price_re = Regex::new(r"([0-9]{1,3}(?:,[0-9]{3})*(?:\.[0-9]{2})|[0-9]+(?:\.[0-9]{2}))")
        .map_err(|_| AppError::ParseFailed)?;
    let rating_re = Regex::new(r"([0-9]+(?:\.[0-9]+)?)").map_err(|_| AppError::ParseFailed)?;

    let mut out = Vec::new();

    for card in doc.select(&card_sel).take(limit) {
        let id = card
            .value()
            .attr("data-asin")
            .unwrap_or_default()
            .to_string();
        if id.is_empty() {
            continue;
        }

        let title = card
            .select(&title_sel)
            .next()
            .map(|node| node.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        if title.is_empty() {
            continue;
        }

        let url = card.select(&link_sel).next().and_then(|node| {
            node.value()
                .attr("href")
                .map(|href| format!("https://www.amazon.com{}", href))
        });

        let price_text = card
            .select(&price_sel)
            .next()
            .map(|node| node.text().collect::<String>());

        let (price, currency) = match price_text {
            Some(text) => {
                let currency = if text.contains('$') {
                    Some("USD".to_string())
                } else if text.contains('€') {
                    Some("EUR".to_string())
                } else if text.contains('£') {
                    Some("GBP".to_string())
                } else {
                    None
                };
                let price = price_re
                    .captures(&text)
                    .and_then(|caps| caps.get(1).map(|m| m.as_str().replace(',', "")))
                    .and_then(|raw| raw.parse::<f64>().ok());
                (price, currency)
            }
            None => (None, None),
        };

        let rating = card.select(&rating_sel).next().and_then(|node| {
            let text = node.text().collect::<String>();
            let capture = rating_re
                .captures(&text)
                .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()));
            capture.and_then(|raw| raw.parse::<f64>().ok())
        });

        let review_count = card.select(&review_sel).next().and_then(|node| {
            node.text()
                .collect::<String>()
                .replace(',', "")
                .trim()
                .parse::<i64>()
                .ok()
        });

        out.push(ProductItem {
            id,
            title,
            price,
            currency,
            rating,
            review_count,
            url,
        });
    }

    Ok(out)
}

fn validate_query(query: &str) -> AppResult<()> {
    if query.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "query cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_http_url(value: &str) -> AppResult<()> {
    let parsed = Url::parse(value)
        .map_err(|_| AppError::InvalidArgument(format!("invalid base url '{value}'")))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::InvalidArgument(
            "base url must use http or https".to_string(),
        ));
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
        "amazon.user-agent" => {
            ensure_not_empty(&args.value)?;
            config.user_agent = Some(args.value.clone());
        }
        "amazon.base-url" => {
            ensure_not_empty(&args.value)?;
            validate_http_url(&args.value)?;
            config.base_url = Some(args.value.clone());
        }
        other => {
            return Err(AppError::InvalidArgument(format!(
                "Unknown config key: {other}"
            )))
        }
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
        println!("user_agent={}", config.user_agent.unwrap_or_default());
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
    Ok(base.join("dee-amazon").join("config.toml"))
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
