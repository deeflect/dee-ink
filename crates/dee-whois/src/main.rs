use anyhow::Result;
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use clap::Parser;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeSet;
use std::io::{self, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[command(
    name = "dee-whois",
    version,
    about = "WHOIS lookup for domains and IPs",
    long_about = "dee-whois - WHOIS lookup for domains and IPs\n\nUSAGE:\n  dee-whois <domain-or-ip> [options]",
    after_help = "EXAMPLES:\n  dee-whois example.com\n  dee-whois example.com --json\n  dee-whois example.com --raw\n  dee-whois example.com --expires --json\n  dee-whois 8.8.8.8 --json"
)]
struct Cli {
    /// Domain or IP to look up
    target: String,

    /// Output raw WHOIS text
    #[arg(long)]
    raw: bool,

    /// Only show expiry information
    #[arg(long)]
    expires: bool,

    /// Output as JSON
    #[arg(short, long)]
    json: bool,

    /// Suppress decorative output
    #[arg(short, long)]
    quiet: bool,

    /// Debug output to stderr
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Serialize)]
struct WhoisItem {
    domain: String,
    registrar: String,
    created: String,
    expires: String,
    updated: String,
    name_servers: Vec<String>,
    status: Vec<String>,
    days_until_expiry: i64,
    whois_server: String,
}

#[derive(Debug, Serialize)]
struct ExpiresItem {
    domain: String,
    expires: String,
    days_until_expiry: i64,
    expired: bool,
}

#[derive(Debug, Serialize)]
struct JsonSuccessItem<T: Serialize> {
    ok: bool,
    item: T,
}

#[derive(Debug, Serialize)]
struct JsonError {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, thiserror::Error)]
enum WhoisError {
    #[error("{0}")]
    InvalidArgument(String),
    #[allow(dead_code)]
    #[error("WHOIS lookup failed: {0}")]
    LookupFailed(String),
    #[allow(dead_code)]
    #[error("Connection to WHOIS server failed: {0}")]
    ConnectionFailed(String),
}

impl WhoisError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::LookupFailed(_) => "WHOIS_LOOKUP_FAILED",
            Self::ConnectionFailed(_) => "NETWORK_ERROR",
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(&cli).await {
        let message = format!("{err:#}");
        if cli.json {
            let code = err
                .downcast_ref::<WhoisError>()
                .map(WhoisError::code)
                .unwrap_or("WHOIS_LOOKUP_FAILED");
            let payload = JsonError {
                ok: false,
                error: message,
                code: code.to_string(),
            };
            let _ = print_json(&payload);
        } else {
            eprintln!("error: {message}");
        }
        std::process::exit(1);
    }
}

async fn run(cli: &Cli) -> Result<()> {
    if cli.raw && cli.expires {
        anyhow::bail!(WhoisError::InvalidArgument(
            "--raw and --expires cannot be used together".to_string()
        ));
    }

    let server = whois_server_for_target(&cli.target);
    if cli.verbose {
        eprintln!("querying {} via {}", cli.target, server);
    }

    let raw = query_whois(&server, &cli.target).await?;

    // For .com/.net, attempt a two-step referral lookup if the response includes a Whois Server
    let (raw, final_server) = if should_try_referral(&server) {
        if let Some(referral) = extract_referral_server(&raw) {
            if referral != server && !referral.is_empty() {
                if cli.verbose {
                    eprintln!("referral: re-querying via {referral}");
                }
                match query_whois(&referral, &cli.target).await {
                    Ok(referral_raw) => (referral_raw, referral),
                    Err(_) => (raw, server), // fall back to registry response
                }
            } else {
                (raw, server)
            }
        } else {
            (raw, server)
        }
    } else {
        (raw, server)
    };

    if cli.raw {
        return output_raw(cli, &raw);
    }

    let parsed = parse_whois(&cli.target, &final_server, &raw);

    if cli.expires {
        let expires = ExpiresItem {
            domain: parsed.domain,
            expires: parsed.expires,
            days_until_expiry: parsed.days_until_expiry,
            expired: parsed.days_until_expiry < 0,
        };
        return output_expires(cli, &expires);
    }

    output_item(cli, &parsed)
}

fn should_try_referral(server: &str) -> bool {
    // Verisign is the registry server for .com/.net; their response includes referral info
    server.contains("verisign") || server.contains("iana.org")
}

fn extract_referral_server(raw: &str) -> Option<String> {
    raw.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("Whois Server:")
            .or_else(|| trimmed.strip_prefix("whois:"))
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty())
    })
}

fn output_raw(cli: &Cli, raw: &str) -> Result<()> {
    if cli.json {
        #[derive(Serialize)]
        struct RawItem<'a> {
            target: &'a str,
            raw: &'a str,
        }
        let payload = JsonSuccessItem {
            ok: true,
            item: RawItem {
                target: &cli.target,
                raw,
            },
        };
        print_json(&payload)
    } else {
        println!("{raw}");
        Ok(())
    }
}

fn output_expires(cli: &Cli, item: &ExpiresItem) -> Result<()> {
    if cli.json {
        let payload = JsonSuccessItem { ok: true, item };
        print_json(&payload)
    } else if cli.quiet {
        println!("{}", item.expires);
        Ok(())
    } else {
        println!("Domain: {}", item.domain);
        println!("Expires: {}", item.expires);
        println!("Days until expiry: {}", item.days_until_expiry);
        Ok(())
    }
}

fn output_item(cli: &Cli, item: &WhoisItem) -> Result<()> {
    if cli.json {
        let payload = JsonSuccessItem { ok: true, item };
        print_json(&payload)
    } else if cli.quiet {
        println!("{}", item.domain);
        Ok(())
    } else {
        println!("Domain: {}", item.domain);
        println!("Registrar: {}", item.registrar);
        println!("Created: {}", item.created);
        println!("Updated: {}", item.updated);
        println!("Expires: {}", item.expires);
        println!("Days until expiry: {}", item.days_until_expiry);
        println!("WHOIS server: {}", item.whois_server);
        println!("Name servers: {}", item.name_servers.join(", "));
        println!("Status: {}", item.status.join(", "));
        Ok(())
    }
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(&mut lock)?;
    Ok(())
}

async fn query_whois(server: &str, query: &str) -> Result<String> {
    let mut stream = TcpStream::connect((server, 43)).await.map_err(|e| {
        WhoisError::ConnectionFailed(format!("failed to connect to WHOIS server {server}: {e}"))
    })?;
    stream
        .write_all(format!("{query}\r\n").as_bytes())
        .await
        .map_err(|e| WhoisError::ConnectionFailed(format!("failed to send WHOIS query: {e}")))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .map_err(|e| WhoisError::ConnectionFailed(format!("failed to read WHOIS response: {e}")))?;

    String::from_utf8(response).map_err(|e| {
        WhoisError::LookupFailed(format!("WHOIS response was not valid UTF-8: {e}")).into()
    })
}

fn whois_server_for_target(target: &str) -> String {
    let lower = target.trim().to_ascii_lowercase();
    if lower.parse::<std::net::IpAddr>().is_ok() {
        return "whois.arin.net".to_string();
    }

    let tld = lower.rsplit('.').next().unwrap_or("com");
    match tld {
        "com" | "net" => "whois.verisign-grs.com".to_string(),
        "org" => "whois.pir.org".to_string(),
        "io" => "whois.nic.io".to_string(),
        "co" => "whois.nic.co".to_string(),
        other => format!("whois.nic.{other}"),
    }
}

fn parse_whois(target: &str, server: &str, raw: &str) -> WhoisItem {
    let registrar = extract_first(raw, &["Registrar:", "Sponsoring Registrar:"])
        .unwrap_or_else(|| "unknown".to_string());

    let created = extract_date(raw, &["Creation Date:", "Created On:", "Created:"])
        .unwrap_or_else(|| "unknown".to_string());
    let updated = extract_date(raw, &["Updated Date:", "Last Updated On:", "Updated:"])
        .unwrap_or_else(|| "unknown".to_string());
    let expires = extract_date(
        raw,
        &[
            "Registry Expiry Date:",
            "Expiration Date:",
            "Registrar Registration Expiration Date:",
            "paid-till:",
        ],
    )
    .unwrap_or_else(|| "unknown".to_string());

    let name_servers = extract_multi(raw, &["Name Server:", "nserver:"]);
    let status = extract_multi(raw, &["Domain Status:", "Status:"]);

    let days_until_expiry = if expires == "unknown" {
        0
    } else {
        parse_any_date(&expires)
            .map(|dt| (dt - Utc::now()).num_days())
            .unwrap_or(0)
    };

    WhoisItem {
        domain: target.to_string(),
        registrar,
        created,
        expires,
        updated,
        name_servers,
        status,
        days_until_expiry,
        whois_server: server.to_string(),
    }
}

fn extract_first(raw: &str, keys: &[&str]) -> Option<String> {
    raw.lines().find_map(|line| {
        let trimmed = line.trim();
        keys.iter().find_map(|k| {
            trimmed
                .strip_prefix(k)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string)
        })
    })
}

fn extract_multi(raw: &str, keys: &[&str]) -> Vec<String> {
    let mut set = BTreeSet::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        for key in keys {
            if let Some(value) = trimmed
                .strip_prefix(key)
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                set.insert(value.to_ascii_lowercase());
            }
        }
    }

    if set.is_empty() {
        vec!["unknown".to_string()]
    } else {
        set.into_iter().collect()
    }
}

fn extract_date(raw: &str, keys: &[&str]) -> Option<String> {
    raw.lines().find_map(|line| {
        let trimmed = line.trim();
        keys.iter().find_map(|k| {
            trimmed
                .strip_prefix(k)
                .map(str::trim)
                .and_then(parse_any_date)
                .map(|dt| dt.to_rfc3339())
        })
    })
}

fn parse_any_date(input: &str) -> Option<DateTime<Utc>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let dt_formats = [
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S%.fZ",
        "%Y-%m-%d %H:%M:%S",
        "%Y.%m.%d %H:%M:%S",
    ];
    if let Some(parsed) = dt_formats
        .iter()
        .find_map(|f| chrono::NaiveDateTime::parse_from_str(trimmed, f).ok())
    {
        return Some(Utc.from_utc_datetime(&parsed));
    }

    if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(parsed.with_timezone(&Utc));
    }

    let date_formats = ["%Y-%m-%d", "%Y.%m.%d", "%d-%b-%Y", "%Y/%m/%d"];
    if let Some(parsed) = date_formats
        .iter()
        .find_map(|f| NaiveDate::parse_from_str(trimmed, f).ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
    {
        return Some(Utc.from_utc_datetime(&parsed));
    }

    let re = Regex::new(r"\d{4}-\d{2}-\d{2}").ok()?;
    re.find(trimmed)
        .and_then(|m| NaiveDate::parse_from_str(m.as_str(), "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|naive| Utc.from_utc_datetime(&naive))
}
