use clap::{Args, Parser, Subcommand, ValueEnum};
use quick_xml::de::from_str;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

const ARXIV_API: &str = "https://export.arxiv.org/api/query";
const S2_API: &str = "https://api.semanticscholar.org/graph/v1/paper/search";

#[derive(Debug, Parser)]
#[command(
    name = "dee-arxiv",
    version,
    about = "Academic paper search CLI",
    after_help = "EXAMPLES:\n  dee-arxiv search \"graph neural networks\" --limit 10 --json\n  dee-arxiv get 2312.12345 --json\n  dee-arxiv author \"Yann LeCun\" --limit 5 --json"
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
    Get(GetArgs),
    Author(AuthorArgs),
}

#[derive(Debug, Clone, ValueEnum)]
enum SortBy {
    Date,
    Citations,
}

#[derive(Debug, Args)]
struct SearchArgs {
    query: String,
    #[arg(long, default_value_t = 10)]
    limit: usize,
    #[arg(long, value_enum, default_value_t = SortBy::Date)]
    sort: SortBy,
    #[arg(long)]
    category: Option<String>,
}

#[derive(Debug, Args)]
struct GetArgs {
    paper_id: String,
}

#[derive(Debug, Args)]
struct AuthorArgs {
    name: String,
    #[arg(long, default_value_t = 10)]
    limit: usize,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("HTTP request failed")]
    RequestFailed,
    #[error("No paper found")]
    NotFound,
    #[error("Response parse failed")]
    ParseFailed,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::RequestFailed => "REQUEST_FAILED",
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
struct ErrorJson {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, Serialize, Clone)]
struct PaperItem {
    id: String,
    title: String,
    authors: Vec<String>,
    year: i32,
    abstract_text: String,
    url: String,
    citations: i64,
    categories: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ArxivFeed {
    #[serde(rename = "entry", default)]
    entries: Vec<ArxivEntry>,
}

#[derive(Debug, Deserialize)]
struct ArxivEntry {
    id: String,
    title: String,
    summary: String,
    published: String,
    #[serde(rename = "author", default)]
    authors: Vec<ArxivAuthor>,
    #[serde(rename = "link", default)]
    links: Vec<ArxivLink>,
    #[serde(rename = "category", default)]
    categories: Vec<ArxivCategory>,
}

#[derive(Debug, Deserialize)]
struct ArxivAuthor {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ArxivLink {
    #[serde(rename = "@href")]
    href: Option<String>,
    #[serde(rename = "@rel")]
    rel: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArxivCategory {
    #[serde(rename = "@term")]
    term: Option<String>,
}

#[derive(Debug, Deserialize)]
struct S2SearchResponse {
    #[serde(default)]
    data: Vec<S2Paper>,
}

#[derive(Debug, Deserialize)]
struct S2Paper {
    #[serde(default)]
    #[serde(rename = "citationCount")]
    citation_count: Option<i64>,
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
        Commands::Get(args) => cmd_get(args, &cli.global),
        Commands::Author(args) => cmd_author(args, &cli.global),
    }
}

fn cmd_search(args: &SearchArgs, out: &GlobalArgs) -> Result<(), AppError> {
    if args.limit == 0 || args.limit > 100 {
        return Err(AppError::InvalidArgument(
            "--limit must be between 1 and 100".to_string(),
        ));
    }

    let mut query = format!("all:{}", args.query.trim());
    if let Some(cat) = &args.category {
        query.push_str("+AND+cat:");
        query.push_str(cat.trim());
    }

    let mut items = fetch_arxiv(&query, args.limit, Some("submittedDate"), out.verbose)?;

    if matches!(args.sort, SortBy::Citations) {
        enrich_citations(&mut items, out.verbose)?;
        items.sort_by(|a, b| b.citations.cmp(&a.citations));
    }

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
            println!("{}", item.title);
            println!("  {}", item.id);
            println!("  citations={} year={}", item.citations, item.year);
        }
    }

    Ok(())
}

fn cmd_get(args: &GetArgs, out: &GlobalArgs) -> Result<(), AppError> {
    let query = format!("id_list={}", urlencoding::encode(&args.paper_id));
    let url = format!("{}?{}", ARXIV_API, query);
    let feed = fetch_feed(&url, out.verbose)?;
    let entry = feed.entries.into_iter().next().ok_or(AppError::NotFound)?;
    let mut item = map_entry(entry);

    let mut one = vec![item.clone()];
    enrich_citations(&mut one, out.verbose)?;
    item.citations = one[0].citations;

    if out.json {
        print_json(&OkItem { ok: true, item });
    } else if out.quiet {
        println!("{}", item.id);
    } else {
        println!("{}", item.title);
        println!("id: {}", item.id);
        println!("year: {}", item.year);
        println!("citations: {}", item.citations);
        println!("url: {}", item.url);
    }

    Ok(())
}

fn cmd_author(args: &AuthorArgs, out: &GlobalArgs) -> Result<(), AppError> {
    if args.limit == 0 || args.limit > 100 {
        return Err(AppError::InvalidArgument(
            "--limit must be between 1 and 100".to_string(),
        ));
    }

    let query = format!("au:{}", args.name.trim());
    let items = fetch_arxiv(&query, args.limit, Some("submittedDate"), out.verbose)?;

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
            println!("{} ({})", item.title, item.year);
        }
    }

    Ok(())
}

fn fetch_arxiv(
    search_query: &str,
    limit: usize,
    sort_by: Option<&str>,
    verbose: bool,
) -> Result<Vec<PaperItem>, AppError> {
    let mut url = format!(
        "{}?search_query={}&start=0&max_results={}",
        ARXIV_API,
        urlencoding::encode(search_query),
        limit
    );

    if let Some(sort) = sort_by {
        url.push_str("&sortBy=");
        url.push_str(sort);
        url.push_str("&sortOrder=descending");
    }

    let feed = fetch_feed(&url, verbose)?;
    Ok(feed.entries.into_iter().map(map_entry).collect())
}

fn fetch_feed(url: &str, verbose: bool) -> Result<ArxivFeed, AppError> {
    if verbose {
        eprintln!("debug: GET {url}");
    }

    let client = Client::builder()
        .user_agent("dee-arxiv/0.1.0 (https://dee.ink)")
        .build()
        .map_err(|_| AppError::RequestFailed)?;

    let text = client
        .get(url)
        .send()
        .map_err(|_| AppError::RequestFailed)?
        .error_for_status()
        .map_err(|_| AppError::RequestFailed)?
        .text()
        .map_err(|_| AppError::ParseFailed)?;

    from_str(&text).map_err(|_| AppError::ParseFailed)
}

fn map_entry(entry: ArxivEntry) -> PaperItem {
    let id = entry.id.rsplit('/').next().unwrap_or(&entry.id).to_string();
    let year = entry
        .published
        .get(0..4)
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(0);
    let url = entry
        .links
        .iter()
        .find(|x| x.rel.as_deref() == Some("alternate"))
        .and_then(|x| x.href.clone())
        .unwrap_or_default();

    PaperItem {
        id,
        title: normalize_whitespace(&entry.title),
        authors: entry.authors.into_iter().map(|a| a.name).collect(),
        year,
        abstract_text: normalize_whitespace(&entry.summary),
        url,
        citations: 0,
        categories: entry
            .categories
            .into_iter()
            .filter_map(|c| c.term)
            .collect(),
    }
}

fn enrich_citations(items: &mut [PaperItem], verbose: bool) -> Result<(), AppError> {
    if items.is_empty() {
        return Ok(());
    }

    let client = Client::builder()
        .user_agent("dee-arxiv/0.1.0 (https://dee.ink)")
        .build()
        .map_err(|_| AppError::RequestFailed)?;

    for item in items {
        let url = format!(
            "{}?query={}&limit=1&fields=citationCount",
            S2_API,
            urlencoding::encode(&item.title)
        );

        if verbose {
            eprintln!("debug: GET {url}");
        }

        let res = client.get(&url).send();
        let Ok(resp) = res else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(parsed) = resp.json::<S2SearchResponse>() else {
            continue;
        };
        let citations = parsed
            .data
            .first()
            .and_then(|x| x.citation_count)
            .unwrap_or(0);
        item.citations = citations;
    }

    Ok(())
}

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
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
