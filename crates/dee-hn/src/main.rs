use anyhow::{anyhow, Context, Result};
use chrono::{TimeZone, Utc};
use clap::{Args, Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const HN_BASE: &str = "https://hacker-news.firebaseio.com/v0";
const ALGOLIA_BASE: &str = "https://hn.algolia.com/api/v1";

#[derive(Parser, Debug)]
#[command(
    name = "dee-hn",
    version,
    about = "Browse Hacker News stories, items, and comments",
    after_help = "EXAMPLES:\n  dee-hn top --limit 10\n  dee-hn new --json\n  dee-hn search \"rust async\" --limit 5 --json\n  dee-hn item 8863 --json\n  dee-hn comments 8863 --depth 2 --json\n  dee-hn user pg --json"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true, help = "Output as JSON")]
    json: bool,

    #[arg(short, long, global = true, help = "Suppress decorative output")]
    quiet: bool,

    #[arg(short, long, global = true, help = "Debug output to stderr")]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Top(ListArgs),
    New(ListArgs),
    Best(ListArgs),
    Ask(ListArgs),
    Show(ListArgs),
    Jobs(ListArgs),
    Search(SearchArgs),
    Item(ItemArgs),
    Comments(CommentsArgs),
    /// Look up a Hacker News user profile
    User(UserArgs),
}

#[derive(Args, Debug)]
struct UserArgs {
    /// HN username
    id: String,
}

#[derive(Args, Debug)]
struct ListArgs {
    #[arg(long, default_value_t = 30)]
    limit: usize,
}

#[derive(Args, Debug)]
struct SearchArgs {
    query: String,
    #[arg(long, default_value_t = 20)]
    limit: usize,
}

#[derive(Args, Debug)]
struct ItemArgs {
    id: u64,
}

#[derive(Args, Debug)]
struct CommentsArgs {
    id: u64,
    #[arg(long, default_value_t = 2)]
    depth: usize,
}

#[derive(Debug, Deserialize)]
struct HnItem {
    id: u64,
    #[serde(rename = "type")]
    item_type: Option<String>,
    by: Option<String>,
    time: Option<i64>,
    title: Option<String>,
    text: Option<String>,
    url: Option<String>,
    score: Option<i64>,
    descendants: Option<u64>,
    kids: Option<Vec<u64>>,
    dead: Option<bool>,
    deleted: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AlgoliaResponse {
    hits: Vec<AlgoliaHit>,
}

#[derive(Debug, Deserialize)]
struct AlgoliaHit {
    #[serde(rename = "objectID")]
    object_id: String,
    title: Option<String>,
    url: Option<String>,
    points: Option<i64>,
    author: Option<String>,
    num_comments: Option<u64>,
    created_at_i: Option<i64>,
}

#[derive(Debug, Serialize)]
struct JsonList<T> {
    ok: bool,
    count: usize,
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct JsonItem<T> {
    ok: bool,
    item: T,
}

#[derive(Debug, Serialize)]
struct JsonError {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, Serialize)]
struct StoryOut {
    id: u64,
    item_type: String,
    title: String,
    by: String,
    score: i64,
    comments: u64,
    time: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    url: String,
}

#[derive(Debug, Serialize)]
struct ItemOut {
    id: u64,
    item_type: String,
    by: String,
    time: String,
    title: String,
    text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    url: String,
    score: i64,
    comments: u64,
    kids_count: usize,
}

#[derive(Debug, Serialize)]
struct CommentOut {
    id: u64,
    by: String,
    time: String,
    text: String,
    depth: usize,
    kids_count: usize,
}

#[derive(Debug, Deserialize)]
struct HnUser {
    id: String,
    #[serde(default)]
    karma: i64,
    #[serde(default)]
    about: String,
    created: Option<i64>,
    #[serde(default)]
    submitted: Vec<u64>,
}

#[derive(Debug, Serialize)]
struct UserOut {
    id: String,
    karma: i64,
    about: String,
    created_at: String,
    submissions: usize,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = run(&cli).await;

    if let Err(err) = result {
        if cli.json {
            let payload = JsonError {
                ok: false,
                error: err.to_string(),
                code: classify_error(&err),
            };
            match serde_json::to_string_pretty(&payload) {
                Ok(s) => println!("{s}"),
                Err(_) => println!("{{\"ok\":false,\"error\":\"serialization failure\",\"code\":\"INTERNAL_ERROR\"}}"),
            }
        } else {
            eprintln!("error: {err}");
        }
        std::process::exit(1);
    }
}

async fn run(cli: &Cli) -> Result<()> {
    let client = Client::builder()
        .user_agent("dee-hn/0.1.0")
        .build()
        .context("failed to initialize HTTP client")?;

    match &cli.command {
        Commands::Top(args) => list_stories(&client, "topstories", args.limit, cli).await,
        Commands::New(args) => list_stories(&client, "newstories", args.limit, cli).await,
        Commands::Best(args) => list_stories(&client, "beststories", args.limit, cli).await,
        Commands::Ask(args) => list_stories(&client, "askstories", args.limit, cli).await,
        Commands::Show(args) => list_stories(&client, "showstories", args.limit, cli).await,
        Commands::Jobs(args) => list_stories(&client, "jobstories", args.limit, cli).await,
        Commands::Search(args) => search_stories(&client, &args.query, args.limit, cli).await,
        Commands::Item(args) => show_item(&client, args.id, cli).await,
        Commands::Comments(args) => show_comments(&client, args.id, args.depth, cli).await,
        Commands::User(args) => show_user(&client, &args.id, cli).await,
    }
}

async fn list_stories(client: &Client, endpoint: &str, limit: usize, cli: &Cli) -> Result<()> {
    let ids_url = format!("{HN_BASE}/{endpoint}.json");
    let ids: Vec<u64> = client
        .get(&ids_url)
        .send()
        .await
        .with_context(|| format!("failed request to {ids_url}"))?
        .error_for_status()
        .with_context(|| format!("request failed for {ids_url}"))?
        .json()
        .await
        .context("failed to decode story id list")?;

    let mut stories = Vec::new();
    for id in ids.into_iter().take(limit) {
        let item = fetch_item(client, id).await?;
        if item.item_type.as_deref() == Some("story") || endpoint == "jobstories" {
            stories.push(to_story_out(item));
        }
    }

    if cli.json {
        print_json(&JsonList {
            ok: true,
            count: stories.len(),
            items: stories,
        })?;
    } else {
        if !cli.quiet {
            println!("Found {} stories", stories.len());
        }
        for story in stories {
            let url_part = if story.url.is_empty() {
                String::new()
            } else {
                format!(" | {}", story.url)
            };
            println!(
                "{} [{}] by {} | {} pts | {} comments | {}{}",
                story.id,
                story.item_type,
                story.by,
                story.score,
                story.comments,
                story.time,
                url_part
            );
            println!("  {}", story.title);
        }
    }

    Ok(())
}

async fn search_stories(client: &Client, query: &str, limit: usize, cli: &Cli) -> Result<()> {
    let url = format!("{ALGOLIA_BASE}/search");
    let response: AlgoliaResponse = client
        .get(url)
        .query(&[
            ("query", query),
            ("tags", "story"),
            ("hitsPerPage", &limit.to_string()),
        ])
        .send()
        .await
        .context("failed request to Algolia search")?
        .error_for_status()
        .context("Algolia search request failed")?
        .json()
        .await
        .context("failed to decode Algolia response")?;

    let items: Vec<StoryOut> = response
        .hits
        .into_iter()
        .filter_map(|hit| {
            let id = hit.object_id.parse::<u64>().ok()?;
            Some(StoryOut {
                id,
                item_type: "story".to_owned(),
                title: hit.title.unwrap_or_default(),
                by: hit.author.unwrap_or_default(),
                score: hit.points.unwrap_or(0),
                comments: hit.num_comments.unwrap_or(0),
                time: iso_time(hit.created_at_i.unwrap_or(0)),
                url: hit.url.unwrap_or_default(),
            })
        })
        .collect();

    if cli.json {
        print_json(&JsonList {
            ok: true,
            count: items.len(),
            items,
        })?;
    } else {
        if !cli.quiet {
            println!("Found {} stories for \"{}\"", items.len(), query);
        }
        for story in items {
            println!(
                "{} by {} | {} pts | {} comments | {}",
                story.id, story.by, story.score, story.comments, story.time
            );
            println!("  {}", story.title);
            if !story.url.is_empty() {
                println!("  {}", story.url);
            }
        }
    }

    Ok(())
}

async fn show_item(client: &Client, id: u64, cli: &Cli) -> Result<()> {
    let item = fetch_item(client, id).await?;
    let out = to_item_out(item);

    if cli.json {
        print_json(&JsonItem {
            ok: true,
            item: out,
        })?;
    } else {
        println!("id: {}", out.id);
        println!("type: {}", out.item_type);
        println!("by: {}", out.by);
        println!("time: {}", out.time);
        if !out.title.is_empty() {
            println!("title: {}", out.title);
        }
        if !out.url.is_empty() {
            println!("url: {}", out.url);
        }
        println!("score: {}", out.score);
        println!("comments: {}", out.comments);
        if !out.text.is_empty() {
            println!("text: {}", out.text);
        }
    }

    Ok(())
}

async fn show_comments(client: &Client, id: u64, max_depth: usize, cli: &Cli) -> Result<()> {
    let root = fetch_item(client, id).await?;
    let kids = root.kids.unwrap_or_default();

    let mut comments = Vec::new();
    let mut stack: Vec<(u64, usize)> = kids.into_iter().rev().map(|kid| (kid, 1usize)).collect();

    while let Some((comment_id, depth)) = stack.pop() {
        let item = fetch_item(client, comment_id).await?;
        if item.item_type.as_deref() == Some("comment")
            && item.deleted != Some(true)
            && item.dead != Some(true)
        {
            let child_kids = item.kids.clone().unwrap_or_default();
            comments.push(CommentOut {
                id: item.id,
                by: item.by.unwrap_or_default(),
                time: iso_time(item.time.unwrap_or(0)),
                text: item.text.unwrap_or_default(),
                depth,
                kids_count: child_kids.len(),
            });

            if depth < max_depth {
                for kid in child_kids.into_iter().rev() {
                    stack.push((kid, depth + 1));
                }
            }
        }
    }

    if cli.json {
        print_json(&JsonList {
            ok: true,
            count: comments.len(),
            items: comments,
        })?;
    } else {
        if !cli.quiet {
            println!("Comments: {}", comments.len());
        }
        for c in comments {
            let indent = "  ".repeat(c.depth.saturating_sub(1));
            println!("{}#{} by {} at {}", indent, c.id, c.by, c.time);
            println!("{}{}", indent, c.text.replace('\n', " "));
        }
    }

    Ok(())
}

async fn show_user(client: &Client, id: &str, cli: &Cli) -> Result<()> {
    let url = format!("{HN_BASE}/user/{id}.json");
    let maybe_user: Option<HnUser> = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("failed request to {url}"))?
        .error_for_status()
        .with_context(|| format!("request failed for {url}"))?
        .json()
        .await
        .with_context(|| format!("failed to decode user {id}"))?;

    let user = maybe_user.ok_or_else(|| anyhow!("user {id} not found"))?;
    let out = UserOut {
        id: user.id,
        karma: user.karma,
        about: user.about,
        created_at: iso_time(user.created.unwrap_or(0)),
        submissions: user.submitted.len(),
    };

    if cli.json {
        print_json(&JsonItem {
            ok: true,
            item: out,
        })?;
    } else {
        println!("id: {}", out.id);
        println!("karma: {}", out.karma);
        println!("created_at: {}", out.created_at);
        println!("submissions: {}", out.submissions);
        if !out.about.is_empty() {
            println!("about: {}", out.about);
        }
    }

    Ok(())
}

async fn fetch_item(client: &Client, id: u64) -> Result<HnItem> {
    let url = format!("{HN_BASE}/item/{id}.json");
    let maybe_item: Option<HnItem> = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("failed request to {url}"))?
        .error_for_status()
        .with_context(|| format!("request failed for {url}"))?
        .json()
        .await
        .with_context(|| format!("failed to decode item {id}"))?;

    maybe_item.ok_or_else(|| anyhow!("item {id} not found"))
}

fn to_story_out(item: HnItem) -> StoryOut {
    StoryOut {
        id: item.id,
        item_type: item.item_type.unwrap_or_else(|| "unknown".to_owned()),
        title: item.title.unwrap_or_default(),
        by: item.by.unwrap_or_default(),
        score: item.score.unwrap_or(0),
        comments: item.descendants.unwrap_or(0),
        time: iso_time(item.time.unwrap_or(0)),
        url: item.url.unwrap_or_default(),
    }
}

fn to_item_out(item: HnItem) -> ItemOut {
    let kids = item.kids.unwrap_or_default();
    ItemOut {
        id: item.id,
        item_type: item.item_type.unwrap_or_else(|| "unknown".to_owned()),
        by: item.by.unwrap_or_default(),
        time: iso_time(item.time.unwrap_or(0)),
        title: item.title.unwrap_or_default(),
        text: item.text.unwrap_or_default(),
        url: item.url.unwrap_or_default(),
        score: item.score.unwrap_or(0),
        comments: item.descendants.unwrap_or(0),
        kids_count: kids.len(),
    }
}

fn iso_time(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "1970-01-01T00:00:00+00:00".to_owned())
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let rendered = serde_json::to_string_pretty(value).context("failed to serialize JSON")?;
    println!("{rendered}");
    Ok(())
}

fn classify_error(err: &anyhow::Error) -> String {
    let lower = err.to_string().to_lowercase();
    if lower.contains("not found") {
        "NOT_FOUND".to_owned()
    } else if lower.contains("request") || lower.contains("network") || lower.contains("timeout") {
        "NETWORK_ERROR".to_owned()
    } else if lower.contains("decode") || lower.contains("serialize") || lower.contains("json") {
        "PARSE_ERROR".to_owned()
    } else {
        "INTERNAL_ERROR".to_owned()
    }
}
