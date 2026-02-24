use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

const YELP_BASE: &str = "https://api.yelp.com/v3";

#[derive(Debug, Parser)]
#[command(
    name = "dee-food",
    version,
    about = "Restaurant search CLI (Yelp)",
    after_help = "EXAMPLES:\n  dee-food search \"New York, NY\" --term sushi --limit 10 --json\n  dee-food show yelp-san-francisco --json\n  dee-food reviews yelp-san-francisco --json\n  dee-food config set yelp.api-key <KEY>"
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
    Reviews(ShowArgs),
    Config(ConfigArgs),
}

#[derive(Debug, Clone, ValueEnum)]
enum SortBy {
    BestMatch,
    Rating,
    ReviewCount,
    Distance,
}

#[derive(Debug, Args)]
struct SearchArgs {
    location: String,
    #[arg(long)]
    term: Option<String>,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = SortBy::BestMatch)]
    sort: SortBy,
}

#[derive(Debug, Args)]
struct ShowArgs {
    business_id: String,
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
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Configuration directory not found")]
    ConfigMissing,
    #[error("Missing Yelp API key. Set yelp.api-key via config set")]
    AuthMissing,
    #[error("Unknown config key: {0}")]
    InvalidConfigKey(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("HTTP request failed")]
    RequestFailed,
    #[error("Yelp API returned an error")]
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
struct BusinessItem {
    id: String,
    name: String,
    url: String,
    rating: f64,
    review_count: i64,
    price: String,
    phone: String,
    location: String,
}

#[derive(Debug, Serialize)]
struct ReviewItem {
    id: String,
    rating: i64,
    text: String,
    time_created: String,
    user_name: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct YelpSearchResponse {
    businesses: Vec<YelpBusiness>,
}

#[derive(Debug, Deserialize)]
struct YelpBusiness {
    id: String,
    name: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    rating: f64,
    #[serde(default)]
    review_count: i64,
    #[serde(default)]
    price: String,
    #[serde(default)]
    display_phone: String,
    #[serde(default)]
    location: YelpLocation,
}

#[derive(Debug, Deserialize, Default)]
struct YelpLocation {
    #[serde(default)]
    display_address: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct YelpReviewsResponse {
    reviews: Vec<YelpReview>,
}

#[derive(Debug, Deserialize)]
struct YelpReview {
    id: String,
    #[serde(default)]
    rating: i64,
    #[serde(default)]
    text: String,
    #[serde(default)]
    time_created: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    user: YelpUser,
}

#[derive(Debug, Deserialize, Default)]
struct YelpUser {
    #[serde(default)]
    name: String,
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
        Commands::Reviews(args) => cmd_reviews(args, &cli.global),
        Commands::Config(args) => cmd_config(args),
    }
}

fn cmd_search(args: &SearchArgs, out: &GlobalArgs) -> Result<(), AppError> {
    if args.limit == 0 || args.limit > 50 {
        return Err(AppError::InvalidArgument(
            "--limit must be between 1 and 50".to_string(),
        ));
    }

    let sort = match args.sort {
        SortBy::BestMatch => "best_match",
        SortBy::Rating => "rating",
        SortBy::ReviewCount => "review_count",
        SortBy::Distance => "distance",
    };

    let mut url = format!(
        "{}/businesses/search?location={}&limit={}&sort_by={}",
        YELP_BASE,
        urlencoding::encode(&args.location),
        args.limit,
        sort
    );

    if let Some(term) = &args.term {
        url.push_str("&term=");
        url.push_str(&urlencoding::encode(term));
    }

    let rows: YelpSearchResponse = get_json(&url, out.verbose)?;
    let items: Vec<BusinessItem> = rows.businesses.into_iter().map(map_business).collect();

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
            println!("  rating={} reviews={}", item.rating, item.review_count);
            if !item.location.is_empty() {
                println!("  {}", item.location);
            }
        }
    }

    Ok(())
}

fn cmd_show(args: &ShowArgs, out: &GlobalArgs) -> Result<(), AppError> {
    let url = format!("{}/businesses/{}", YELP_BASE, args.business_id);
    let raw: YelpBusiness = get_json(&url, out.verbose)?;
    let item = map_business(raw);

    if out.json {
        print_json(&OkItem { ok: true, item });
    } else if out.quiet {
        println!("{}", item.id);
    } else {
        println!("{} ({})", item.name, item.id);
        println!("rating: {}", item.rating);
        println!("reviews: {}", item.review_count);
        if !item.location.is_empty() {
            println!("location: {}", item.location);
        }
        if !item.phone.is_empty() {
            println!("phone: {}", item.phone);
        }
        if !item.url.is_empty() {
            println!("url: {}", item.url);
        }
    }

    Ok(())
}

fn cmd_reviews(args: &ShowArgs, out: &GlobalArgs) -> Result<(), AppError> {
    let url = format!("{}/businesses/{}/reviews", YELP_BASE, args.business_id);
    let raw: YelpReviewsResponse = get_json(&url, out.verbose)?;

    let items: Vec<ReviewItem> = raw
        .reviews
        .into_iter()
        .map(|review| ReviewItem {
            id: review.id,
            rating: review.rating,
            text: review.text,
            time_created: review.time_created,
            user_name: review.user.name,
            url: review.url,
        })
        .collect();

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
            println!("{} ({})", item.user_name, item.rating);
            if !item.text.is_empty() {
                println!("  {}", item.text.replace('\n', " "));
            }
        }
    }

    Ok(())
}

fn get_json<T: for<'de> Deserialize<'de>>(url: &str, verbose: bool) -> Result<T, AppError> {
    let cfg = load_config().map_err(|_| AppError::ConfigMissing)?;
    let key = cfg
        .api_key
        .filter(|x| !x.trim().is_empty())
        .ok_or(AppError::AuthMissing)?;

    if verbose {
        eprintln!("debug: GET {url}");
    }

    let client = Client::builder()
        .user_agent("dee-food/0.1.0 (https://dee.ink)")
        .build()
        .map_err(|_| AppError::RequestFailed)?;

    let response = client
        .get(url)
        .bearer_auth(key)
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

fn map_business(row: YelpBusiness) -> BusinessItem {
    BusinessItem {
        id: row.id,
        name: row.name,
        url: row.url,
        rating: row.rating,
        review_count: row.review_count,
        price: row.price,
        phone: row.display_phone,
        location: row.location.display_address.join(", "),
    }
}

fn cmd_config(args: &ConfigArgs) -> Result<(), AppError> {
    match &args.command {
        ConfigCommand::Set(input) => {
            let mut cfg = load_config().unwrap_or_default();
            match input.key.as_str() {
                "yelp.api-key" | "api_key" => cfg.api_key = Some(input.value.clone()),
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
                let state = cfg.api_key.as_deref().map(|_| "set").unwrap_or("missing");
                println!("api_key: {state}");
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
    path.push("dee-food");
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
