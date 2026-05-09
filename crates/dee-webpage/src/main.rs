use clap::{Args, Parser, Subcommand};
use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;
use reqwest::{StatusCode, Url};
use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::time::Duration;

const USER_AGENT: &str = concat!(
    "dee-webpage/",
    env!("CARGO_PKG_VERSION"),
    " (https://dee.ink)"
);
const DEFAULT_MAX_BYTES: usize = 2_000_000;
const DEFAULT_MAX_CHARS: usize = 20_000;
const DEFAULT_LINK_LIMIT: usize = 200;

#[derive(Debug, Parser)]
#[command(
    name = "dee-webpage",
    version,
    about = "Fetch webpage metadata, text, and links",
    after_help = "EXAMPLES:\n  dee-webpage metadata https://example.com --json\n  dee-webpage text https://example.com --max-chars 4000 --json\n  dee-webpage markdown https://example.com --json\n  dee-webpage links https://example.com --limit 50 --json\n  dee-webpage text https://example.com --quiet"
)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Args)]
struct GlobalArgs {
    #[arg(short = 'j', long, global = true, help = "Output as JSON")]
    json: bool,
    #[arg(short = 'q', long, global = true, help = "Suppress decorative output")]
    quiet: bool,
    #[arg(short = 'v', long, global = true, help = "Debug output to stderr")]
    verbose: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Fetch status, title, meta tags, headings, and counts
    Metadata(FetchArgs),
    /// Extract clean readable text from article/main/body
    Text(TextArgs),
    /// Extract simple Markdown from article/main/body
    Markdown(TextArgs),
    /// Extract HTTP links from a page
    Links(LinksArgs),
}

#[derive(Debug, Clone, Args)]
struct FetchArgs {
    /// URL to fetch
    url: String,
    /// Request timeout in seconds
    #[arg(long, default_value_t = 20)]
    timeout_secs: u64,
    /// Maximum response body size in bytes
    #[arg(long, default_value_t = DEFAULT_MAX_BYTES)]
    max_bytes: usize,
}

#[derive(Debug, Clone, Args)]
struct TextArgs {
    /// URL to fetch
    url: String,
    /// Optional CSS selector to extract instead of article/main/body
    #[arg(long)]
    selector: Option<String>,
    /// Maximum text characters to return
    #[arg(long, default_value_t = DEFAULT_MAX_CHARS)]
    max_chars: usize,
    /// Request timeout in seconds
    #[arg(long, default_value_t = 20)]
    timeout_secs: u64,
    /// Maximum response body size in bytes
    #[arg(long, default_value_t = DEFAULT_MAX_BYTES)]
    max_bytes: usize,
}

#[derive(Debug, Clone, Args)]
struct LinksArgs {
    /// URL to fetch
    url: String,
    /// Maximum links to return
    #[arg(long, default_value_t = DEFAULT_LINK_LIMIT)]
    limit: usize,
    /// Only show links on the same host
    #[arg(long)]
    internal: bool,
    /// Only show links on a different host
    #[arg(long)]
    external: bool,
    /// Request timeout in seconds
    #[arg(long, default_value_t = 20)]
    timeout_secs: u64,
    /// Maximum response body size in bytes
    #[arg(long, default_value_t = DEFAULT_MAX_BYTES)]
    max_bytes: usize,
}

#[derive(Debug, Serialize)]
struct JsonItem<T> {
    ok: bool,
    item: T,
}

#[derive(Debug, Serialize)]
struct JsonList<T> {
    ok: bool,
    count: usize,
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct JsonError {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, Serialize)]
struct MetadataItem {
    url: String,
    final_url: String,
    status: u16,
    content_type: String,
    bytes: usize,
    content_sha256: String,
    title: String,
    description: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    canonical_url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    lang: String,
    headings_count: usize,
    headings: Vec<HeadingItem>,
    links_count: usize,
    images_count: usize,
}

#[derive(Debug, Serialize, Clone)]
struct HeadingItem {
    level: u8,
    text: String,
}

#[derive(Debug, Serialize)]
struct TextItem {
    url: String,
    final_url: String,
    title: String,
    selector: String,
    text: String,
    chars: usize,
    truncated: bool,
    content_sha256: String,
}

#[derive(Debug, Serialize)]
struct MarkdownItem {
    url: String,
    final_url: String,
    title: String,
    selector: String,
    markdown: String,
    chars: usize,
    truncated: bool,
    content_sha256: String,
}

#[derive(Debug, Serialize, Clone)]
struct LinkItem {
    source_url: String,
    url: String,
    text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    rel: String,
    internal: bool,
}

#[derive(Debug)]
struct FetchedPage {
    requested_url: String,
    final_url: Url,
    status: StatusCode,
    content_type: String,
    bytes: Vec<u8>,
    content_sha256: String,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("HTTP request returned status {0}")]
    HttpStatus(u16),
    #[error("Response body exceeded --max-bytes ({0})")]
    ResponseTooLarge(usize),
    #[error("Response parse failed: {0}")]
    ParseFailed(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::RequestFailed(_) => "REQUEST_FAILED",
            Self::HttpStatus(_) => "HTTP_STATUS",
            Self::ResponseTooLarge(_) => "RESPONSE_TOO_LARGE",
            Self::ParseFailed(_) => "PARSE_FAILED",
            Self::Internal(_) => "INTERNAL",
        }
    }
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            if err.kind() == clap::error::ErrorKind::DisplayHelp
                || err.kind() == clap::error::ErrorKind::DisplayVersion
            {
                err.exit();
            }

            if wants_json_from_args() {
                print_json(&JsonError {
                    ok: false,
                    error: err.to_string().trim().to_string(),
                    code: "INVALID_ARGUMENT".to_string(),
                });
                std::process::exit(1);
            }

            err.exit();
        }
    };

    if let Err(err) = run(&cli) {
        if cli.global.json {
            print_json(&JsonError {
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

fn wants_json_from_args() -> bool {
    std::env::args_os().any(|arg| arg == "--json" || arg == "-j")
}

fn run(cli: &Cli) -> Result<(), AppError> {
    match &cli.command {
        Commands::Metadata(args) => cmd_metadata(args, &cli.global),
        Commands::Text(args) => cmd_text(args, &cli.global),
        Commands::Markdown(args) => cmd_markdown(args, &cli.global),
        Commands::Links(args) => cmd_links(args, &cli.global),
    }
}

fn cmd_metadata(args: &FetchArgs, out: &GlobalArgs) -> Result<(), AppError> {
    validate_fetch_args(&args.url, args.timeout_secs, args.max_bytes)?;
    let page = fetch_page(&args.url, args.timeout_secs, args.max_bytes, out)?;
    let html = page.html()?;
    let item = build_metadata(&page, &html)?;

    if out.json {
        print_json(&JsonItem { ok: true, item });
    } else if out.quiet {
        println!("{}", non_empty(&item.title, &item.final_url));
    } else {
        println!("{}", non_empty(&item.title, &item.final_url));
        println!("  url={}", item.final_url);
        println!("  status={} bytes={}", item.status, item.bytes);
        if !item.description.is_empty() {
            println!("  {}", item.description);
        }
        println!(
            "  headings={} links={} images={}",
            item.headings_count, item.links_count, item.images_count
        );
    }
    Ok(())
}

fn cmd_text(args: &TextArgs, out: &GlobalArgs) -> Result<(), AppError> {
    validate_fetch_args(&args.url, args.timeout_secs, args.max_bytes)?;
    if args.max_chars == 0 || args.max_chars > 1_000_000 {
        return Err(AppError::InvalidArgument(
            "--max-chars must be between 1 and 1000000".to_string(),
        ));
    }

    let page = fetch_page(&args.url, args.timeout_secs, args.max_bytes, out)?;
    let html = page.html()?;
    let title = first_text(&html, "title")?;
    let (selector, text) = extract_text(&html, args.selector.as_deref())?;
    let (text, truncated) = truncate_chars(&text, args.max_chars);
    let chars = text.chars().count();
    let item = TextItem {
        url: page.requested_url,
        final_url: page.final_url.to_string(),
        title,
        selector,
        text,
        chars,
        truncated,
        content_sha256: page.content_sha256,
    };

    if out.json {
        print_json(&JsonItem { ok: true, item });
    } else if out.quiet {
        println!("{}", item.text);
    } else {
        println!("{}", non_empty(&item.title, &item.final_url));
        println!(
            "  selector={} chars={} truncated={}",
            item.selector, item.chars, item.truncated
        );
        println!();
        println!("{}", item.text);
    }
    Ok(())
}

fn cmd_markdown(args: &TextArgs, out: &GlobalArgs) -> Result<(), AppError> {
    validate_fetch_args(&args.url, args.timeout_secs, args.max_bytes)?;
    if args.max_chars == 0 || args.max_chars > 1_000_000 {
        return Err(AppError::InvalidArgument(
            "--max-chars must be between 1 and 1000000".to_string(),
        ));
    }

    let page = fetch_page(&args.url, args.timeout_secs, args.max_bytes, out)?;
    let html = page.html()?;
    let title = first_text(&html, "title")?;
    let (selector, markdown) = extract_markdown(&html, args.selector.as_deref())?;
    let (markdown, truncated) = truncate_chars(&markdown, args.max_chars);
    let chars = markdown.chars().count();
    let item = MarkdownItem {
        url: page.requested_url,
        final_url: page.final_url.to_string(),
        title,
        selector,
        markdown,
        chars,
        truncated,
        content_sha256: page.content_sha256,
    };

    if out.json {
        print_json(&JsonItem { ok: true, item });
    } else if out.quiet {
        println!("{}", item.markdown);
    } else {
        println!("{}", non_empty(&item.title, &item.final_url));
        println!(
            "  selector={} chars={} truncated={}",
            item.selector, item.chars, item.truncated
        );
        println!();
        println!("{}", item.markdown);
    }
    Ok(())
}

fn cmd_links(args: &LinksArgs, out: &GlobalArgs) -> Result<(), AppError> {
    validate_fetch_args(&args.url, args.timeout_secs, args.max_bytes)?;
    if args.limit == 0 || args.limit > 10_000 {
        return Err(AppError::InvalidArgument(
            "--limit must be between 1 and 10000".to_string(),
        ));
    }
    if args.internal && args.external {
        return Err(AppError::InvalidArgument(
            "--internal and --external cannot be used together".to_string(),
        ));
    }

    let page = fetch_page(&args.url, args.timeout_secs, args.max_bytes, out)?;
    let html = page.html()?;
    let mut items = extract_links(&html, &page.final_url)?;
    if args.internal {
        items.retain(|item| item.internal);
    }
    if args.external {
        items.retain(|item| !item.internal);
    }
    items.truncate(args.limit);

    if out.json {
        print_json(&JsonList {
            ok: true,
            count: items.len(),
            items,
        });
    } else if out.quiet {
        for item in items {
            println!("{}", item.url);
        }
    } else {
        for item in items {
            println!("{}", item.url);
            if !item.text.is_empty() {
                println!("  {}", item.text);
            }
            println!("  internal={}", item.internal);
        }
    }
    Ok(())
}

fn validate_fetch_args(url: &str, timeout_secs: u64, max_bytes: usize) -> Result<(), AppError> {
    let parsed = Url::parse(url)
        .map_err(|_| AppError::InvalidArgument("url must be a valid absolute URL".to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::InvalidArgument(
            "url scheme must be http or https".to_string(),
        ));
    }
    if timeout_secs == 0 || timeout_secs > 300 {
        return Err(AppError::InvalidArgument(
            "--timeout-secs must be between 1 and 300".to_string(),
        ));
    }
    if max_bytes == 0 || max_bytes > 50_000_000 {
        return Err(AppError::InvalidArgument(
            "--max-bytes must be between 1 and 50000000".to_string(),
        ));
    }
    Ok(())
}

fn fetch_page(
    url: &str,
    timeout_secs: u64,
    max_bytes: usize,
    out: &GlobalArgs,
) -> Result<FetchedPage, AppError> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(timeout_secs))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|err| AppError::Internal(err.to_string()))?;

    if out.verbose && !out.quiet {
        eprintln!("debug: GET {url}");
    }

    let response = client
        .get(url)
        .send()
        .map_err(|err| AppError::RequestFailed(err.to_string()))?;
    parse_response(url, response, max_bytes)
}

fn parse_response(
    url: &str,
    response: Response,
    max_bytes: usize,
) -> Result<FetchedPage, AppError> {
    let status = response.status();
    let final_url = response.url().clone();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();

    if let Some(length) = response.content_length() {
        if length > max_bytes as u64 {
            return Err(AppError::ResponseTooLarge(max_bytes));
        }
    }

    if !status.is_success() {
        return Err(AppError::HttpStatus(status.as_u16()));
    }

    let mut bytes = Vec::new();
    response
        .take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|err| AppError::RequestFailed(err.to_string()))?;
    if bytes.len() > max_bytes {
        return Err(AppError::ResponseTooLarge(max_bytes));
    }

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let content_sha256 = format!("{:x}", hasher.finalize());

    Ok(FetchedPage {
        requested_url: url.to_string(),
        final_url,
        status,
        content_type,
        bytes,
        content_sha256,
    })
}

impl FetchedPage {
    fn html(&self) -> Result<Html, AppError> {
        let body = std::str::from_utf8(&self.bytes).map_err(|err| {
            AppError::ParseFailed(format!("response is not valid UTF-8 HTML: {err}"))
        })?;
        Ok(Html::parse_document(body))
    }
}

fn build_metadata(page: &FetchedPage, html: &Html) -> Result<MetadataItem, AppError> {
    let headings = extract_headings(html)?;
    let links = extract_links(html, &page.final_url)?;
    let images_count = count_selector(html, "img")?;
    Ok(MetadataItem {
        url: page.requested_url.clone(),
        final_url: page.final_url.to_string(),
        status: page.status.as_u16(),
        content_type: page.content_type.clone(),
        bytes: page.bytes.len(),
        content_sha256: page.content_sha256.clone(),
        title: first_text(html, "title")?,
        description: meta_content(
            html,
            &["description", "og:description", "twitter:description"],
        ),
        canonical_url: canonical_url(html, &page.final_url),
        lang: html_lang(html),
        headings_count: headings.len(),
        headings,
        links_count: links.len(),
        images_count,
    })
}

fn extract_headings(html: &Html) -> Result<Vec<HeadingItem>, AppError> {
    let selector = parse_selector("h1, h2, h3")?;
    let mut headings = Vec::new();
    for node in html.select(&selector) {
        let name = node.value().name();
        let level = name.trim_start_matches('h').parse::<u8>().unwrap_or(0);
        let text = normalize_ws(&node.text().collect::<Vec<_>>().join(" "));
        if !text.is_empty() {
            headings.push(HeadingItem { level, text });
        }
    }
    Ok(headings)
}

fn extract_text(html: &Html, selector: Option<&str>) -> Result<(String, String), AppError> {
    if let Some(selector) = selector {
        let parsed = parse_selector(selector)?;
        let text = html
            .select(&parsed)
            .map(element_text)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        return Ok((selector.to_string(), text));
    }

    for candidate in ["article", "main", "[role=main]", "body"] {
        let parsed = parse_selector(candidate)?;
        if let Some(node) = html.select(&parsed).next() {
            let text = element_text(node);
            if !text.is_empty() {
                return Ok((candidate.to_string(), text));
            }
        }
    }

    Ok((
        "document".to_string(),
        normalize_ws(&html.root_element().text().collect::<Vec<_>>().join(" ")),
    ))
}

fn extract_markdown(html: &Html, selector: Option<&str>) -> Result<(String, String), AppError> {
    if let Some(selector) = selector {
        let parsed = parse_selector(selector)?;
        let markdown = html
            .select(&parsed)
            .map(markdown_for_element)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        return Ok((selector.to_string(), markdown));
    }

    for candidate in ["article", "main", "[role=main]", "body"] {
        let parsed = parse_selector(candidate)?;
        if let Some(node) = html.select(&parsed).next() {
            let markdown = markdown_for_element(node);
            if !markdown.is_empty() {
                return Ok((candidate.to_string(), markdown));
            }
        }
    }

    Ok((
        "document".to_string(),
        normalize_ws(&html.root_element().text().collect::<Vec<_>>().join(" ")),
    ))
}

fn markdown_for_element(node: ElementRef<'_>) -> String {
    let name = node.value().name();
    match name {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "li" | "pre" | "blockquote" => {
            markdown_block(node)
        }
        _ => {
            let selector = match parse_selector("h1, h2, h3, h4, h5, h6, p, li, pre, blockquote") {
                Ok(selector) => selector,
                Err(_) => return element_text(node),
            };
            let blocks = node
                .select(&selector)
                .map(markdown_block)
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>();
            if blocks.is_empty() {
                element_text(node)
            } else {
                blocks.join("\n\n")
            }
        }
    }
}

fn markdown_block(node: ElementRef<'_>) -> String {
    let text = element_text(node);
    if text.is_empty() {
        return String::new();
    }

    match node.value().name() {
        "h1" => format!("# {text}"),
        "h2" => format!("## {text}"),
        "h3" => format!("### {text}"),
        "h4" => format!("#### {text}"),
        "h5" => format!("##### {text}"),
        "h6" => format!("###### {text}"),
        "li" => format!("- {text}"),
        "pre" => format!("```\n{text}\n```"),
        "blockquote" => text
            .lines()
            .map(|line| format!("> {line}"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => text,
    }
}

fn extract_links(html: &Html, base: &Url) -> Result<Vec<LinkItem>, AppError> {
    let selector = parse_selector("a[href]")?;
    let base_host = base.host_str().unwrap_or("");
    let mut links = Vec::new();

    for node in html.select(&selector) {
        let Some(href) = node.value().attr("href") else {
            continue;
        };
        let Ok(resolved) = base.join(href.trim()) else {
            continue;
        };
        if !matches!(resolved.scheme(), "http" | "https") {
            continue;
        }
        let rel = node.value().attr("rel").unwrap_or("").to_string();
        links.push(LinkItem {
            source_url: base.to_string(),
            url: resolved.to_string(),
            text: element_text(node),
            rel,
            internal: resolved.host_str().unwrap_or("") == base_host,
        });
    }

    Ok(links)
}

fn first_text(html: &Html, selector: &str) -> Result<String, AppError> {
    let selector = parse_selector(selector)?;
    Ok(html
        .select(&selector)
        .next()
        .map(element_text)
        .unwrap_or_default())
}

fn meta_content(html: &Html, keys: &[&str]) -> String {
    let Ok(selector) = Selector::parse("meta") else {
        return String::new();
    };
    for node in html.select(&selector) {
        let name = node.value().attr("name").unwrap_or("").to_ascii_lowercase();
        let property = node
            .value()
            .attr("property")
            .unwrap_or("")
            .to_ascii_lowercase();
        if keys.iter().any(|key| name == *key || property == *key) {
            if let Some(content) = node.value().attr("content") {
                let content = normalize_ws(content);
                if !content.is_empty() {
                    return content;
                }
            }
        }
    }
    String::new()
}

fn canonical_url(html: &Html, base: &Url) -> String {
    let Ok(selector) = Selector::parse("link[href]") else {
        return String::new();
    };
    for node in html.select(&selector) {
        let rel = node.value().attr("rel").unwrap_or("").to_ascii_lowercase();
        if rel.split_whitespace().any(|part| part == "canonical") {
            if let Some(href) = node.value().attr("href") {
                if let Ok(resolved) = base.join(href.trim()) {
                    return resolved.to_string();
                }
            }
        }
    }
    String::new()
}

fn html_lang(html: &Html) -> String {
    let Ok(selector) = Selector::parse("html") else {
        return String::new();
    };
    html.select(&selector)
        .next()
        .and_then(|node| node.value().attr("lang"))
        .unwrap_or("")
        .to_string()
}

fn count_selector(html: &Html, selector: &str) -> Result<usize, AppError> {
    let selector = parse_selector(selector)?;
    Ok(html.select(&selector).count())
}

fn parse_selector(selector: &str) -> Result<Selector, AppError> {
    Selector::parse(selector)
        .map_err(|_| AppError::InvalidArgument(format!("invalid CSS selector: {selector}")))
}

fn element_text(node: ElementRef<'_>) -> String {
    normalize_ws(&node.text().collect::<Vec<_>>().join(" "))
}

fn normalize_ws(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(input: &str, max_chars: usize) -> (String, bool) {
    let mut chars = input.chars();
    let output = chars.by_ref().take(max_chars).collect();
    (output, chars.next().is_some())
}

fn non_empty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(_) => {
            println!("{{\"ok\":false,\"error\":\"serialization failure\",\"code\":\"INTERNAL\"}}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<title>Example Page</title>
<meta name="description" content="An example page for agents.">
<link rel="canonical" href="/canonical">
</head>
<body>
<main><h1>Hello Agents</h1><p>First paragraph.</p><p>Second paragraph.</p></main>
<a href="/about" rel="next">About us</a>
<a href="https://external.example/post">External post</a>
<img src="/image.png" alt="demo">
</body></html>"#;

    #[test]
    fn metadata_extracts_core_fields() {
        let url = Url::parse("https://example.com/page").unwrap();
        let html = Html::parse_document(HTML);
        let page = FetchedPage {
            requested_url: url.to_string(),
            final_url: url,
            status: StatusCode::OK,
            content_type: "text/html".to_string(),
            bytes: HTML.as_bytes().to_vec(),
            content_sha256: "abc".to_string(),
        };
        let item = build_metadata(&page, &html).unwrap();
        assert_eq!(item.title, "Example Page");
        assert_eq!(item.description, "An example page for agents.");
        assert_eq!(item.canonical_url, "https://example.com/canonical");
        assert_eq!(item.headings_count, 1);
        assert_eq!(item.links_count, 2);
        assert_eq!(item.images_count, 1);
    }

    #[test]
    fn text_prefers_main() {
        let html = Html::parse_document(HTML);
        let (selector, text) = extract_text(&html, None).unwrap();
        assert_eq!(selector, "main");
        assert!(text.contains("First paragraph"));
        assert!(!text.contains("About us"));
    }

    #[test]
    fn links_resolve_relative_urls() {
        let html = Html::parse_document(HTML);
        let base = Url::parse("https://example.com/page").unwrap();
        let links = extract_links(&html, &base).unwrap();
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].url, "https://example.com/about");
        assert!(links[0].internal);
        assert!(!links[1].internal);
    }
}
