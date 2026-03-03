use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine as _;
use chrono::{SecondsFormat, Utc};
use clap::{Args, Parser, Subcommand};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "dee-receipt",
    version,
    about = "Scan receipt images into structured expense JSON",
    long_about = "dee-receipt - Extract structured receipt fields from images using a vision model.",
    after_help = "EXAMPLES:\n  dee-receipt config set openai.api-key sk-...\n  dee-receipt scan ./receipt.jpg --json\n  dee-receipt scan ./receipt.png --model gpt-4o-mini --json\n  dee-receipt config show --json"
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
    /// Scan a receipt image
    Scan(ScanArgs),
    /// Manage configuration
    Config(ConfigArgs),
}

#[derive(Debug, Args)]
struct ScanArgs {
    image: String,

    #[arg(long, default_value = "gpt-4o-mini")]
    model: String,

    #[arg(long)]
    prompt: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Default)]
struct AppConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    openai_api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    openai_base_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReceiptItem {
    merchant: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    currency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tax: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tip: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    line_items: Vec<LineItem>,
    parsed_at: String,
}

#[derive(Debug, Serialize)]
struct LineItem {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    qty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Configuration directory not found")]
    ConfigMissing,
    #[error("Missing OpenAI API key. Set openai.api-key via config set")]
    AuthMissing,
    #[error("Unknown config key: {0}")]
    InvalidConfigKey(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("HTTP request failed")]
    RequestFailed,
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Response parse failed: {0}")]
    ParseFailed(String),
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
        Commands::Scan(args) => cmd_scan(args, &cli.global),
        Commands::Config(args) => cmd_config(args, &cli.global),
    }
}

fn cmd_scan(args: &ScanArgs, global: &GlobalArgs) -> AppResult<()> {
    if args.model.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "model cannot be empty".to_string(),
        ));
    }

    let image_path = PathBuf::from(&args.image);
    if !image_path.exists() {
        return Err(AppError::InvalidArgument(format!(
            "image not found: {}",
            image_path.display()
        )));
    }

    let bytes = fs::read(&image_path)
        .map_err(|_| AppError::InvalidArgument("failed to read image file".to_string()))?;

    if bytes.is_empty() {
        return Err(AppError::InvalidArgument("image file is empty".to_string()));
    }

    let config = load_config_or_default()?;
    let api_key = config.openai_api_key.ok_or(AppError::AuthMissing)?;
    let base_url = config
        .openai_base_url
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    let mime = infer_mime_type(&image_path);
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let data_url = format!("data:{mime};base64,{encoded}");

    let prompt = args.prompt.clone().unwrap_or_else(default_prompt);

    let payload = serde_json::json!({
        "model": args.model,
        "response_format": {"type": "json_object"},
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": prompt},
                {"type": "image_url", "image_url": {"url": data_url}}
            ]
        }]
    });

    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let client = Client::new();
    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .map_err(|_| AppError::RequestFailed)?;

    let status = response.status();
    let body = response.text().map_err(|_| AppError::RequestFailed)?;

    if !status.is_success() {
        return Err(AppError::ApiError(format!("HTTP {}", status.as_u16())));
    }

    let parsed: OpenAiResponse = serde_json::from_str(&body)
        .map_err(|_| AppError::ParseFailed("invalid API JSON".to_string()))?;

    let content = parsed
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .ok_or_else(|| AppError::ParseFailed("missing model content".to_string()))?;

    let model_json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|_| AppError::ParseFailed("model did not return JSON object".to_string()))?;

    let item = normalize_receipt(model_json);

    if global.json {
        print_json(&OkItem { ok: true, item });
        return Ok(());
    }

    if global.quiet {
        if let Some(total) = item.total {
            println!("{total:.2}");
        } else {
            println!("{}", item.merchant);
        }
        return Ok(());
    }

    println!("Merchant: {}", item.merchant);
    if let Some(date) = item.date.as_deref() {
        println!("Date: {date}");
    }
    if let Some(total) = item.total {
        let currency = item.currency.as_deref().unwrap_or("USD");
        println!("Total: {total:.2} {currency}");
    }
    println!("Line items: {}", item.line_items.len());

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
        "openai.api-key" => {
            if args.value.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "value cannot be empty".to_string(),
                ));
            }
            config.openai_api_key = Some(args.value.clone());
        }
        "openai.base-url" => {
            if args.value.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "value cannot be empty".to_string(),
                ));
            }
            config.openai_base_url = Some(args.value.clone());
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
        println!(
            "openai_api_key={}",
            config.openai_api_key.unwrap_or_default()
        );
        println!(
            "openai_base_url={}",
            config.openai_base_url.unwrap_or_default()
        );
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

fn normalize_receipt(value: serde_json::Value) -> ReceiptItem {
    let merchant = value
        .get("merchant")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let date = value
        .get("date")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(ToOwned::to_owned);
    let currency = value
        .get("currency")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_ascii_uppercase());
    let total = read_number(value.get("total"));
    let tax = read_number(value.get("tax"));
    let tip = read_number(value.get("tip"));

    let line_items = value
        .get("line_items")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| LineItem {
                    name: item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    qty: read_number(item.get("qty")),
                    unit_price: read_number(item.get("unit_price")),
                    total: read_number(item.get("total")),
                })
                .filter(|item| !item.name.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    ReceiptItem {
        merchant,
        date,
        currency,
        total,
        tax,
        tip,
        line_items,
        parsed_at: now_timestamp(),
    }
}

fn read_number(value: Option<&serde_json::Value>) -> Option<f64> {
    match value {
        Some(serde_json::Value::Number(number)) => number.as_f64(),
        Some(serde_json::Value::String(text)) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn default_prompt() -> String {
    "Extract this receipt as JSON with keys: merchant, date, total, currency, tax, tip, line_items (array of {name, qty, unit_price, total}). Return JSON only."
        .to_string()
}

fn infer_mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    }
}

fn config_path() -> AppResult<PathBuf> {
    let base = dirs::config_dir().ok_or(AppError::ConfigMissing)?;
    Ok(base.join("dee-receipt").join("config.toml"))
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

fn now_timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
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
