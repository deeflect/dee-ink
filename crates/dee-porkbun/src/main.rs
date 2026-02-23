use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const API_BASE: &str = "https://api.porkbun.com/api/json/v3";

#[derive(Debug, Parser)]
#[command(
    name = "dee-porkbun",
    version,
    about = "Porkbun API CLI",
    long_about = "dee-porkbun - Full Porkbun API wrapper with agent-friendly JSON output.",
    after_help = "EXAMPLES:\n  dee-porkbun config set api_key pk1_xxx\n  dee-porkbun config set secret_key sk1_xxx\n  dee-porkbun domains pricing --tld com --json\n  dee-porkbun domains list-all --json\n  dee-porkbun dns retrieve dee.ink --json\n  dee-porkbun dns create dee.ink --type A --name www --content 1.1.1.1 --confirm --json\n  dee-porkbun dnssec get dee.ink --json\n  dee-porkbun ssl retrieve dee.ink --json"
)]
struct Cli {
    #[command(flatten)]
    global: OutputFlags,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Args)]
struct OutputFlags {
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
    /// Manage configuration
    Config(ConfigArgs),
    /// Domain endpoints
    Domains(DomainsArgs),
    /// DNS endpoints
    Dns(DnsArgs),
    /// DNSSEC endpoints
    Dnssec(DnssecArgs),
    /// SSL endpoints
    Ssl(SslArgs),
}

#[derive(Debug, Args)]
struct ConfigArgs {
    #[command(subcommand)]
    command: ConfigCommand,
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    /// Set a config value (api_key|secret_key)
    Set(ConfigSetArgs),
    /// Show current config
    Show,
    /// Print config path
    Path,
}

#[derive(Debug, Args)]
struct ConfigSetArgs {
    /// Config key: api_key or secret_key
    key: String,
    /// Config value
    value: String,
}

#[derive(Debug, Args)]
struct DomainsArgs {
    #[command(subcommand)]
    command: DomainsCommand,
}

#[derive(Debug, Subcommand)]
enum DomainsCommand {
    /// API connectivity check
    Ping,
    /// Domain pricing
    Pricing(PricingArgs),
    /// List all domains
    ListAll(ListAllArgs),
    /// Check domain availability
    Check(CheckArgs),
    /// Register a domain
    Create(CreateDomainArgs),
    /// Update nameservers
    UpdateNs(UpdateNsArgs),
    /// Get nameservers
    GetNs(GetDomainArgs),
    /// Update auto-renew for one or more domains
    UpdateAutoRenew(UpdateAutoRenewArgs),
    /// Add URL forward
    AddUrlForward(AddUrlForwardArgs),
    /// Get URL forwarding
    GetUrlForwarding(GetDomainArgs),
    /// Delete URL forward by record id
    DeleteUrlForward(DeleteUrlForwardArgs),
    /// Create glue host
    CreateGlue(GlueUpsertArgs),
    /// Update glue host
    UpdateGlue(GlueUpsertArgs),
    /// Delete glue host
    DeleteGlue(GlueDeleteArgs),
    /// Get glue hosts
    GetGlue(GetDomainArgs),
}

#[derive(Debug, Args)]
struct DnsArgs {
    #[command(subcommand)]
    command: DnsCommand,
}

#[derive(Debug, Subcommand)]
enum DnsCommand {
    /// Create DNS record
    Create(DnsCreateArgs),
    /// Edit DNS record by domain/id
    Edit(DnsEditArgs),
    /// Edit DNS records by name/type
    EditByNameType(DnsEditByNameTypeArgs),
    /// Delete DNS record by domain/id
    Delete(DnsDeleteArgs),
    /// Delete DNS records by name/type
    DeleteByNameType(DnsDeleteByNameTypeArgs),
    /// Retrieve DNS records by domain or id
    Retrieve(DnsRetrieveArgs),
    /// Retrieve DNS records by name/type
    RetrieveByNameType(DnsRetrieveByNameTypeArgs),
}

#[derive(Debug, Args)]
struct DnssecArgs {
    #[command(subcommand)]
    command: DnssecCommand,
}

#[derive(Debug, Subcommand)]
enum DnssecCommand {
    /// Create DNSSEC record
    Create(DnssecCreateArgs),
    /// Get DNSSEC records
    Get(GetDomainArgs),
    /// Delete DNSSEC record by key tag
    Delete(DnssecDeleteArgs),
}

#[derive(Debug, Args)]
struct SslArgs {
    #[command(subcommand)]
    command: SslCommand,
}

#[derive(Debug, Subcommand)]
enum SslCommand {
    /// Retrieve SSL bundle for a domain
    Retrieve(GetDomainArgs),
}

#[derive(Debug, Args)]
struct PricingArgs {
    /// Optional TLD filter, e.g. com
    #[arg(long)]
    tld: Option<String>,
}

#[derive(Debug, Args)]
struct ListAllArgs {
    /// Optional start index (chunked by 1000)
    #[arg(long)]
    start: Option<u64>,

    /// Include domain labels
    #[arg(long)]
    include_labels: bool,
}

#[derive(Debug, Args)]
struct CheckArgs {
    /// Domain name
    domain: String,
}

#[derive(Debug, Args)]
struct GetDomainArgs {
    /// Domain name
    domain: String,
}

#[derive(Debug, Args)]
struct CreateDomainArgs {
    /// Domain name
    domain: String,

    /// Cost in pennies from `domains check`
    #[arg(long)]
    cost: Option<u64>,

    /// Acknowledge registration terms
    #[arg(long)]
    agree_to_terms: bool,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct UpdateNsArgs {
    /// Domain name
    domain: String,

    /// Nameserver (repeatable)
    #[arg(long = "ns", required = true)]
    nameservers: Vec<String>,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct UpdateAutoRenewArgs {
    /// on|off
    status: String,

    /// Optional domain in URL path
    domain: Option<String>,

    /// Domains to update (repeatable)
    #[arg(long = "domain")]
    domains: Vec<String>,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct AddUrlForwardArgs {
    /// Domain name
    domain: String,

    /// Subdomain for forward, empty for root
    #[arg(long, default_value = "")]
    subdomain: String,

    /// Forward destination URL
    #[arg(long)]
    location: String,

    /// temporary|permanent
    #[arg(long)]
    r#type: String,

    /// yes|no
    #[arg(long, default_value = "no")]
    include_path: String,

    /// yes|no
    #[arg(long, default_value = "no")]
    wildcard: String,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct DeleteUrlForwardArgs {
    /// Domain name
    domain: String,

    /// URL forward record id
    record_id: String,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct GlueUpsertArgs {
    /// Domain name
    domain: String,

    /// Glue host subdomain, e.g. ns1
    host: String,

    /// IP address for glue host (repeatable)
    #[arg(long = "ip", required = true)]
    ips: Vec<String>,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct GlueDeleteArgs {
    /// Domain name
    domain: String,

    /// Glue host subdomain, e.g. ns1
    host: String,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct DnsCreateArgs {
    /// Domain name
    domain: String,

    /// Record type (A, MX, TXT, ...)
    #[arg(long)]
    r#type: String,

    /// Subdomain, empty for apex
    #[arg(long, default_value = "")]
    name: String,

    /// Record content
    #[arg(long)]
    content: String,

    /// TTL seconds
    #[arg(long)]
    ttl: Option<u32>,

    /// Priority
    #[arg(long)]
    prio: Option<u32>,

    /// Notes
    #[arg(long)]
    notes: Option<String>,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct DnsEditArgs {
    /// Domain name
    domain: String,

    /// DNS record id
    record_id: String,

    /// Record type
    #[arg(long)]
    r#type: String,

    /// Subdomain, empty for apex
    #[arg(long, default_value = "")]
    name: String,

    /// Record content
    #[arg(long)]
    content: String,

    /// TTL seconds
    #[arg(long)]
    ttl: Option<u32>,

    /// Priority
    #[arg(long)]
    prio: Option<u32>,

    /// Notes
    #[arg(long)]
    notes: Option<String>,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct DnsEditByNameTypeArgs {
    /// Domain name
    domain: String,

    /// Record type
    record_type: String,

    /// Optional subdomain
    subdomain: Option<String>,

    /// Record content
    #[arg(long)]
    content: String,

    /// TTL seconds
    #[arg(long)]
    ttl: Option<u32>,

    /// Priority
    #[arg(long)]
    prio: Option<u32>,

    /// Notes
    #[arg(long)]
    notes: Option<String>,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct DnsDeleteArgs {
    /// Domain name
    domain: String,

    /// DNS record id
    record_id: String,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct DnsDeleteByNameTypeArgs {
    /// Domain name
    domain: String,

    /// Record type
    record_type: String,

    /// Optional subdomain
    subdomain: Option<String>,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct DnsRetrieveArgs {
    /// Domain name
    domain: String,

    /// Optional DNS record id
    record_id: Option<String>,
}

#[derive(Debug, Args)]
struct DnsRetrieveByNameTypeArgs {
    /// Domain name
    domain: String,

    /// Record type
    record_type: String,

    /// Optional subdomain
    subdomain: Option<String>,
}

#[derive(Debug, Args)]
struct DnssecCreateArgs {
    /// Domain name
    domain: String,

    #[arg(long)]
    key_tag: String,
    #[arg(long)]
    alg: String,
    #[arg(long)]
    digest_type: String,
    #[arg(long)]
    digest: String,
    #[arg(long)]
    max_sig_life: Option<String>,
    #[arg(long)]
    key_data_flags: Option<String>,
    #[arg(long)]
    key_data_protocol: Option<String>,
    #[arg(long)]
    key_data_algo: Option<String>,
    #[arg(long)]
    key_data_pub_key: Option<String>,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
}

#[derive(Debug, Args)]
struct DnssecDeleteArgs {
    /// Domain name
    domain: String,

    /// DNSSEC key tag
    key_tag: String,

    /// Required for mutating commands
    #[arg(long)]
    confirm: bool,
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
struct SuccessMessage {
    ok: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorJson {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Config directory is unavailable")]
    ConfigDirUnavailable,
    #[error("Config file not found. Run `dee-porkbun config set api_key <value>` and `dee-porkbun config set secret_key <value>`")]
    ConfigMissing,
    #[error(
        "Authentication keys are missing. Set api_key and secret_key via `dee-porkbun config set`"
    )]
    AuthMissing,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Confirmation required: rerun with --confirm")]
    ConfirmRequired,
    #[error("Network request failed: {0}")]
    RequestFailed(String),
    #[error("Porkbun API error: {0}")]
    ApiError(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Failed to parse API response")]
    ParseFailed,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::ConfigDirUnavailable | Self::ConfigMissing => "CONFIG_MISSING",
            Self::AuthMissing => "AUTH_MISSING",
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::ConfirmRequired => "CONFIRM_REQUIRED",
            Self::RequestFailed(_) => "REQUEST_FAILED",
            Self::ApiError(_) => "API_ERROR",
            Self::NotFound(_) => "NOT_FOUND",
            Self::ParseFailed => "PARSE_FAILED",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct AppConfig {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    secret_key: String,
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(&cli) {
        if cli.global.json {
            let payload = ErrorJson {
                ok: false,
                error: err.to_string(),
                code: classify_error_code(&err).to_string(),
            };
            if let Ok(out) = serde_json::to_string(&payload) {
                println!("{out}");
            } else {
                println!("{{\"ok\":false,\"error\":\"Internal serialization error\",\"code\":\"INTERNAL_ERROR\"}}");
            }
        } else {
            eprintln!("error: {err:#}");
        }
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Commands::Config(args) => handle_config(args, &cli.global),
        Commands::Domains(args) => handle_domains(args, &cli.global),
        Commands::Dns(args) => handle_dns(args, &cli.global),
        Commands::Dnssec(args) => handle_dnssec(args, &cli.global),
        Commands::Ssl(args) => handle_ssl(args, &cli.global),
    }
}

fn handle_config(args: &ConfigArgs, output: &OutputFlags) -> Result<()> {
    match &args.command {
        ConfigCommand::Set(set_args) => {
            let mut cfg = load_config_or_default()?;
            match set_args.key.as_str() {
                "api_key" => cfg.api_key = set_args.value.clone(),
                "secret_key" => cfg.secret_key = set_args.value.clone(),
                other => {
                    return Err(AppError::InvalidArgument(format!(
                        "unknown config key `{other}`; expected api_key|secret_key"
                    ))
                    .into())
                }
            }
            save_config(&cfg)?;
            output_action(output, &format!("Set {}", set_args.key))
        }
        ConfigCommand::Show => {
            let cfg = load_config_or_default()?;
            let item = serde_json::json!({
                "api_key_set": !cfg.api_key.is_empty(),
                "secret_key_set": !cfg.secret_key.is_empty(),
            });
            if output.json {
                print_json(&SuccessItem { ok: true, item })
            } else {
                println!("api_key_set={}", !cfg.api_key.is_empty());
                println!("secret_key_set={}", !cfg.secret_key.is_empty());
                Ok(())
            }
        }
        ConfigCommand::Path => {
            let path = config_path()?;
            if output.json {
                let item = serde_json::json!({ "path": path.display().to_string() });
                print_json(&SuccessItem { ok: true, item })
            } else {
                println!("{}", path.display());
                Ok(())
            }
        }
    }
}

fn handle_domains(args: &DomainsArgs, output: &OutputFlags) -> Result<()> {
    match &args.command {
        DomainsCommand::Ping => {
            let cfg = require_auth_config()?;
            let value = call_api("/ping", Map::new(), Some(&cfg), output.verbose)?;
            let item = serde_json::json!({
                "status": "ok",
                "message": value.get("yourIp").and_then(Value::as_str).unwrap_or("pong")
            });
            if output.json {
                print_json(&SuccessItem { ok: true, item })
            } else if output.quiet {
                println!("ok");
                Ok(())
            } else {
                println!("pong");
                Ok(())
            }
        }
        DomainsCommand::Pricing(pricing_args) => handle_pricing(pricing_args, output),
        DomainsCommand::ListAll(list_args) => {
            let cfg = require_auth_config()?;
            let mut body = Map::new();
            if let Some(start) = list_args.start {
                body.insert("start".to_string(), Value::String(start.to_string()));
            }
            if list_args.include_labels {
                body.insert(
                    "includeLabels".to_string(),
                    Value::String("yes".to_string()),
                );
            }
            let value = call_api("/domain/listAll", body, Some(&cfg), output.verbose)?;
            let items = value
                .get("domains")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            output_value_list(output, items)
        }
        DomainsCommand::Check(check_args) => {
            validate_domain(&check_args.domain)?;
            let cfg = require_auth_config()?;
            let path = format!("/domain/checkDomain/{}", enc(&check_args.domain));
            let value = call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            let response = value
                .get("response")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let item = serde_json::json!({
                "domain": check_args.domain,
                "available": parse_available(&value),
                "price": find_first_string(&value, &["price", "cost", "priceAmount"]),
                "currency": find_first_string(&value, &["currency", "currencySymbol"]),
                "response": response,
            });
            if output.json {
                print_json(&SuccessItem { ok: true, item })
            } else if output.quiet {
                println!(
                    "{}",
                    item.get("available")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                );
                Ok(())
            } else {
                println!("domain: {}", check_args.domain);
                println!(
                    "available: {}",
                    item.get("available")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                );
                println!(
                    "price: {}",
                    item.get("price").and_then(Value::as_str).unwrap_or("")
                );
                Ok(())
            }
        }
        DomainsCommand::Create(create_args) => {
            require_confirm(create_args.confirm)?;
            validate_domain(&create_args.domain)?;
            let cost = create_args
                .cost
                .ok_or_else(|| AppError::InvalidArgument("--cost is required".to_string()))?;
            if !create_args.agree_to_terms {
                return Err(AppError::InvalidArgument(
                    "--agree-to-terms is required for domain create".to_string(),
                )
                .into());
            }
            let cfg = require_auth_config()?;
            let mut body = Map::new();
            body.insert("cost".to_string(), Value::Number(cost.into()));
            body.insert("agreeToTerms".to_string(), Value::String("yes".to_string()));
            let path = format!("/domain/create/{}", enc(&create_args.domain));
            let value = call_api(&path, body, Some(&cfg), output.verbose)?;
            let item = serde_json::json!({
                "domain": value.get("domain").and_then(Value::as_str).unwrap_or(create_args.domain.as_str()),
                "cost": value.get("cost").cloned().unwrap_or(Value::Number(cost.into())),
                "order_id": value.get("orderId").cloned().unwrap_or_else(|| Value::String(String::new())),
                "balance": value.get("balance").cloned().unwrap_or_else(|| Value::String(String::new())),
            });
            if output.json {
                print_json(&SuccessItem { ok: true, item })
            } else {
                output_action(output, "Domain create request accepted")
            }
        }
        DomainsCommand::UpdateNs(update_args) => {
            require_confirm(update_args.confirm)?;
            validate_domain(&update_args.domain)?;
            if update_args.nameservers.is_empty() {
                return Err(
                    AppError::InvalidArgument("at least one --ns is required".to_string()).into(),
                );
            }
            let cfg = require_auth_config()?;
            let mut body = Map::new();
            body.insert(
                "ns".to_string(),
                Value::Array(
                    update_args
                        .nameservers
                        .iter()
                        .map(|x| Value::String(x.clone()))
                        .collect(),
                ),
            );
            let path = format!("/domain/updateNs/{}", enc(&update_args.domain));
            call_api(&path, body, Some(&cfg), output.verbose)?;
            output_action(output, "Nameservers updated")
        }
        DomainsCommand::GetNs(get_args) => {
            validate_domain(&get_args.domain)?;
            let cfg = require_auth_config()?;
            let path = format!("/domain/getNs/{}", enc(&get_args.domain));
            let value = call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            let items = value
                .get("ns")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            output_value_list(output, items)
        }
        DomainsCommand::UpdateAutoRenew(auto_args) => {
            require_confirm(auto_args.confirm)?;
            if auto_args.domain.is_none() && auto_args.domains.is_empty() {
                return Err(AppError::InvalidArgument(
                    "provide a domain argument or at least one --domain".to_string(),
                )
                .into());
            }
            let status = to_on_off(&auto_args.status)?;
            let cfg = require_auth_config()?;
            let mut body = Map::new();
            body.insert("status".to_string(), Value::String(status.to_string()));
            if !auto_args.domains.is_empty() {
                body.insert(
                    "domains".to_string(),
                    Value::Array(
                        auto_args
                            .domains
                            .iter()
                            .map(|x| Value::String(x.clone()))
                            .collect(),
                    ),
                );
            }
            let path = if let Some(domain) = &auto_args.domain {
                validate_domain(domain)?;
                format!("/domain/updateAutoRenew/{}", enc(domain))
            } else {
                "/domain/updateAutoRenew".to_string()
            };
            let value = call_api(&path, body, Some(&cfg), output.verbose)?;
            let item = serde_json::json!({
                "status": value.get("status").cloned().unwrap_or(Value::String("SUCCESS".to_string())),
                "results": value.get("results").cloned().unwrap_or_else(|| serde_json::json!({}))
            });
            if output.json {
                print_json(&SuccessItem { ok: true, item })
            } else {
                output_action(output, "Auto-renew updated")
            }
        }
        DomainsCommand::AddUrlForward(forward_args) => {
            require_confirm(forward_args.confirm)?;
            validate_domain(&forward_args.domain)?;
            let forward_type = match forward_args.r#type.to_ascii_lowercase().as_str() {
                "temporary" | "permanent" => forward_args.r#type.to_ascii_lowercase(),
                _ => {
                    return Err(AppError::InvalidArgument(
                        "--type must be temporary or permanent".to_string(),
                    )
                    .into())
                }
            };
            let include_path = to_yes_no(&forward_args.include_path)?;
            let wildcard = to_yes_no(&forward_args.wildcard)?;
            if !forward_args.location.starts_with("http://")
                && !forward_args.location.starts_with("https://")
            {
                return Err(AppError::InvalidArgument(
                    "--location must start with http:// or https://".to_string(),
                )
                .into());
            }
            let cfg = require_auth_config()?;
            let mut body = Map::new();
            body.insert(
                "subdomain".to_string(),
                Value::String(forward_args.subdomain.clone()),
            );
            body.insert(
                "location".to_string(),
                Value::String(forward_args.location.clone()),
            );
            body.insert("type".to_string(), Value::String(forward_type));
            body.insert(
                "includePath".to_string(),
                Value::String(include_path.to_string()),
            );
            body.insert("wildcard".to_string(), Value::String(wildcard.to_string()));
            let path = format!("/domain/addUrlForward/{}", enc(&forward_args.domain));
            call_api(&path, body, Some(&cfg), output.verbose)?;
            output_action(output, "URL forward added")
        }
        DomainsCommand::GetUrlForwarding(get_args) => {
            validate_domain(&get_args.domain)?;
            let cfg = require_auth_config()?;
            let path = format!("/domain/getUrlForwarding/{}", enc(&get_args.domain));
            let value = call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            let items = value
                .get("forwards")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            output_value_list(output, items)
        }
        DomainsCommand::DeleteUrlForward(delete_args) => {
            require_confirm(delete_args.confirm)?;
            validate_domain(&delete_args.domain)?;
            if delete_args.record_id.trim().is_empty() {
                return Err(AppError::InvalidArgument("record_id is required".to_string()).into());
            }
            let cfg = require_auth_config()?;
            let path = format!(
                "/domain/deleteUrlForward/{}/{}",
                enc(&delete_args.domain),
                enc(&delete_args.record_id)
            );
            call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            output_action(output, "URL forward deleted")
        }
        DomainsCommand::CreateGlue(glue_args) => handle_glue_upsert(glue_args, output, true),
        DomainsCommand::UpdateGlue(glue_args) => handle_glue_upsert(glue_args, output, false),
        DomainsCommand::DeleteGlue(delete_args) => {
            require_confirm(delete_args.confirm)?;
            validate_domain(&delete_args.domain)?;
            validate_non_empty("host", &delete_args.host)?;
            let cfg = require_auth_config()?;
            let path = format!(
                "/domain/deleteGlue/{}/{}",
                enc(&delete_args.domain),
                enc(&delete_args.host)
            );
            call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            output_action(output, "Glue record deleted")
        }
        DomainsCommand::GetGlue(get_args) => {
            validate_domain(&get_args.domain)?;
            let cfg = require_auth_config()?;
            let path = format!("/domain/getGlue/{}", enc(&get_args.domain));
            let value = call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            let hosts = value
                .get("hosts")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            output_value_list(output, hosts)
        }
    }
}

fn handle_dns(args: &DnsArgs, output: &OutputFlags) -> Result<()> {
    match &args.command {
        DnsCommand::Create(create_args) => {
            require_confirm(create_args.confirm)?;
            validate_domain(&create_args.domain)?;
            let cfg = require_auth_config()?;
            let mut body = dns_body_from_common(
                &create_args.r#type,
                &create_args.name,
                &create_args.content,
                create_args.ttl,
                create_args.prio,
                create_args.notes.clone(),
            )?;
            let path = format!("/dns/create/{}", enc(&create_args.domain));
            let value = call_api(&path, std::mem::take(&mut body), Some(&cfg), output.verbose)?;
            let item = serde_json::json!({
                "id": value.get("id").and_then(Value::as_str).unwrap_or(""),
            });
            if output.json {
                print_json(&SuccessItem { ok: true, item })
            } else {
                output_action(output, "DNS record created")
            }
        }
        DnsCommand::Edit(edit_args) => {
            require_confirm(edit_args.confirm)?;
            validate_domain(&edit_args.domain)?;
            validate_non_empty("record_id", &edit_args.record_id)?;
            let cfg = require_auth_config()?;
            let mut body = dns_body_from_common(
                &edit_args.r#type,
                &edit_args.name,
                &edit_args.content,
                edit_args.ttl,
                edit_args.prio,
                edit_args.notes.clone(),
            )?;
            let path = format!(
                "/dns/edit/{}/{}",
                enc(&edit_args.domain),
                enc(&edit_args.record_id)
            );
            call_api(&path, std::mem::take(&mut body), Some(&cfg), output.verbose)?;
            output_action(output, "DNS record updated")
        }
        DnsCommand::EditByNameType(edit_args) => {
            require_confirm(edit_args.confirm)?;
            validate_domain(&edit_args.domain)?;
            validate_record_type(&edit_args.record_type)?;
            let cfg = require_auth_config()?;
            let mut body = Map::new();
            body.insert(
                "content".to_string(),
                Value::String(edit_args.content.clone()),
            );
            if let Some(ttl) = edit_args.ttl {
                body.insert("ttl".to_string(), Value::String(ttl.to_string()));
            }
            if let Some(prio) = edit_args.prio {
                body.insert("prio".to_string(), Value::String(prio.to_string()));
            }
            if let Some(notes) = &edit_args.notes {
                body.insert("notes".to_string(), Value::String(notes.clone()));
            }
            let path = path_with_optional_subdomain(
                "/dns/editByNameType",
                &edit_args.domain,
                &edit_args.record_type,
                edit_args.subdomain.as_deref(),
            );
            call_api(&path, body, Some(&cfg), output.verbose)?;
            output_action(output, "DNS records updated")
        }
        DnsCommand::Delete(delete_args) => {
            require_confirm(delete_args.confirm)?;
            validate_domain(&delete_args.domain)?;
            validate_non_empty("record_id", &delete_args.record_id)?;
            let cfg = require_auth_config()?;
            let path = format!(
                "/dns/delete/{}/{}",
                enc(&delete_args.domain),
                enc(&delete_args.record_id)
            );
            call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            output_action(output, "DNS record deleted")
        }
        DnsCommand::DeleteByNameType(delete_args) => {
            require_confirm(delete_args.confirm)?;
            validate_domain(&delete_args.domain)?;
            validate_record_type(&delete_args.record_type)?;
            let cfg = require_auth_config()?;
            let path = path_with_optional_subdomain(
                "/dns/deleteByNameType",
                &delete_args.domain,
                &delete_args.record_type,
                delete_args.subdomain.as_deref(),
            );
            call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            output_action(output, "DNS records deleted")
        }
        DnsCommand::Retrieve(retrieve_args) => {
            validate_domain(&retrieve_args.domain)?;
            let cfg = require_auth_config()?;
            let path = if let Some(record_id) = &retrieve_args.record_id {
                format!(
                    "/dns/retrieve/{}/{}",
                    enc(&retrieve_args.domain),
                    enc(record_id)
                )
            } else {
                format!("/dns/retrieve/{}", enc(&retrieve_args.domain))
            };
            let value = call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            let items = value
                .get("records")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            output_value_list(output, items)
        }
        DnsCommand::RetrieveByNameType(retrieve_args) => {
            validate_domain(&retrieve_args.domain)?;
            validate_record_type(&retrieve_args.record_type)?;
            let cfg = require_auth_config()?;
            let path = path_with_optional_subdomain(
                "/dns/retrieveByNameType",
                &retrieve_args.domain,
                &retrieve_args.record_type,
                retrieve_args.subdomain.as_deref(),
            );
            let value = call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            let items = value
                .get("records")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            output_value_list(output, items)
        }
    }
}

fn handle_dnssec(args: &DnssecArgs, output: &OutputFlags) -> Result<()> {
    match &args.command {
        DnssecCommand::Create(create_args) => {
            require_confirm(create_args.confirm)?;
            validate_domain(&create_args.domain)?;
            let cfg = require_auth_config()?;
            let mut body = Map::new();
            body.insert(
                "keyTag".to_string(),
                Value::String(create_args.key_tag.clone()),
            );
            body.insert("alg".to_string(), Value::String(create_args.alg.clone()));
            body.insert(
                "digestType".to_string(),
                Value::String(create_args.digest_type.clone()),
            );
            body.insert(
                "digest".to_string(),
                Value::String(create_args.digest.clone()),
            );
            body.insert(
                "maxSigLife".to_string(),
                Value::String(create_args.max_sig_life.clone().unwrap_or_default()),
            );
            body.insert(
                "keyDataFlags".to_string(),
                Value::String(create_args.key_data_flags.clone().unwrap_or_default()),
            );
            body.insert(
                "keyDataProtocol".to_string(),
                Value::String(create_args.key_data_protocol.clone().unwrap_or_default()),
            );
            body.insert(
                "keyDataAlgo".to_string(),
                Value::String(create_args.key_data_algo.clone().unwrap_or_default()),
            );
            body.insert(
                "keyDataPubKey".to_string(),
                Value::String(create_args.key_data_pub_key.clone().unwrap_or_default()),
            );

            let path = format!("/dns/createDnssecRecord/{}", enc(&create_args.domain));
            call_api(&path, body, Some(&cfg), output.verbose)?;
            output_action(output, "DNSSEC record created")
        }
        DnssecCommand::Get(get_args) => {
            validate_domain(&get_args.domain)?;
            let cfg = require_auth_config()?;
            let path = format!("/dns/getDnssecRecords/{}", enc(&get_args.domain));
            let value = call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            let item = value
                .get("records")
                .filter(|v| !v.is_null())
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            if output.json {
                print_json(&SuccessItem { ok: true, item })
            } else if output.quiet {
                println!("{}", serde_json::to_string(&item)?);
                Ok(())
            } else {
                println!("{}", serde_json::to_string_pretty(&item)?);
                Ok(())
            }
        }
        DnssecCommand::Delete(delete_args) => {
            require_confirm(delete_args.confirm)?;
            validate_domain(&delete_args.domain)?;
            validate_non_empty("key_tag", &delete_args.key_tag)?;
            let cfg = require_auth_config()?;
            let path = format!(
                "/dns/deleteDnssecRecord/{}/{}",
                enc(&delete_args.domain),
                enc(&delete_args.key_tag)
            );
            call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            output_action(output, "DNSSEC record deleted")
        }
    }
}

fn handle_ssl(args: &SslArgs, output: &OutputFlags) -> Result<()> {
    match &args.command {
        SslCommand::Retrieve(retrieve_args) => {
            validate_domain(&retrieve_args.domain)?;
            let cfg = require_auth_config()?;
            let path = format!("/ssl/retrieve/{}", enc(&retrieve_args.domain));
            let value = call_api(&path, Map::new(), Some(&cfg), output.verbose)?;
            let item = serde_json::json!({
                "certificatechain": value.get("certificatechain").and_then(Value::as_str).unwrap_or(""),
                "privatekey": value.get("privatekey").and_then(Value::as_str).unwrap_or(""),
                "publickey": value.get("publickey").and_then(Value::as_str).unwrap_or(""),
            });
            if output.json {
                print_json(&SuccessItem { ok: true, item })
            } else if output.quiet {
                println!("{}", retrieve_args.domain);
                Ok(())
            } else {
                println!("SSL bundle retrieved for {}", retrieve_args.domain);
                println!(
                    "certificatechain: {} bytes",
                    item["certificatechain"].as_str().unwrap_or("").len()
                );
                println!(
                    "privatekey: {} bytes",
                    item["privatekey"].as_str().unwrap_or("").len()
                );
                println!(
                    "publickey: {} bytes",
                    item["publickey"].as_str().unwrap_or("").len()
                );
                Ok(())
            }
        }
    }
}

fn handle_glue_upsert(args: &GlueUpsertArgs, output: &OutputFlags, create: bool) -> Result<()> {
    require_confirm(args.confirm)?;
    validate_domain(&args.domain)?;
    validate_non_empty("host", &args.host)?;
    if args.ips.is_empty() {
        return Err(AppError::InvalidArgument("at least one --ip is required".to_string()).into());
    }
    let cfg = require_auth_config()?;
    let mut body = Map::new();
    body.insert(
        "ips".to_string(),
        Value::Array(args.ips.iter().map(|x| Value::String(x.clone())).collect()),
    );
    let action = if create { "createGlue" } else { "updateGlue" };
    let path = format!(
        "/domain/{}/{}/{}",
        action,
        enc(&args.domain),
        enc(&args.host)
    );
    call_api(&path, body, Some(&cfg), output.verbose)?;
    if create {
        output_action(output, "Glue record created")
    } else {
        output_action(output, "Glue record updated")
    }
}

fn handle_pricing(args: &PricingArgs, output: &OutputFlags) -> Result<()> {
    let cfg = load_config_or_default()?;
    let auth = if cfg.api_key.is_empty() || cfg.secret_key.is_empty() {
        None
    } else {
        Some(cfg)
    };

    let value = call_api("/pricing/get", Map::new(), auth.as_ref(), output.verbose)?;
    let pricing = value
        .get("pricing")
        .and_then(Value::as_object)
        .ok_or(AppError::ParseFailed)?;

    let mut items = Vec::new();
    for (tld, row) in pricing {
        let map = row.as_object().cloned().unwrap_or_default();
        items.push(serde_json::json!({
            "tld": tld,
            "registration": map.get("registration").and_then(Value::as_str).unwrap_or(""),
            "renewal": map.get("renewal").and_then(Value::as_str).unwrap_or(""),
            "transfer": map.get("transfer").and_then(Value::as_str).unwrap_or(""),
        }));
    }
    items.sort_by(|a, b| {
        let at = a.get("tld").and_then(Value::as_str).unwrap_or("");
        let bt = b.get("tld").and_then(Value::as_str).unwrap_or("");
        at.cmp(bt)
    });

    if let Some(filter_tld) = args.tld.as_deref() {
        let filter_tld = filter_tld.trim_start_matches('.').to_ascii_lowercase();
        let item = items
            .into_iter()
            .find(|x| x.get("tld").and_then(Value::as_str) == Some(filter_tld.as_str()))
            .ok_or_else(|| AppError::NotFound(format!("pricing for .{}", filter_tld)))?;

        if output.json {
            print_json(&SuccessItem { ok: true, item })
        } else if output.quiet {
            println!(".{}", filter_tld);
            Ok(())
        } else {
            println!(
                ".{} registration={} renewal={} transfer={}",
                filter_tld,
                item.get("registration")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                item.get("renewal").and_then(Value::as_str).unwrap_or(""),
                item.get("transfer").and_then(Value::as_str).unwrap_or("")
            );
            Ok(())
        }
    } else {
        output_value_list(output, items)
    }
}

fn output_value_list(output: &OutputFlags, items: Vec<Value>) -> Result<()> {
    if output.json {
        print_json(&SuccessList {
            ok: true,
            count: items.len(),
            items,
        })
    } else if output.quiet {
        for item in &items {
            if let Some(s) = item.as_str() {
                println!("{s}");
            } else if let Some(domain) = item.get("domain").and_then(Value::as_str) {
                println!("{domain}");
            } else if let Some(id) = item.get("id").and_then(Value::as_str) {
                println!("{id}");
            } else {
                println!("{}", serde_json::to_string(item)?);
            }
        }
        Ok(())
    } else {
        println!("Found {} item(s)", items.len());
        for item in &items {
            println!("{}", serde_json::to_string(item)?);
        }
        Ok(())
    }
}

fn output_action(output: &OutputFlags, message: &str) -> Result<()> {
    if output.json {
        print_json(&SuccessMessage {
            ok: true,
            message: message.to_string(),
        })
    } else if output.quiet {
        println!("ok");
        Ok(())
    } else {
        println!("{message}");
        Ok(())
    }
}

fn require_confirm(confirm: bool) -> Result<()> {
    if confirm {
        Ok(())
    } else {
        Err(AppError::ConfirmRequired.into())
    }
}

fn validate_non_empty(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(AppError::InvalidArgument(format!("{field} cannot be empty")).into())
    } else {
        Ok(())
    }
}

fn validate_domain(domain: &str) -> Result<()> {
    let value = domain.trim();
    if value.is_empty() || !value.contains('.') || value.contains(' ') {
        return Err(AppError::InvalidArgument(format!("invalid domain `{domain}`")).into());
    }
    Ok(())
}

fn validate_record_type(record_type: &str) -> Result<()> {
    validate_non_empty("record_type", record_type)?;
    let t = record_type.to_ascii_uppercase();
    let allowed = [
        "A", "MX", "CNAME", "ALIAS", "TXT", "NS", "AAAA", "SRV", "TLSA", "CAA", "HTTPS", "SVCB",
        "SSHFP",
    ];
    if allowed.contains(&t.as_str()) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("unsupported record type `{record_type}`")).into())
    }
}

fn to_yes_no(value: &str) -> Result<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "yes" | "y" | "1" | "true" => Ok("yes"),
        "no" | "n" | "0" | "false" => Ok("no"),
        _ => Err(AppError::InvalidArgument(format!("expected yes/no value, got `{value}`")).into()),
    }
}

fn to_on_off(value: &str) -> Result<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "on" | "1" | "true" => Ok("on"),
        "off" | "0" | "false" => Ok("off"),
        _ => Err(AppError::InvalidArgument(format!("expected on/off value, got `{value}`")).into()),
    }
}

fn dns_body_from_common(
    record_type: &str,
    name: &str,
    content: &str,
    ttl: Option<u32>,
    prio: Option<u32>,
    notes: Option<String>,
) -> Result<Map<String, Value>> {
    validate_record_type(record_type)?;
    validate_non_empty("content", content)?;
    let mut body = Map::new();
    body.insert(
        "type".to_string(),
        Value::String(record_type.to_ascii_uppercase()),
    );
    body.insert("name".to_string(), Value::String(name.to_string()));
    body.insert("content".to_string(), Value::String(content.to_string()));
    if let Some(ttl) = ttl {
        body.insert("ttl".to_string(), Value::String(ttl.to_string()));
    }
    if let Some(prio) = prio {
        body.insert("prio".to_string(), Value::String(prio.to_string()));
    }
    if let Some(notes) = notes {
        body.insert("notes".to_string(), Value::String(notes));
    }
    Ok(body)
}

fn path_with_optional_subdomain(
    prefix: &str,
    domain: &str,
    record_type: &str,
    subdomain: Option<&str>,
) -> String {
    let domain_enc = enc(domain);
    let type_enc = enc(&record_type.to_ascii_uppercase());
    if let Some(sub) = subdomain {
        format!("{prefix}/{domain_enc}/{type_enc}/{}", enc(sub))
    } else {
        format!("{prefix}/{domain_enc}/{type_enc}")
    }
}

fn enc(input: &str) -> String {
    urlencoding::encode(input).to_string()
}

fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir().ok_or(AppError::ConfigDirUnavailable)?;
    Ok(dir.join("dee-porkbun").join("config.toml"))
}

fn load_config_or_default() -> Result<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed reading config file {}", path.display()))?;
    let cfg = toml::from_str::<AppConfig>(&raw)
        .with_context(|| format!("failed parsing config file {}", path.display()))?;
    Ok(cfg)
}

fn require_auth_config() -> Result<AppConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Err(AppError::ConfigMissing.into());
    }
    let cfg = load_config_or_default()?;
    if cfg.api_key.is_empty() || cfg.secret_key.is_empty() {
        return Err(AppError::AuthMissing.into());
    }
    Ok(cfg)
}

fn save_config(cfg: &AppConfig) -> Result<()> {
    let path = config_path()?;
    ensure_parent_dir(&path)?;
    let raw = toml::to_string(cfg)?;
    fs::write(&path, raw)
        .with_context(|| format!("failed writing config file {}", path.display()))?;
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    let parent = path.parent().ok_or(AppError::ConfigDirUnavailable)?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed creating config directory {}", parent.display()))?;
    Ok(())
}

fn call_api(
    path: &str,
    mut body: Map<String, Value>,
    cfg: Option<&AppConfig>,
    verbose: bool,
) -> Result<Value> {
    if let Some(cfg) = cfg {
        body.insert("apikey".to_string(), Value::String(cfg.api_key.clone()));
        body.insert(
            "secretapikey".to_string(),
            Value::String(cfg.secret_key.clone()),
        );
    }

    let url = format!("{}{}", API_BASE, path);
    if verbose {
        eprintln!("debug: POST {url}");
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("dee-porkbun/0.2.0 (https://dee.ink)")
        .build()
        .map_err(|e| AppError::RequestFailed(e.to_string()))?;

    let response = client
        .post(url)
        .json(&body)
        .send()
        .map_err(|e| AppError::RequestFailed(e.to_string()))?;
    let status_code = response.status();
    let response_text = response
        .text()
        .map_err(|e| AppError::RequestFailed(e.to_string()))?;

    let value: Value = serde_json::from_str(&response_text).map_err(|_| {
        if status_code.is_success() {
            AppError::ParseFailed
        } else {
            AppError::RequestFailed(format!("HTTP {} with non-JSON body", status_code))
        }
    })?;

    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if status.eq_ignore_ascii_case("SUCCESS") {
        return Ok(value);
    }

    let message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("unknown API error");
    let expanded = if status_code.is_success() {
        message.to_string()
    } else {
        format!("{} (HTTP {})", message, status_code)
    };
    Err(AppError::ApiError(expanded).into())
}

fn parse_available(value: &Value) -> bool {
    if let Some(v) = value.get("available") {
        return parse_boolish(v);
    }
    if let Some(response) = value.get("response") {
        if let Some(v) = response.get("available") {
            return parse_boolish(v);
        }
        if let Some(v) = response.get("avail") {
            return parse_boolish(v);
        }
    }
    false
}

fn parse_boolish(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_i64().unwrap_or_default() != 0,
        Value::String(s) => matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "y"),
        _ => false,
    }
}

fn find_first_string(value: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(s) = value.get(*key).and_then(Value::as_str) {
            return s.to_string();
        }
        if let Some(s) = value
            .get("response")
            .and_then(|v| v.get(*key))
            .and_then(Value::as_str)
        {
            return s.to_string();
        }
    }
    String::new()
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn classify_error_code(err: &anyhow::Error) -> &'static str {
    if let Some(app) = err.downcast_ref::<AppError>() {
        return app.code();
    }
    "INTERNAL_ERROR"
}

#[allow(dead_code)]
fn stable_map(value: &Map<String, Value>) -> BTreeMap<String, Value> {
    value.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}
