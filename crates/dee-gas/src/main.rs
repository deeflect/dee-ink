use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

const EIA_BASE: &str = "https://api.eia.gov/v2/petroleum/pri/gnd/data/";

#[derive(Debug, Parser)]
#[command(
    name = "dee-gas",
    version,
    about = "Gas prices by US region/state",
    after_help = "EXAMPLES:\n  dee-gas national --json\n  dee-gas prices --state CA --grade regular --json\n  dee-gas history --state TX --weeks 6 --json\n  dee-gas config set eia.api-key <KEY>"
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
    Prices(PricesArgs),
    National(OutOnlyArgs),
    History(HistoryArgs),
    Config(ConfigArgs),
}

#[derive(Debug, Clone, ValueEnum)]
enum Grade {
    Regular,
    Midgrade,
    Premium,
    Diesel,
}

#[derive(Debug, Args)]
struct PricesArgs {
    #[arg(long)]
    state: Option<String>,
    #[arg(long)]
    region: bool,
    #[arg(long, value_enum, default_value_t = Grade::Regular)]
    grade: Grade,
}

#[derive(Debug, Args)]
struct HistoryArgs {
    #[arg(long)]
    state: Option<String>,
    #[arg(long, default_value_t = 4)]
    weeks: usize,
    #[arg(long, value_enum, default_value_t = Grade::Regular)]
    grade: Grade,
}

#[derive(Debug, Args)]
struct OutOnlyArgs {}

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
struct GasPoint {
    period: String,
    area: String,
    series: String,
    grade: String,
    price: f64,
    units: String,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Configuration directory not found")]
    ConfigMissing,
    #[error("Missing EIA API key. Set eia.api-key via config set")]
    AuthMissing,
    #[error("Unknown config key: {0}")]
    InvalidConfigKey(String),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("HTTP request failed")]
    RequestFailed,
    #[error("EIA API returned an error")]
    ApiError,
    #[error("No data found")]
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
struct EiaRoot {
    response: Option<EiaResponse>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct EiaResponse {
    data: Vec<EiaRow>,
}

#[derive(Debug, Deserialize)]
struct EiaRow {
    period: String,
    series: String,
    #[serde(default)]
    area_name: Option<String>,
    #[serde(default)]
    units: Option<String>,
    #[serde(default)]
    value: Option<f64>,
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
        Commands::Prices(args) => cmd_prices(args, &cli.global),
        Commands::National(_) => cmd_national(&cli.global),
        Commands::History(args) => cmd_history(args, &cli.global),
        Commands::Config(args) => cmd_config(args),
    }
}

fn cmd_prices(args: &PricesArgs, out: &GlobalArgs) -> Result<(), AppError> {
    if args.region && args.state.is_some() {
        return Err(AppError::InvalidArgument(
            "use either --region or --state".to_string(),
        ));
    }

    let mut series_codes = Vec::new();
    if args.region {
        series_codes.extend(["R1X", "R2X", "R3X", "R4X"].map(|x| x.to_string()));
    } else if let Some(state) = &args.state {
        let code = state.trim().to_uppercase();
        if code.len() != 2 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(AppError::InvalidArgument(
                "--state must be 2 letters".to_string(),
            ));
        }
        series_codes.push(code);
    } else {
        series_codes.push("NUS".to_string());
    }

    let mut items = Vec::new();
    for area in series_codes {
        let series = series_code(&area, &args.grade);
        let mut rows = fetch_series(&series, 1, out.verbose)?;
        if let Some(item) = rows.pop() {
            items.push(item);
        }
    }

    if items.is_empty() {
        return Err(AppError::NotFound);
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
            println!(
                "{} {}: ${:.3}/gal ({})",
                item.area, item.grade, item.price, item.period
            );
        }
    }

    Ok(())
}

fn cmd_national(out: &GlobalArgs) -> Result<(), AppError> {
    let series = series_code("NUS", &Grade::Regular);
    let mut rows = fetch_series(&series, 1, out.verbose)?;
    let item = rows.pop().ok_or(AppError::NotFound)?;

    if out.json {
        print_json(&OkItem { ok: true, item });
    } else if out.quiet {
        println!("{:.3}", item.price);
    } else {
        println!(
            "US national regular: ${:.3}/gal ({})",
            item.price, item.period
        );
    }

    Ok(())
}

fn cmd_history(args: &HistoryArgs, out: &GlobalArgs) -> Result<(), AppError> {
    if args.weeks == 0 {
        return Err(AppError::InvalidArgument("--weeks must be > 0".to_string()));
    }

    let area = args
        .state
        .as_ref()
        .map(|x| x.trim().to_uppercase())
        .unwrap_or_else(|| "NUS".to_string());
    if area.len() != 3 && area.len() != 2 {
        return Err(AppError::InvalidArgument(
            "--state must be 2-letter code".to_string(),
        ));
    }

    let series = series_code(&area, &args.grade);
    let items = fetch_series(&series, args.weeks, out.verbose)?;
    if items.is_empty() {
        return Err(AppError::NotFound);
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
            println!("{}: ${:.3}/gal", item.period, item.price);
        }
    }

    Ok(())
}

fn fetch_series(series: &str, length: usize, verbose: bool) -> Result<Vec<GasPoint>, AppError> {
    let cfg = load_config().map_err(|_| AppError::ConfigMissing)?;
    let api_key = cfg
        .api_key
        .filter(|x| !x.trim().is_empty())
        .ok_or(AppError::AuthMissing)?;

    let url = format!(
        "{base}?api_key={api}&frequency=weekly&data[0]=value&facets[series][]={series}&sort[0][column]=period&sort[0][direction]=desc&length={length}",
        base = EIA_BASE,
        api = urlencoding::encode(&api_key),
        series = urlencoding::encode(series),
        length = length
    );

    if verbose {
        eprintln!("debug: GET {url}");
    }

    let client = Client::builder()
        .user_agent("dee-gas/0.1.0 (https://dee.ink)")
        .build()
        .map_err(|_| AppError::RequestFailed)?;

    let body: EiaRoot = client
        .get(&url)
        .send()
        .map_err(|_| AppError::RequestFailed)?
        .error_for_status()
        .map_err(|_| AppError::RequestFailed)?
        .json()
        .map_err(|_| AppError::ParseFailed)?;

    if body.error.is_some() {
        return Err(AppError::ApiError);
    }

    let response = body.response.ok_or(AppError::ParseFailed)?;
    let mut out = Vec::new();
    for row in response.data {
        let Some(value) = row.value else {
            continue;
        };
        let area = row
            .area_name
            .clone()
            .unwrap_or_else(|| extract_area_from_series(&row.series));
        out.push(GasPoint {
            period: row.period,
            area,
            series: row.series,
            grade: "regular".to_string(),
            price: value,
            units: row.units.unwrap_or_else(|| "USD/gal".to_string()),
        });
    }

    Ok(out)
}

fn series_code(area_code: &str, grade: &Grade) -> String {
    let grade_code = match grade {
        Grade::Regular => "PTE",
        Grade::Midgrade => "PTM",
        Grade::Premium => "PTP",
        Grade::Diesel => "EPD2D",
    };

    if matches!(grade, Grade::Diesel) {
        format!("EMM_{grade_code}_{area}_DPG", area = area_code)
    } else {
        format!(
            "EMM_EPMRR_{grade}_{area}_DPG",
            grade = grade_code,
            area = area_code
        )
    }
}

fn extract_area_from_series(series: &str) -> String {
    let parts: Vec<&str> = series.split('_').collect();
    parts
        .iter()
        .find(|part| part.len() == 2 || part.len() == 3)
        .copied()
        .unwrap_or("NUS")
        .to_string()
}

fn cmd_config(args: &ConfigArgs) -> Result<(), AppError> {
    match &args.command {
        ConfigCommand::Set(input) => {
            let mut cfg = load_config().unwrap_or_default();
            match input.key.as_str() {
                "eia.api-key" | "api_key" => cfg.api_key = Some(input.value.clone()),
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
    path.push("dee-gas");
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
