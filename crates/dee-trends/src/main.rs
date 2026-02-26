use clap::{Args, Parser, Subcommand};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::{json, Value};

const DEFAULT_BASE_URL: &str = "https://trends.google.com/trends/api";

#[derive(Debug, Parser)]
#[command(
    name = "dee-trends",
    version,
    about = "Google Trends CLI",
    long_about = "dee-trends - Fetch Google Trends interest and related queries as structured JSON.",
    after_help = "EXAMPLES:\n  dee-trends interest \"rust\" --geo US --time \"today 12-m\"\n  dee-trends interest \"openrouter\" --json\n  dee-trends related \"claude\" --geo US --json\n  dee-trends explore \"llm agents\" --json"
)]
struct Cli {
    #[command(flatten)]
    global: GlobalFlags,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Args)]
struct GlobalFlags {
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
    /// Fetch interest over time points
    Interest(QueryArgs),
    /// Fetch related queries (top/rising)
    Related(QueryArgs),
    /// Show raw explore widgets summary
    Explore(QueryArgs),
}

#[derive(Debug, Args)]
struct QueryArgs {
    /// Search keyword
    keyword: String,

    /// Geo code, e.g. US, GB, or empty for global
    #[arg(long, default_value = "")]
    geo: String,

    /// Time range (Google Trends format), e.g. "today 12-m"
    #[arg(long, default_value = "today 12-m")]
    time: String,

    /// Language (hl), e.g. en-US
    #[arg(long, default_value = "en-US")]
    hl: String,

    /// Timezone offset minutes from UTC
    #[arg(long, default_value_t = 0)]
    tz: i32,
}

#[derive(Debug, Serialize)]
struct ListResponse<T> {
    ok: bool,
    count: usize,
    items: Vec<T>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, Serialize)]
struct InterestPoint {
    timestamp: String,
    formatted_time: String,
    value: i64,
}

#[derive(Debug, Serialize)]
struct RelatedQuery {
    query: String,
    query_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formatted_value: Option<String>,
}

#[derive(Debug, Serialize)]
struct WidgetSummary {
    id: String,
    title: String,
    token: String,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Request failed")]
    RequestFailed,
    #[error("Upstream API error")]
    ApiError,
    #[error("Response parse failed")]
    ParseFailed,
    #[error("Expected data not found")]
    NotFound,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::RequestFailed => "REQUEST_FAILED",
            Self::ApiError => "API_ERROR",
            Self::ParseFailed => "PARSE_FAILED",
            Self::NotFound => "NOT_FOUND",
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let result = run(&cli);

    if let Err(err) = result {
        if cli.global.json {
            print_json(&ErrorResponse {
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

fn run(cli: &Cli) -> Result<(), AppError> {
    let args = match &cli.command {
        Commands::Interest(a) | Commands::Related(a) | Commands::Explore(a) => a,
    };

    validate_args(args)?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|_| AppError::RequestFailed)?;

    let api = TrendsApi::new(client, base_url(), cli.global.verbose);

    match &cli.command {
        Commands::Explore(a) => {
            let widgets = api.explore(a)?;
            let items = widgets
                .iter()
                .map(|w| WidgetSummary {
                    id: w.id.clone(),
                    title: w.title.clone(),
                    token: w.token.clone(),
                })
                .collect::<Vec<_>>();

            if cli.global.json {
                print_json(&ListResponse {
                    ok: true,
                    count: items.len(),
                    items,
                });
            } else if cli.global.quiet {
                println!("{}", items.len());
            } else {
                for item in items {
                    println!("{} - {}", item.id, item.title);
                }
            }
        }
        Commands::Interest(a) => {
            let points = api.interest_points(a)?;
            if cli.global.json {
                print_json(&ListResponse {
                    ok: true,
                    count: points.len(),
                    items: points,
                });
            } else if cli.global.quiet {
                println!("{}", points.len());
            } else if points.is_empty() {
                println!("no interest points found");
            } else {
                for p in points {
                    println!("{} {} {}", p.timestamp, p.formatted_time, p.value);
                }
            }
        }
        Commands::Related(a) => {
            let items = api.related_queries(a)?;
            if cli.global.json {
                print_json(&ListResponse {
                    ok: true,
                    count: items.len(),
                    items,
                });
            } else if cli.global.quiet {
                println!("{}", items.len());
            } else if items.is_empty() {
                println!("no related queries found");
            } else {
                for item in items {
                    let value = item.value.unwrap_or(0);
                    println!("{} [{}] {}", item.query, item.query_type, value);
                }
            }
        }
    }

    Ok(())
}

fn validate_args(args: &QueryArgs) -> Result<(), AppError> {
    if args.keyword.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "keyword must not be empty".to_string(),
        ));
    }

    if args.hl.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "hl must not be empty".to_string(),
        ));
    }

    Ok(())
}

fn base_url() -> String {
    std::env::var("DEE_TRENDS_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

#[derive(Debug, Clone)]
struct Widget {
    id: String,
    title: String,
    token: String,
    request: Value,
}

struct TrendsApi {
    client: Client,
    base_url: String,
    verbose: bool,
}

impl TrendsApi {
    fn new(client: Client, base_url: String, verbose: bool) -> Self {
        Self {
            client,
            base_url,
            verbose,
        }
    }

    fn explore(&self, args: &QueryArgs) -> Result<Vec<Widget>, AppError> {
        let req_json = json!({
            "comparisonItem": [{
                "keyword": args.keyword,
                "geo": args.geo,
                "time": args.time,
            }],
            "category": 0,
            "property": "",
        });

        let req_body = req_json.to_string();
        let req_encoded = urlencoding::encode(&req_body);
        let url = format!(
            "{}/explore?hl={}&tz={}&req={}",
            self.base_url,
            urlencoding::encode(&args.hl),
            args.tz,
            req_encoded
        );

        let raw = self.get_text(&url)?;
        let body = strip_xssi_prefix(&raw);
        let value: Value = serde_json::from_str(body).map_err(|_| AppError::ParseFailed)?;

        let widgets = value
            .get("widgets")
            .and_then(Value::as_array)
            .ok_or(AppError::ParseFailed)?;

        let mut out = Vec::new();
        for widget in widgets {
            let id = widget
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let title = widget
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let token = widget
                .get("token")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let request = widget.get("request").cloned().unwrap_or_else(|| json!({}));

            if !id.is_empty() && !token.is_empty() {
                out.push(Widget {
                    id,
                    title,
                    token,
                    request,
                });
            }
        }

        if out.is_empty() {
            return Err(AppError::NotFound);
        }

        Ok(out)
    }

    fn interest_points(&self, args: &QueryArgs) -> Result<Vec<InterestPoint>, AppError> {
        let widgets = self.explore(args)?;
        let widget = widgets
            .iter()
            .find(|w| w.id == "TIMESERIES")
            .ok_or(AppError::NotFound)?;

        let req_body = widget.request.to_string();
        let req_encoded = urlencoding::encode(&req_body);
        let url = format!(
            "{}/widgetdata/multiline?hl={}&tz={}&req={}&token={}",
            self.base_url,
            urlencoding::encode(&args.hl),
            args.tz,
            req_encoded,
            urlencoding::encode(&widget.token)
        );

        let raw = self.get_text(&url)?;
        let body = strip_xssi_prefix(&raw);
        let value: Value = serde_json::from_str(body).map_err(|_| AppError::ParseFailed)?;
        let timeline = value
            .get("default")
            .and_then(|v| v.get("timelineData"))
            .and_then(Value::as_array)
            .ok_or(AppError::ParseFailed)?;

        let mut points = Vec::new();
        for point in timeline {
            let timestamp = point
                .get("time")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let formatted = point
                .get("formattedTime")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let value = point
                .get("value")
                .and_then(Value::as_array)
                .and_then(|arr| arr.first())
                .and_then(Value::as_i64)
                .unwrap_or(0);

            points.push(InterestPoint {
                timestamp,
                formatted_time: formatted,
                value,
            });
        }

        Ok(points)
    }

    fn related_queries(&self, args: &QueryArgs) -> Result<Vec<RelatedQuery>, AppError> {
        let widgets = self.explore(args)?;
        let related_widgets: Vec<&Widget> = widgets
            .iter()
            .filter(|w| w.id == "RELATED_QUERIES")
            .collect();

        if related_widgets.is_empty() {
            return Err(AppError::NotFound);
        }

        let mut items = Vec::new();

        for widget in related_widgets {
            let req_body = widget.request.to_string();
            let req_encoded = urlencoding::encode(&req_body);
            let url = format!(
                "{}/widgetdata/relatedsearches?hl={}&tz={}&req={}&token={}",
                self.base_url,
                urlencoding::encode(&args.hl),
                args.tz,
                req_encoded,
                urlencoding::encode(&widget.token)
            );

            let raw = self.get_text(&url)?;
            let body = strip_xssi_prefix(&raw);
            let value: Value = serde_json::from_str(body).map_err(|_| AppError::ParseFailed)?;
            let ranked_list = value
                .get("default")
                .and_then(|v| v.get("rankedList"))
                .and_then(Value::as_array)
                .ok_or(AppError::ParseFailed)?;

            for bucket in ranked_list {
                let query_type = bucket
                    .get("rankedKeyword")
                    .and_then(Value::as_array)
                    .and_then(|kw| kw.first())
                    .and_then(|first| first.get("queryType"))
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();

                if let Some(entries) = bucket.get("rankedKeyword").and_then(Value::as_array) {
                    for entry in entries {
                        let query = entry
                            .get("query")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                        if query.is_empty() {
                            continue;
                        }
                        let value = entry.get("value").and_then(Value::as_i64);
                        let formatted_value = entry
                            .get("formattedValue")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned);

                        items.push(RelatedQuery {
                            query,
                            query_type: query_type.clone(),
                            value,
                            formatted_value,
                        });
                    }
                }
            }
        }

        Ok(items)
    }

    fn get_text(&self, url: &str) -> Result<String, AppError> {
        if self.verbose {
            eprintln!("[dee-trends] GET {url}");
        }

        let response = self
            .client
            .get(url)
            .header("User-Agent", "dee-trends/0.1")
            .send()
            .map_err(|_| AppError::RequestFailed)?;

        if response.status() != StatusCode::OK {
            return Err(AppError::ApiError);
        }

        response.text().map_err(|_| AppError::RequestFailed)
    }
}

fn strip_xssi_prefix(input: &str) -> &str {
    input
        .trim_start_matches(")]}'")
        .trim_start_matches('\n')
        .trim_start()
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(text) => println!("{text}"),
        Err(_) => {
            println!(
                "{{\"ok\":false,\"error\":\"JSON serialization failed\",\"code\":\"SERIALIZE\"}}"
            );
            std::process::exit(1);
        }
    }
}
