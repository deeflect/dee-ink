use std::fs;
use std::path::PathBuf;

use base64::Engine as _;
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "dee-reddit",
    version,
    about = "Search Reddit posts and browse subreddits",
    long_about = "dee-reddit - Query Reddit OAuth APIs with agent-friendly JSON output.",
    after_help = "EXAMPLES:\n  dee-reddit config set reddit.client-id <ID>\n  dee-reddit config set reddit.client-secret <SECRET>\n  dee-reddit config set reddit.user-agent \"dee-reddit/0.1 by u/yourname\"\n  dee-reddit search \"rust async\" --limit 10 --json\n  dee-reddit subreddit rust --sort top --limit 10 --json"
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
    Subreddit(SubredditArgs),
    Config(ConfigArgs),
}

#[derive(Debug, Clone, ValueEnum)]
enum SortArg {
    Relevance,
    Hot,
    Top,
    New,
    Comments,
}

impl SortArg {
    fn as_api_value(&self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Hot => "hot",
            Self::Top => "top",
            Self::New => "new",
            Self::Comments => "comments",
        }
    }
}

#[derive(Debug, Args)]
struct SearchArgs {
    query: String,

    #[arg(long, default_value_t = 20)]
    limit: usize,

    #[arg(long, value_enum, default_value_t = SortArg::Relevance)]
    sort: SortArg,
}

#[derive(Debug, Args)]
struct SubredditArgs {
    name: String,

    #[arg(long, default_value_t = 20)]
    limit: usize,

    #[arg(long, value_enum, default_value_t = SortArg::Hot)]
    sort: SortArg,
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
    user_agent: Option<String>,
    #[serde(default)]
    oauth_base_url: Option<String>,
    #[serde(default)]
    api_base_url: Option<String>,
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
struct PostItem {
    id: String,
    title: String,
    subreddit: String,
    author: String,
    score: i64,
    comments: i64,
    nsfw: bool,
    created_utc: f64,
    permalink: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct ListingResponse {
    data: ListingData,
}

#[derive(Debug, Deserialize)]
struct ListingData {
    #[serde(default)]
    children: Vec<Child>,
}

#[derive(Debug, Deserialize)]
struct Child {
    data: PostData,
}

#[derive(Debug, Deserialize)]
struct PostData {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    subreddit: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    score: i64,
    #[serde(default)]
    num_comments: i64,
    #[serde(default)]
    over_18: bool,
    #[serde(default)]
    created_utc: f64,
    #[serde(default)]
    permalink: String,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Configuration directory not found")]
    ConfigMissing,
    #[error("Missing Reddit credentials. Set reddit.client-id and reddit.client-secret")]
    AuthMissing,
    #[error("Missing Reddit user-agent. Set reddit.user-agent")]
    UserAgentMissing,
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
            Self::AuthMissing | Self::UserAgentMissing => "AUTH_MISSING",
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
        Commands::Subreddit(args) => cmd_subreddit(args, &cli.global),
        Commands::Config(args) => cmd_config(args, &cli.global),
    }
}

fn cmd_search(args: &SearchArgs, global: &GlobalArgs) -> AppResult<()> {
    validate_query(&args.query)?;
    validate_limit(args.limit)?;

    let config = load_config_or_default()?;
    let auth = read_auth_from_config(&config)?;

    let token = fetch_access_token(&auth)?;

    let url = format!(
        "{}/search.json?q={}&sort={}&limit={}&type=link",
        auth.api_base,
        urlencoding::encode(args.query.trim()),
        args.sort.as_api_value(),
        args.limit
    );

    let items = fetch_listing(&url, &token, &auth.user_agent)?;
    render_items(items, global)
}

fn cmd_subreddit(args: &SubredditArgs, global: &GlobalArgs) -> AppResult<()> {
    validate_query(&args.name)?;
    validate_limit(args.limit)?;

    let config = load_config_or_default()?;
    let auth = read_auth_from_config(&config)?;

    let token = fetch_access_token(&auth)?;

    let endpoint = match args.sort {
        SortArg::Top => "top",
        SortArg::New => "new",
        SortArg::Comments => "new",
        SortArg::Relevance | SortArg::Hot => "hot",
    };

    let url = format!(
        "{}/r/{}/{}.json?limit={}",
        auth.api_base,
        urlencoding::encode(args.name.trim()),
        endpoint,
        args.limit
    );

    let mut items = fetch_listing(&url, &token, &auth.user_agent)?;

    if matches!(args.sort, SortArg::Comments) {
        items.sort_by(|a, b| b.comments.cmp(&a.comments));
    }

    render_items(items, global)
}

fn render_items(items: Vec<PostItem>, global: &GlobalArgs) -> AppResult<()> {
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
        println!("No posts found.");
    } else {
        for item in &items {
            println!(
                "{} | r/{} | {} | score {}",
                item.id, item.subreddit, item.title, item.score
            );
        }
    }

    Ok(())
}

struct AuthConfig {
    oauth_base: String,
    api_base: String,
    client_id: String,
    client_secret: String,
    user_agent: String,
}

fn read_auth_from_config(config: &AppConfig) -> AppResult<AuthConfig> {
    let client_id = config.client_id.clone().ok_or(AppError::AuthMissing)?;
    let client_secret = config.client_secret.clone().ok_or(AppError::AuthMissing)?;
    let user_agent = config
        .user_agent
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or(AppError::UserAgentMissing)?;

    let oauth_base = config
        .oauth_base_url
        .clone()
        .unwrap_or_else(|| "https://www.reddit.com/api/v1".to_string());

    let api_base = config
        .api_base_url
        .clone()
        .unwrap_or_else(|| "https://oauth.reddit.com".to_string());

    Ok(AuthConfig {
        oauth_base,
        api_base,
        client_id,
        client_secret,
        user_agent,
    })
}

fn fetch_access_token(auth: &AuthConfig) -> AppResult<String> {
    let basic = base64::engine::general_purpose::STANDARD.encode(format!(
        "{}:{}",
        auth.client_id.trim(),
        auth.client_secret.trim()
    ));

    let endpoint = format!("{}/access_token", auth.oauth_base.trim_end_matches('/'));

    let client = Client::new();
    let response = client
        .post(endpoint)
        .header(AUTHORIZATION, format!("Basic {basic}"))
        .header(USER_AGENT, auth.user_agent.clone())
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body("grant_type=client_credentials")
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

fn fetch_listing(url: &str, token: &str, user_agent: &str) -> AppResult<Vec<PostItem>> {
    let client = Client::new();
    let response = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .header(USER_AGENT, user_agent)
        .send()
        .map_err(|_| AppError::RequestFailed)?;

    let status = response.status();
    let body = response.text().map_err(|_| AppError::RequestFailed)?;

    if !status.is_success() {
        return Err(AppError::ApiError(format!("HTTP {}", status.as_u16())));
    }

    let parsed: ListingResponse = serde_json::from_str(&body).map_err(|_| AppError::ParseFailed)?;

    Ok(parsed
        .data
        .children
        .into_iter()
        .map(|child| {
            let post = child.data;
            PostItem {
                id: post.id,
                title: post.title,
                subreddit: post.subreddit,
                author: post.author,
                score: post.score,
                comments: post.num_comments,
                nsfw: post.over_18,
                created_utc: post.created_utc,
                permalink: post.permalink,
                url: post.url,
            }
        })
        .collect())
}

fn validate_query(query: &str) -> AppResult<()> {
    if query.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "query cannot be empty".to_string(),
        ));
    }
    Ok(())
}

fn validate_limit(limit: usize) -> AppResult<()> {
    if !(1..=100).contains(&limit) {
        return Err(AppError::InvalidArgument(
            "limit must be between 1 and 100".to_string(),
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
        "reddit.client-id" => {
            ensure_not_empty(&args.value)?;
            config.client_id = Some(args.value.clone());
        }
        "reddit.client-secret" => {
            ensure_not_empty(&args.value)?;
            config.client_secret = Some(args.value.clone());
        }
        "reddit.user-agent" => {
            ensure_not_empty(&args.value)?;
            config.user_agent = Some(args.value.clone());
        }
        "reddit.oauth-base-url" => {
            ensure_not_empty(&args.value)?;
            config.oauth_base_url = Some(args.value.clone());
        }
        "reddit.api-base-url" => {
            ensure_not_empty(&args.value)?;
            config.api_base_url = Some(args.value.clone());
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
        println!("client_id={}", config.client_id.unwrap_or_default());
        println!("client_secret={}", config.client_secret.unwrap_or_default());
        println!("user_agent={}", config.user_agent.unwrap_or_default());
        println!(
            "oauth_base_url={}",
            config.oauth_base_url.unwrap_or_default()
        );
        println!("api_base_url={}", config.api_base_url.unwrap_or_default());
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
    Ok(base.join("dee-reddit").join("config.toml"))
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
