use clap::{Args, Parser, Subcommand};
use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;

const CRATES_API: &str = "https://crates.io/api/v1/crates";
const USER_AGENT: &str = concat!(
    "dee-package/",
    env!("CARGO_PKG_VERSION"),
    " (https://dee.ink)"
);

#[derive(Debug, Parser)]
#[command(
    name = "dee-package",
    version,
    about = "Package metadata lookup CLI for agents",
    after_help = "EXAMPLES:\n  dee-package search crates serde --limit 5 --json\n  dee-package info crates serde --json\n  dee-package latest crates serde --json\n  dee-package search crates \"http client\" --quiet"
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
    /// Search packages in an ecosystem
    Search(SearchArgs),
    /// Show package metadata
    Info(PackageArgs),
    /// Show latest package version metadata
    Latest(PackageArgs),
}

#[derive(Debug, Args)]
struct SearchArgs {
    /// Ecosystem to search. MVP supports: crates, crates.io, cargo
    ecosystem: String,
    /// Search query
    query: String,
    /// Maximum results to return (1-100)
    #[arg(long, default_value_t = 10)]
    limit: usize,
}

#[derive(Debug, Args)]
struct PackageArgs {
    /// Ecosystem to query. MVP supports: crates, crates.io, cargo
    ecosystem: String,
    /// Package name
    name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ecosystem {
    CratesIo,
}

impl Ecosystem {
    fn parse(input: &str) -> Result<Self, AppError> {
        match input.trim().to_ascii_lowercase().as_str() {
            "crates" | "crates.io" | "crate" | "cargo" | "rust" => Ok(Self::CratesIo),
            other => Err(AppError::UnsupportedEcosystem(other.to_string())),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::CratesIo => "crates.io",
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Unsupported ecosystem '{0}'. Supported ecosystems: crates, crates.io, cargo")]
    UnsupportedEcosystem(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Package '{name}' not found in {ecosystem}")]
    NotFound { ecosystem: String, name: String },
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("HTTP request returned status {0}")]
    HttpStatus(u16),
    #[error("Response parse failed: {0}")]
    ParseFailed(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedEcosystem(_) => "UNSUPPORTED_ECOSYSTEM",
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::RequestFailed(_) => "REQUEST_FAILED",
            Self::HttpStatus(_) => "HTTP_STATUS",
            Self::ParseFailed(_) => "PARSE_FAILED",
            Self::Internal(_) => "INTERNAL",
        }
    }
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

#[derive(Debug, Serialize, Clone)]
struct PackageSummary {
    ecosystem: String,
    name: String,
    version: String,
    description: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    license: String,
    downloads: u64,
    recent_downloads: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    updated_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    repository: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    documentation: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    homepage: String,
    source_url: String,
}

#[derive(Debug, Serialize)]
struct PackageInfo {
    ecosystem: String,
    name: String,
    latest_version: String,
    stable_version: String,
    description: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    license: String,
    downloads: u64,
    recent_downloads: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    created_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    updated_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    repository: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    documentation: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    homepage: String,
    keywords: Vec<String>,
    categories: Vec<String>,
    versions_count: usize,
    yanked_versions: usize,
    source_url: String,
}

#[derive(Debug, Serialize)]
struct VersionInfo {
    ecosystem: String,
    name: String,
    version: String,
    yanked: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    license: String,
    downloads: u64,
    #[serde(skip_serializing_if = "String::is_empty")]
    published_at: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    crate_size: Option<u64>,
    source_url: String,
}

#[derive(Debug, Deserialize)]
struct CratesSearchResponse {
    #[serde(default)]
    crates: Vec<CrateSearchItem>,
}

#[derive(Debug, Deserialize)]
struct CrateSearchItem {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    max_version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    recent_downloads: Option<u64>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    documentation: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    krate: CrateDetails,
    #[serde(default)]
    versions: Vec<CrateVersion>,
}

#[derive(Debug, Deserialize)]
struct CrateDetails {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    max_version: String,
    #[serde(default)]
    newest_version: String,
    #[serde(default)]
    max_stable_version: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    recent_downloads: Option<u64>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    repository: Option<String>,
    #[serde(default)]
    documentation: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    categories: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CrateVersion {
    num: String,
    #[serde(default)]
    yanked: bool,
    #[serde(default)]
    license: Option<String>,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    crate_size: Option<u64>,
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
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|err| AppError::Internal(err.to_string()))?;

    match &cli.command {
        Commands::Search(args) => search_packages(&client, args, &cli.global),
        Commands::Info(args) => show_info(&client, args, &cli.global),
        Commands::Latest(args) => show_latest(&client, args, &cli.global),
    }
}

fn search_packages(client: &Client, args: &SearchArgs, out: &GlobalArgs) -> Result<(), AppError> {
    let ecosystem = Ecosystem::parse(&args.ecosystem)?;
    if args.limit == 0 || args.limit > 100 {
        return Err(AppError::InvalidArgument(
            "--limit must be between 1 and 100".to_string(),
        ));
    }
    if args.query.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "query must not be empty".to_string(),
        ));
    }

    match ecosystem {
        Ecosystem::CratesIo => {
            verbose(
                out,
                &format!("GET {CRATES_API}?q={}&per_page={}", args.query, args.limit),
            );
            let response = client
                .get(CRATES_API)
                .query(&[
                    ("q", args.query.trim().to_string()),
                    ("per_page", args.limit.to_string()),
                    ("page", "1".to_string()),
                ])
                .send()
                .map_err(|err| AppError::RequestFailed(err.to_string()))?;
            let parsed: CratesSearchResponse = parse_response(response, None)?;
            let items: Vec<PackageSummary> = parsed
                .crates
                .into_iter()
                .map(|item| item.into_summary(ecosystem))
                .collect();

            if out.json {
                print_json(&JsonList {
                    ok: true,
                    count: items.len(),
                    items,
                });
            } else if out.quiet {
                for item in items {
                    println!("{} {}", item.name, item.version);
                }
            } else {
                for item in items {
                    println!("{} {}", item.name, item.version);
                    if !item.description.is_empty() {
                        println!("  {}", item.description);
                    }
                    println!(
                        "  downloads={} ecosystem={}",
                        item.downloads, item.ecosystem
                    );
                    if !item.repository.is_empty() {
                        println!("  repo={}", item.repository);
                    }
                }
            }
        }
    }

    Ok(())
}

fn show_info(client: &Client, args: &PackageArgs, out: &GlobalArgs) -> Result<(), AppError> {
    let ecosystem = Ecosystem::parse(&args.ecosystem)?;
    let response = fetch_crate(client, ecosystem, &args.name, out)?;
    let item = response.into_info(ecosystem);

    if out.json {
        print_json(&JsonItem { ok: true, item });
    } else if out.quiet {
        println!("{} {}", item.name, item.latest_version);
    } else {
        println!("{} {}", item.name, item.latest_version);
        println!("  ecosystem={}", item.ecosystem);
        println!(
            "  downloads={} recent={}",
            item.downloads, item.recent_downloads
        );
        if !item.description.is_empty() {
            println!("  {}", item.description);
        }
        if !item.repository.is_empty() {
            println!("  repo={}", item.repository);
        }
    }

    Ok(())
}

fn show_latest(client: &Client, args: &PackageArgs, out: &GlobalArgs) -> Result<(), AppError> {
    let ecosystem = Ecosystem::parse(&args.ecosystem)?;
    let response = fetch_crate(client, ecosystem, &args.name, out)?;
    let item = response.into_latest(ecosystem);

    if out.json {
        print_json(&JsonItem { ok: true, item });
    } else if out.quiet {
        println!("{} {}", item.name, item.version);
    } else {
        println!("{} {}", item.name, item.version);
        println!("  ecosystem={}", item.ecosystem);
        println!("  yanked={} downloads={}", item.yanked, item.downloads);
        if !item.license.is_empty() {
            println!("  license={}", item.license);
        }
    }

    Ok(())
}

fn fetch_crate(
    client: &Client,
    ecosystem: Ecosystem,
    name: &str,
    out: &GlobalArgs,
) -> Result<CrateResponse, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "name must not be empty".to_string(),
        ));
    }

    match ecosystem {
        Ecosystem::CratesIo => {
            let encoded = urlencoding::encode(name.trim());
            let url = format!("{CRATES_API}/{encoded}");
            verbose(out, &format!("GET {url}"));
            let response = client
                .get(&url)
                .send()
                .map_err(|err| AppError::RequestFailed(err.to_string()))?;
            parse_response(
                response,
                Some((ecosystem.as_str().to_string(), name.trim().to_string())),
            )
        }
    }
}

fn parse_response<T: DeserializeOwned>(
    response: Response,
    not_found_context: Option<(String, String)>,
) -> Result<T, AppError> {
    let status = response.status();
    if status == StatusCode::NOT_FOUND {
        if let Some((ecosystem, name)) = not_found_context {
            return Err(AppError::NotFound { ecosystem, name });
        }
        return Err(AppError::HttpStatus(status.as_u16()));
    }
    if !status.is_success() {
        return Err(AppError::HttpStatus(status.as_u16()));
    }
    response
        .json::<T>()
        .map_err(|err| AppError::ParseFailed(err.to_string()))
}

fn verbose(out: &GlobalArgs, message: &str) {
    if out.verbose && !out.quiet {
        eprintln!("debug: {message}");
    }
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(_) => println!(
            "{}",
            r#"{"ok":false,"error":"serialization failure","code":"INTERNAL"}"#
        ),
    }
}

impl CrateSearchItem {
    fn into_summary(self, ecosystem: Ecosystem) -> PackageSummary {
        let name = if self.name.is_empty() {
            self.id
        } else {
            self.name
        };
        PackageSummary {
            ecosystem: ecosystem.as_str().to_string(),
            source_url: crate_url(&name),
            name,
            version: self.max_version,
            description: self.description.unwrap_or_default(),
            license: self.license.unwrap_or_default(),
            downloads: self.downloads,
            recent_downloads: self.recent_downloads.unwrap_or_default(),
            updated_at: self.updated_at.unwrap_or_default(),
            repository: self.repository.unwrap_or_default(),
            documentation: self.documentation.unwrap_or_default(),
            homepage: self.homepage.unwrap_or_default(),
        }
    }
}

impl CrateResponse {
    fn into_info(self, ecosystem: Ecosystem) -> PackageInfo {
        let latest = choose_latest_version(&self.krate);
        let latest_version = find_version(&self.versions, &latest);
        let license = self
            .krate
            .license
            .clone()
            .or_else(|| latest_version.and_then(|version| version.license.clone()))
            .unwrap_or_default();
        let name = if self.krate.name.is_empty() {
            self.krate.id.clone()
        } else {
            self.krate.name.clone()
        };

        PackageInfo {
            ecosystem: ecosystem.as_str().to_string(),
            name: name.clone(),
            latest_version: latest,
            stable_version: self.krate.max_stable_version.unwrap_or_default(),
            description: self.krate.description.unwrap_or_default(),
            license,
            downloads: self.krate.downloads,
            recent_downloads: self.krate.recent_downloads.unwrap_or_default(),
            created_at: self.krate.created_at.unwrap_or_default(),
            updated_at: self.krate.updated_at.unwrap_or_default(),
            repository: self.krate.repository.unwrap_or_default(),
            documentation: self.krate.documentation.unwrap_or_default(),
            homepage: self.krate.homepage.unwrap_or_default(),
            keywords: self.krate.keywords,
            categories: self.krate.categories,
            versions_count: self.versions.len(),
            yanked_versions: self
                .versions
                .iter()
                .filter(|version| version.yanked)
                .count(),
            source_url: crate_url(&name),
        }
    }

    fn into_latest(self, ecosystem: Ecosystem) -> VersionInfo {
        let name = if self.krate.name.is_empty() {
            self.krate.id.clone()
        } else {
            self.krate.name.clone()
        };
        let latest = choose_latest_version(&self.krate);
        let version = find_version(&self.versions, &latest);

        VersionInfo {
            ecosystem: ecosystem.as_str().to_string(),
            name: name.clone(),
            version: latest,
            yanked: version.map(|version| version.yanked).unwrap_or(false),
            license: self
                .krate
                .license
                .or_else(|| version.and_then(|version| version.license.clone()))
                .unwrap_or_default(),
            downloads: version.map(|version| version.downloads).unwrap_or_default(),
            published_at: version
                .and_then(|version| version.created_at.clone())
                .unwrap_or_default(),
            updated_at: version
                .and_then(|version| version.updated_at.clone())
                .unwrap_or_default(),
            crate_size: version.and_then(|version| version.crate_size),
            source_url: crate_url(&name),
        }
    }
}

fn choose_latest_version(krate: &CrateDetails) -> String {
    krate
        .max_stable_version
        .clone()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            if !krate.max_version.is_empty() {
                krate.max_version.clone()
            } else {
                krate.newest_version.clone()
            }
        })
}

fn find_version<'a>(versions: &'a [CrateVersion], version: &str) -> Option<&'a CrateVersion> {
    versions.iter().find(|candidate| candidate.num == version)
}

fn crate_url(name: &str) -> String {
    format!("https://crates.io/crates/{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_crates_aliases() {
        assert_eq!(Ecosystem::parse("crates").unwrap(), Ecosystem::CratesIo);
        assert_eq!(Ecosystem::parse("crates.io").unwrap(), Ecosystem::CratesIo);
        assert_eq!(Ecosystem::parse("cargo").unwrap(), Ecosystem::CratesIo);
    }

    #[test]
    fn rejects_other_ecosystems_for_mvp() {
        let err = Ecosystem::parse("npm").unwrap_err();
        assert_eq!(err.code(), "UNSUPPORTED_ECOSYSTEM");
    }

    #[test]
    fn chooses_stable_version_first() {
        let krate = CrateDetails {
            id: "serde".to_string(),
            name: "serde".to_string(),
            max_version: "2.0.0-alpha".to_string(),
            newest_version: "2.0.0-alpha".to_string(),
            max_stable_version: Some("1.0.0".to_string()),
            description: None,
            license: None,
            downloads: 0,
            recent_downloads: None,
            created_at: None,
            updated_at: None,
            repository: None,
            documentation: None,
            homepage: None,
            keywords: Vec::new(),
            categories: Vec::new(),
        };
        assert_eq!(choose_latest_version(&krate), "1.0.0");
    }
}
