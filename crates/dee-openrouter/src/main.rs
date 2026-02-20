use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const API_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

#[derive(Parser, Debug)]
#[command(
    name = "dee-openrouter",
    version,
    about = "Search, filter, and inspect OpenRouter models",
    long_about = None,
    after_help = "EXAMPLES:\n  dee-openrouter list --provider google\n  dee-openrouter list --free --limit 10 --json\n  dee-openrouter search gemini --json\n  dee-openrouter show google/gemini-2.5-pro --json\n  dee-openrouter config set openrouter.api-key sk-xxx\n  dee-openrouter config show --json\n  dee-openrouter config path"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List models with optional filters
    List(ListArgs),
    /// Show one model by id
    Show(ItemArgs),
    /// Search models by id/name/description
    Search(SearchArgs),
    /// Manage configuration
    Config(ConfigArgs),
}

#[derive(Args, Debug, Clone)]
struct OutputFlags {
    /// Output as JSON
    #[arg(short = 'j', long)]
    json: bool,
    /// Suppress decorative output
    #[arg(short = 'q', long)]
    quiet: bool,
    /// Debug output to stderr
    #[arg(short = 'v', long)]
    verbose: bool,
}

#[derive(Args, Debug)]
struct ListArgs {
    /// Filter by provider prefix (e.g. google, openai, anthropic)
    #[arg(long)]
    provider: Option<String>,
    /// Only include free models
    #[arg(long)]
    free: bool,
    /// Maximum price per 1M input tokens
    #[arg(long)]
    max_price: Option<f64>,
    /// Minimum context window
    #[arg(long)]
    context_min: Option<u64>,
    /// Limit number of results
    #[arg(long)]
    limit: Option<usize>,
    #[command(flatten)]
    output: OutputFlags,
}

#[derive(Args, Debug)]
struct ItemArgs {
    /// OpenRouter model id (e.g. google/gemini-2.5-pro)
    model_id: String,
    #[command(flatten)]
    output: OutputFlags,
}

#[derive(Args, Debug)]
struct SearchArgs {
    /// Search query over id/name/description
    query: String,
    /// Limit number of results
    #[arg(long)]
    limit: Option<usize>,
    #[command(flatten)]
    output: OutputFlags,
}

#[derive(Args, Debug)]
struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Subcommand, Debug)]
enum ConfigCommand {
    /// Set a configuration value (e.g. openrouter.api-key <key>)
    Set(ConfigSetArgs),
    /// Show current configuration
    Show(ShowFlags),
    /// Print the path to the config file
    Path,
}

#[derive(Args, Debug)]
struct ConfigSetArgs {
    /// Config key (e.g. openrouter.api-key)
    key: String,
    /// Value to set
    value: String,
    #[command(flatten)]
    output: ShowFlags,
}

#[derive(Args, Debug)]
struct ShowFlags {
    /// Output as JSON
    #[arg(short = 'j', long)]
    json: bool,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    context_length: u64,
    pricing: OpenRouterPricing,
    #[serde(default)]
    top_provider: OpenRouterTopProvider,
    #[serde(default)]
    created: u64,
}

#[derive(Debug, Deserialize, Default)]
struct OpenRouterPricing {
    #[serde(default)]
    prompt: String,
    #[serde(default)]
    completion: String,
}

#[derive(Debug, Deserialize, Default)]
struct OpenRouterTopProvider {
    #[serde(default)]
    context_length: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
struct ModelItem {
    id: String,
    provider: String,
    name: String,
    description: String,
    context_length: u64,
    price_prompt_per_1m: f64,
    price_completion_per_1m: f64,
    free: bool,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct SuccessList<T: Serialize> {
    ok: bool,
    count: usize,
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct SuccessItem<T: Serialize> {
    ok: bool,
    item: T,
}

#[derive(Debug, Serialize)]
struct SuccessMsg {
    ok: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct JsonError {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Model not found: {0}")]
    NotFound(String),
    #[error("Unknown config key: {0}")]
    UnknownKey(String),
}

/// Serializable config stored in ~/.config/dee-openrouter/config.toml
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct AppConfig {
    #[serde(default)]
    api_key: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json_errors = command_json_mode(&cli.command);

    let run = dispatch(cli).await;
    if let Err(err) = run {
        if json_errors {
            let payload = JsonError {
                ok: false,
                error: err.to_string(),
                code: classify_error_code(&err).to_string(),
            };
            if let Ok(rendered) = serde_json::to_string_pretty(&payload) {
                println!("{rendered}");
            } else {
                println!("{{\"ok\":false,\"error\":\"serialization failure\",\"code\":\"INTERNAL_ERROR\"}}");
            }
        } else {
            eprintln!("{err:#}");
        }
        std::process::exit(1);
    }
}

async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::List(args) => handle_list(args).await,
        Commands::Show(args) => handle_show(args).await,
        Commands::Search(args) => handle_search(args).await,
        Commands::Config(args) => handle_config(args),
    }
}

async fn handle_list(args: ListArgs) -> Result<()> {
    let api_key = load_config().ok().and_then(|c| c.api_key);
    let models = fetch_models(args.output.verbose, api_key.as_deref()).await?;
    let provider_filter = args.provider.as_deref().map(str::to_lowercase);

    let mut items: Vec<ModelItem> = models
        .into_iter()
        .map(normalize_model)
        .filter(|item| {
            provider_filter
                .as_deref()
                .map(|provider| item.provider == provider)
                .unwrap_or(true)
        })
        .filter(|item| !args.free || item.free)
        .filter(|item| {
            args.max_price
                .map(|max| item.price_prompt_per_1m <= max)
                .unwrap_or(true)
        })
        .filter(|item| {
            args.context_min
                .map(|min| item.context_length >= min)
                .unwrap_or(true)
        })
        .collect();

    if let Some(limit) = args.limit {
        items.truncate(limit);
    }

    if args.output.json {
        print_json(&SuccessList {
            ok: true,
            count: items.len(),
            items,
        })
    } else {
        print_models_table(&items, args.output.quiet);
        Ok(())
    }
}

async fn handle_show(args: ItemArgs) -> Result<()> {
    let api_key = load_config().ok().and_then(|c| c.api_key);
    let model_id = args.model_id.to_lowercase();
    let item = fetch_models(args.output.verbose, api_key.as_deref())
        .await?
        .into_iter()
        .map(normalize_model)
        .find(|item| item.id.to_lowercase() == model_id)
        .ok_or_else(|| anyhow::anyhow!(AppError::NotFound(args.model_id.clone())))?;

    if args.output.json {
        print_json(&SuccessItem { ok: true, item })
    } else {
        if !args.output.quiet {
            println!("{}", item.id);
            println!("provider: {}", item.provider);
            println!("name: {}", item.name);
            println!("context_length: {}", item.context_length);
            println!("price_prompt_per_1m: {:.6}", item.price_prompt_per_1m);
            println!(
                "price_completion_per_1m: {:.6}",
                item.price_completion_per_1m
            );
            println!("free: {}", item.free);
            println!("created_at: {}", item.created_at);
            println!("description: {}", item.description);
        } else {
            println!("{}", item.id);
        }
        Ok(())
    }
}

async fn handle_search(args: SearchArgs) -> Result<()> {
    let api_key = load_config().ok().and_then(|c| c.api_key);
    let q = args.query.to_lowercase();
    let mut items: Vec<ModelItem> = fetch_models(args.output.verbose, api_key.as_deref())
        .await?
        .into_iter()
        .map(normalize_model)
        .filter(|item| {
            item.id.to_lowercase().contains(&q)
                || item.name.to_lowercase().contains(&q)
                || item.description.to_lowercase().contains(&q)
        })
        .collect();

    if let Some(limit) = args.limit {
        items.truncate(limit);
    }

    if args.output.json {
        print_json(&SuccessList {
            ok: true,
            count: items.len(),
            items,
        })
    } else {
        print_models_table(&items, args.output.quiet);
        Ok(())
    }
}

fn handle_config(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Set(set_args) => {
            if set_args.key != "openrouter.api-key" {
                return Err(anyhow::anyhow!(AppError::UnknownKey(set_args.key)));
            }
            let mut cfg = load_config().unwrap_or_default();
            cfg.api_key = Some(set_args.value);
            save_config(&cfg)?;
            if set_args.output.json {
                print_json(&SuccessMsg {
                    ok: true,
                    message: format!("Set {}", set_args.key),
                })?;
            } else {
                println!("Saved {}", set_args.key);
            }
            Ok(())
        }
        ConfigCommand::Show(flags) => {
            let cfg = load_config().unwrap_or_default();
            if flags.json {
                #[derive(Serialize)]
                struct ConfigShow {
                    ok: bool,
                    item: ConfigShowItem,
                }
                #[derive(Serialize)]
                struct ConfigShowItem {
                    path: String,
                    api_key_set: bool,
                }
                print_json(&ConfigShow {
                    ok: true,
                    item: ConfigShowItem {
                        path: config_path().display().to_string(),
                        api_key_set: cfg.api_key.is_some(),
                    },
                })
            } else {
                println!("path: {}", config_path().display());
                println!("api_key_set: {}", cfg.api_key.is_some());
                Ok(())
            }
        }
        ConfigCommand::Path => {
            println!("{}", config_path().display());
            Ok(())
        }
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dee-openrouter")
        .join("config.toml")
}

fn load_config() -> Result<AppConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    toml::from_str(&content).context("invalid config.toml")
}

fn save_config(cfg: &AppConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(cfg).context("failed to serialize config")?;
    fs::write(&path, content).with_context(|| format!("failed to write config {}", path.display()))
}

async fn fetch_models(verbose: bool, api_key: Option<&str>) -> Result<Vec<OpenRouterModel>> {
    if verbose {
        eprintln!("Fetching models from {API_MODELS_URL}");
    }

    let client = reqwest::Client::new();
    let mut req = client
        .get(API_MODELS_URL)
        .header("Accept", "application/json");

    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let response = req.send().await.context("request to OpenRouter failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "unable to read response body".to_string());
        anyhow::bail!("OpenRouter API error: {status} - {body}");
    }

    let parsed = response
        .json::<OpenRouterResponse>()
        .await
        .context("invalid OpenRouter API response")?;

    Ok(parsed.data)
}

fn normalize_model(model: OpenRouterModel) -> ModelItem {
    let provider = model
        .id
        .split('/')
        .next()
        .map(str::to_lowercase)
        .unwrap_or_default();

    let prompt = parse_price_per_1m(&model.pricing.prompt).unwrap_or(0.0);
    let completion = parse_price_per_1m(&model.pricing.completion).unwrap_or(0.0);
    let context = model
        .top_provider
        .context_length
        .filter(|len| *len > 0)
        .unwrap_or(model.context_length);

    let created_at = match i64::try_from(model.created) {
        Ok(sec) if sec > 0 => chrono::DateTime::from_timestamp(sec, 0)
            .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string()),
        _ => "1970-01-01T00:00:00Z".to_string(),
    };

    ModelItem {
        id: model.id,
        provider,
        name: default_if_empty(model.name, "(unnamed)"),
        description: default_if_empty(model.description, ""),
        context_length: context,
        price_prompt_per_1m: prompt,
        price_completion_per_1m: completion,
        free: prompt == 0.0 && completion == 0.0,
        created_at,
    }
}

fn parse_price_per_1m(raw: &str) -> Option<f64> {
    if raw.trim().is_empty() {
        return Some(0.0);
    }
    raw.trim()
        .parse::<f64>()
        .ok()
        .map(|per_token| per_token * 1_000_000.0)
}

fn default_if_empty(value: String, default: &str) -> String {
    if value.trim().is_empty() {
        default.to_string()
    } else {
        value
    }
}

fn print_models_table(items: &[ModelItem], quiet: bool) {
    if quiet {
        for item in items {
            println!("{}", item.id);
        }
        return;
    }

    println!("Found {} model(s):", items.len());
    for item in items {
        println!(
            "- {} | ctx={} | in=${:.6}/1M | out=${:.6}/1M{}",
            item.id,
            item.context_length,
            item.price_prompt_per_1m,
            item.price_completion_per_1m,
            if item.free { " | FREE" } else { "" }
        );
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let output = serde_json::to_string_pretty(value)?;
    println!("{output}");
    Ok(())
}

fn command_json_mode(command: &Commands) -> bool {
    match command {
        Commands::List(args) => args.output.json,
        Commands::Show(args) => args.output.json,
        Commands::Search(args) => args.output.json,
        Commands::Config(args) => match &args.command {
            ConfigCommand::Set(a) => a.output.json,
            ConfigCommand::Show(a) => a.json,
            ConfigCommand::Path => false,
        },
    }
}

fn classify_error_code(err: &anyhow::Error) -> &'static str {
    if let Some(app) = err.downcast_ref::<AppError>() {
        return match app {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::UnknownKey(_) => "INVALID_ARGUMENT",
        };
    }
    if err.to_string().contains("OpenRouter API error") {
        "API_ERROR"
    } else if err.to_string().contains("request to OpenRouter failed") {
        "NETWORK_ERROR"
    } else {
        "INTERNAL_ERROR"
    }
}
