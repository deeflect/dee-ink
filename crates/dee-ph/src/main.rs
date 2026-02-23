use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

const PH_API_URL: &str = "https://api.producthunt.com/v2/api/graphql";

#[derive(Debug, Parser)]
#[command(
    name = "dee-ph",
    version,
    about = "Product Hunt CLI",
    after_help = "EXAMPLES:\n  dee-ph top --limit 10\n  dee-ph search ai --json\n  dee-ph show chatgpt --json\n  dee-ph config set ph.api-key <TOKEN>\n  dee-ph config show --json\n  dee-ph config path"
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
    /// Top Product Hunt posts
    Top(TopArgs),
    /// Search posts by topic/query
    Search(SearchArgs),
    /// Show one post by slug
    Show(ShowArgs),
    /// Manage config
    Config(ConfigArgs),
}

#[derive(Debug, Args)]
struct TopArgs {
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = TopOrder::Votes)]
    order: TopOrder,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TopOrder {
    Votes,
    Newest,
}

#[derive(Debug, Args)]
struct SearchArgs {
    topic: String,
    #[arg(long, default_value_t = 20)]
    limit: usize,
}

#[derive(Debug, Args)]
struct ShowArgs {
    product_slug: String,
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
    slug: String,
    name: String,
    tagline: String,
    votes_count: i64,
    comments_count: i64,
    website: String,
    url: String,
    created_at: String,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Configuration directory not found")]
    ConfigMissing,
    #[error("Missing Product Hunt API key. Set ph.api-key via config set")]
    AuthMissing,
    #[error("Unknown config key: {0}")]
    InvalidConfigKey(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("HTTP request failed")]
    RequestFailed,
    #[error("Product Hunt API returned an error")]
    ApiError,
    #[error("No product found")]
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

#[derive(Debug, Deserialize)]
struct GqlRoot<T> {
    data: Option<T>,
    errors: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct TopData {
    posts: EdgeList<PostNode>,
}

#[derive(Debug, Deserialize)]
struct SearchData {
    posts: EdgeList<PostNode>,
}

#[derive(Debug, Deserialize)]
struct ShowData {
    post: Option<PostNode>,
}

#[derive(Debug, Deserialize)]
struct EdgeList<T> {
    edges: Vec<Edge<T>>,
}

#[derive(Debug, Deserialize)]
struct Edge<T> {
    node: T,
}

#[derive(Debug, Deserialize)]
struct PostNode {
    id: String,
    slug: String,
    name: String,
    #[serde(default)]
    tagline: String,
    #[serde(default)]
    #[serde(rename = "votesCount")]
    votes_count: i64,
    #[serde(default)]
    #[serde(rename = "commentsCount")]
    comments_count: i64,
    #[serde(default)]
    website: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    #[serde(rename = "createdAt")]
    created_at: String,
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
        Commands::Top(args) => cmd_top(args, &cli.global),
        Commands::Search(args) => cmd_search(args, &cli.global),
        Commands::Show(args) => cmd_show(args, &cli.global),
        Commands::Config(args) => cmd_config(args),
    }
}

fn cmd_top(args: &TopArgs, out: &GlobalArgs) -> Result<(), AppError> {
    if args.limit == 0 {
        return Err(AppError::InvalidArgument("--limit must be > 0".to_string()));
    }

    let order = match args.order {
        TopOrder::Votes => "VOTES",
        TopOrder::Newest => "NEWEST",
    };

    let query = r#"query TopPosts($first: Int!, $order: PostsOrder!) {
  posts(first: $first, order: $order) {
    edges {
      node {
        id slug name tagline votesCount commentsCount website url createdAt
      }
    }
  }
}"#;

    let vars = json!({"first": args.limit as i64, "order": order});
    let data: TopData = gql_request(query, vars, out.verbose)?;
    let items = map_posts(data.posts.edges.into_iter().map(|x| x.node).collect());

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
            println!("{} ({})", item.name, item.slug);
            println!(
                "  votes={} comments={}",
                item.votes_count, item.comments_count
            );
            if !item.tagline.is_empty() {
                println!("  {}", item.tagline);
            }
            if !item.url.is_empty() {
                println!("  {}", item.url);
            }
        }
    }

    Ok(())
}

fn cmd_search(args: &SearchArgs, out: &GlobalArgs) -> Result<(), AppError> {
    if args.limit == 0 {
        return Err(AppError::InvalidArgument("--limit must be > 0".to_string()));
    }

    let query = r#"query SearchPosts($query: String!, $first: Int!) {
  posts(query: $query, first: $first) {
    edges {
      node {
        id slug name tagline votesCount commentsCount website url createdAt
      }
    }
  }
}"#;

    let vars = json!({"query": args.topic, "first": args.limit as i64});
    let data: SearchData = gql_request(query, vars, out.verbose)?;
    let items = map_posts(data.posts.edges.into_iter().map(|x| x.node).collect());

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
            println!("{} ({})", item.name, item.slug);
            if !item.tagline.is_empty() {
                println!("  {}", item.tagline);
            }
            if !item.url.is_empty() {
                println!("  {}", item.url);
            }
        }
    }

    Ok(())
}

fn cmd_show(args: &ShowArgs, out: &GlobalArgs) -> Result<(), AppError> {
    let query = r#"query GetPost($slug: String!) {
  post(slug: $slug) {
    id slug name tagline votesCount commentsCount website url createdAt
  }
}"#;

    let vars = json!({"slug": args.product_slug});
    let data: ShowData = gql_request(query, vars, out.verbose)?;
    let post = data.post.ok_or(AppError::NotFound)?;
    let item = map_post(post);

    if out.json {
        print_json(&OkItem { ok: true, item });
    } else if out.quiet {
        println!("{}", item.slug);
    } else {
        println!("{} ({})", item.name, item.slug);
        println!("votes: {}", item.votes_count);
        println!("comments: {}", item.comments_count);
        if !item.tagline.is_empty() {
            println!("tagline: {}", item.tagline);
        }
        if !item.website.is_empty() {
            println!("website: {}", item.website);
        }
        if !item.url.is_empty() {
            println!("url: {}", item.url);
        }
        if !item.created_at.is_empty() {
            println!("created_at: {}", item.created_at);
        }
    }

    Ok(())
}

fn cmd_config(args: &ConfigArgs) -> Result<(), AppError> {
    match &args.command {
        ConfigCommand::Set(input) => {
            let mut cfg = load_config().unwrap_or_default();
            match input.key.as_str() {
                "ph.api-key" | "api_key" => cfg.api_key = Some(input.value.clone()),
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
                let key_set = cfg.api_key.as_deref().map(|_| "set").unwrap_or("missing");
                println!("api_key: {key_set}");
            }
            Ok(())
        }
        ConfigCommand::Path => {
            println!("{}", config_path().display());
            Ok(())
        }
    }
}

fn gql_request<T: for<'de> Deserialize<'de>>(
    query: &str,
    variables: serde_json::Value,
    verbose: bool,
) -> Result<T, AppError> {
    let cfg = load_config().map_err(|_| AppError::ConfigMissing)?;
    let token = cfg
        .api_key
        .filter(|x| !x.trim().is_empty())
        .ok_or(AppError::AuthMissing)?;

    if verbose {
        eprintln!("debug: POST {PH_API_URL}");
    }

    let client = Client::builder()
        .user_agent("dee-ph/0.1.0 (https://dee.ink)")
        .build()
        .map_err(|_| AppError::RequestFailed)?;

    let root: GqlRoot<T> = client
        .post(PH_API_URL)
        .bearer_auth(token)
        .json(&json!({"query": query, "variables": variables}))
        .send()
        .map_err(|_| AppError::RequestFailed)?
        .error_for_status()
        .map_err(|_| AppError::RequestFailed)?
        .json()
        .map_err(|_| AppError::ParseFailed)?;

    if root.errors.as_ref().is_some_and(|errs| !errs.is_empty()) {
        return Err(AppError::ApiError);
    }

    root.data.ok_or(AppError::ParseFailed)
}

fn map_posts(posts: Vec<PostNode>) -> Vec<ProductItem> {
    posts.into_iter().map(map_post).collect()
}

fn map_post(node: PostNode) -> ProductItem {
    ProductItem {
        id: node.id,
        slug: node.slug,
        name: node.name,
        tagline: node.tagline,
        votes_count: node.votes_count,
        comments_count: node.comments_count,
        website: node.website,
        url: node.url,
        created_at: node.created_at,
    }
}

fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("dee-ph");
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
