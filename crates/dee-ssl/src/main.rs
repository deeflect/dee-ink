use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use clap::{ArgAction, Args, Parser, Subcommand};
use rustls::client::ClientConnection;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, RootCertStore, StreamOwned};
use serde::Serialize;
use thiserror::Error;
use x509_parser::extensions::ParsedExtension;
use x509_parser::prelude::FromDer;

#[derive(Parser, Debug)]
#[command(
    name = "dee-ssl",
    version,
    about = "SSL certificate checker for domains",
    after_help = "EXAMPLES:\n  dee-ssl check example.com\n  dee-ssl check example.com --chain\n  dee-ssl check example.com --warn-days 30\n  dee-ssl check example.com --json\n  dee-ssl check example.com --port 8443\n  dee-ssl check example.com --timeout-secs 5"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short = 'j', long, global = true, action = ArgAction::SetTrue)]
    json: bool,

    #[arg(short = 'q', long, global = true, action = ArgAction::SetTrue)]
    quiet: bool,

    #[arg(short = 'v', long, global = true, action = ArgAction::SetTrue)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check SSL certificate details for a domain
    Check(CheckArgs),
}

#[derive(Args, Debug)]
struct CheckArgs {
    /// Domain to check
    domain: String,

    /// TLS port
    #[arg(long, default_value_t = 443)]
    port: u16,

    /// Show full certificate chain
    #[arg(long, action = ArgAction::SetTrue)]
    chain: bool,

    /// Exit with code 1 when cert expires in N days or less
    #[arg(long, default_value_t = 0)]
    warn_days: i64,

    /// Connection and handshake timeout in seconds
    #[arg(long, default_value_t = 10)]
    timeout_secs: u64,
}

#[derive(Debug, Error, Clone)]
enum AppError {
    #[error("failed to resolve address for {domain}:{port}")]
    ResolveAddress { domain: String, port: u16 },
    #[error("failed TLS handshake with {domain}:{port}: {reason}")]
    TlsHandshake {
        domain: String,
        port: u16,
        reason: String,
    },
    #[error("no peer certificates presented by {domain}:{port}")]
    MissingCertificate { domain: String, port: u16 },
    #[error("certificate parsing failed: {reason}")]
    ParseCert { reason: String },
    #[error(
        "certificate expires within warning window ({days_until_expiry} days <= {warn_days} days)"
    )]
    ExpiringSoon {
        days_until_expiry: i64,
        warn_days: i64,
    },
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::ResolveAddress { .. } => "RESOLVE_FAILED",
            Self::TlsHandshake { .. } => "TLS_HANDSHAKE_FAILED",
            Self::MissingCertificate { .. } => "MISSING_CERTIFICATE",
            Self::ParseCert { .. } => "PARSE_CERT_FAILED",
            Self::ExpiringSoon { .. } => "EXPIRING_SOON",
        }
    }
}

#[derive(Debug, Serialize)]
struct JsonError<'a> {
    ok: bool,
    error: &'a str,
    code: &'a str,
}

#[derive(Debug, Serialize)]
struct CertItem {
    domain: String,
    port: u16,
    valid: bool,
    expires: String,
    days_until_expiry: i64,
    issuer: String,
    subject: String,
    sans: Vec<String>,
    chain_depth: usize,
}

#[derive(Debug, Serialize)]
struct ChainCertItem {
    index: usize,
    subject: String,
    issuer: String,
    not_before: String,
    not_after: String,
}

#[derive(Debug, Serialize)]
struct SingleOk<T> {
    ok: bool,
    item: T,
}

#[derive(Debug, Serialize)]
struct ListOk<T> {
    ok: bool,
    count: usize,
    items: Vec<T>,
}

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(&cli) {
        let app_err =
            err.downcast_ref::<AppError>()
                .cloned()
                .unwrap_or_else(|| AppError::ParseCert {
                    reason: err.to_string(),
                });

        if cli.json {
            let payload = JsonError {
                ok: false,
                error: &app_err.to_string(),
                code: app_err.code(),
            };
            if let Ok(serialized) = serde_json::to_string(&payload) {
                println!("{serialized}");
            } else {
                println!("{{\"ok\":false,\"error\":\"serialization failed\",\"code\":\"SERIALIZE_ERROR\"}}");
            }
        } else {
            eprintln!("error: {app_err}");
        }
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Commands::Check(args) => handle_check(cli, args),
    }
}

fn handle_check(cli: &Cli, args: &CheckArgs) -> Result<()> {
    let certs = fetch_cert_chain(&args.domain, args.port, cli.verbose, args.timeout_secs)?;
    let leaf = certs.first().ok_or_else(|| AppError::MissingCertificate {
        domain: args.domain.clone(),
        port: args.port,
    })?;

    let parsed = parse_cert(leaf)?;
    let expires = parsed.not_after.clone();
    let expires_dt = parse_rfc3339_utc(&expires)?;
    let now = Utc::now();
    let days_until_expiry = expires_dt.signed_duration_since(now).num_days();

    if args.warn_days > 0 && days_until_expiry <= args.warn_days {
        return Err(AppError::ExpiringSoon {
            days_until_expiry,
            warn_days: args.warn_days,
        }
        .into());
    }

    if args.chain {
        let items = certs
            .iter()
            .enumerate()
            .map(|(index, cert)| cert_to_chain_item(index, cert))
            .collect::<Result<Vec<_>>>()?;

        if cli.json {
            let payload = ListOk {
                ok: true,
                count: items.len(),
                items,
            };
            println!("{}", serde_json::to_string(&payload)?);
            return Ok(());
        }

        if !cli.quiet {
            println!("Certificate chain for {}:{}", args.domain, args.port);
            for item in &items {
                println!(
                    "[{}] {}\n     issuer: {}\n     valid: {} → {}",
                    item.index, item.subject, item.issuer, item.not_before, item.not_after
                );
            }
        } else {
            for item in &items {
                println!("{}", item.subject);
            }
        }
        return Ok(());
    }

    let item = CertItem {
        domain: args.domain.clone(),
        port: args.port,
        valid: parsed
            .x509
            .validity()
            .is_valid_at(x509_parser::time::ASN1Time::now()),
        expires,
        days_until_expiry,
        issuer: parsed.issuer,
        subject: parsed.subject,
        sans: parsed.sans,
        chain_depth: certs.len(),
    };

    if cli.json {
        let payload = SingleOk { ok: true, item };
        println!("{}", serde_json::to_string(&payload)?);
        return Ok(());
    }

    if cli.quiet {
        println!("{}", item.expires);
    } else {
        println!("Domain: {}:{}", item.domain, item.port);
        println!("Valid now: {}", item.valid);
        println!(
            "Expires: {} ({} days)",
            item.expires, item.days_until_expiry
        );
        println!("Issuer: {}", item.issuer);
        println!("Subject: {}", item.subject);
        println!("SANs: {}", item.sans.join(", "));
        println!("Chain depth: {}", item.chain_depth);
    }

    Ok(())
}

fn fetch_cert_chain(
    domain: &str,
    port: u16,
    verbose: bool,
    timeout_secs: u64,
) -> Result<Vec<CertificateDer<'static>>> {
    let timeout = Duration::from_secs(timeout_secs);
    let addr = format!("{domain}:{port}");
    let mut addrs = addr
        .to_socket_addrs()
        .map_err(|_| AppError::ResolveAddress {
            domain: domain.to_string(),
            port,
        })?;
    let target = addrs.next().ok_or_else(|| AppError::ResolveAddress {
        domain: domain.to_string(),
        port,
    })?;

    let stream =
        TcpStream::connect_timeout(&target, timeout).map_err(|e| AppError::TlsHandshake {
            domain: domain.to_string(),
            port,
            reason: e.to_string(),
        })?;

    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| AppError::TlsHandshake {
            domain: domain.to_string(),
            port,
            reason: e.to_string(),
        })?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| AppError::TlsHandshake {
            domain: domain.to_string(),
            port,
            reason: e.to_string(),
        })?;

    let mut roots = RootCertStore::empty();
    let cert_result = rustls_native_certs::load_native_certs();
    for cert in cert_result.certs {
        if let Err(error) = roots.add(cert) {
            if verbose {
                eprintln!("warning: failed to add root cert: {error}");
            }
        }
    }

    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();

    let server_name =
        ServerName::try_from(domain.to_string()).map_err(|e| AppError::TlsHandshake {
            domain: domain.to_string(),
            port,
            reason: e.to_string(),
        })?;

    let conn = ClientConnection::new(Arc::new(config), server_name).map_err(|e| {
        AppError::TlsHandshake {
            domain: domain.to_string(),
            port,
            reason: e.to_string(),
        }
    })?;

    let mut tls = StreamOwned::new(conn, stream);
    tls.flush().map_err(|e| AppError::TlsHandshake {
        domain: domain.to_string(),
        port,
        reason: e.to_string(),
    })?;

    let certs = tls
        .conn
        .peer_certificates()
        .ok_or_else(|| AppError::MissingCertificate {
            domain: domain.to_string(),
            port,
        })?;

    Ok(certs.to_vec())
}

struct ParsedCert<'a> {
    x509: x509_parser::certificate::X509Certificate<'a>,
    issuer: String,
    subject: String,
    sans: Vec<String>,
    not_before: String,
    not_after: String,
}

fn parse_cert<'a>(cert: &'a CertificateDer<'a>) -> Result<ParsedCert<'a>> {
    let (_, x509) =
        x509_parser::certificate::X509Certificate::from_der(cert.as_ref()).map_err(|e| {
            AppError::ParseCert {
                reason: e.to_string(),
            }
        })?;

    let issuer = x509.issuer().to_string();
    let subject = x509.subject().to_string();

    let sans = x509
        .extensions()
        .iter()
        .find_map(|ext| {
            if let ParsedExtension::SubjectAlternativeName(san) = ext.parsed_extension() {
                Some(
                    san.general_names
                        .iter()
                        .filter_map(|name| match name {
                            x509_parser::extensions::GeneralName::DNSName(value) => {
                                Some((*value).to_string())
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default();

    let not_before = as_utc_string(x509.validity().not_before)?;
    let not_after = as_utc_string(x509.validity().not_after)?;

    Ok(ParsedCert {
        x509,
        issuer,
        subject,
        sans,
        not_before,
        not_after,
    })
}

fn cert_to_chain_item(index: usize, cert: &CertificateDer<'_>) -> Result<ChainCertItem> {
    let parsed = parse_cert(cert)?;

    Ok(ChainCertItem {
        index,
        subject: parsed.subject,
        issuer: parsed.issuer,
        not_before: parsed.not_before,
        not_after: parsed.not_after,
    })
}

fn as_utc_string(time: x509_parser::time::ASN1Time) -> Result<String> {
    let offset = time.to_datetime();
    let timestamp = offset.unix_timestamp();
    let dt = DateTime::<Utc>::from_timestamp(timestamp, 0).ok_or_else(|| AppError::ParseCert {
        reason: "invalid certificate timestamp".to_string(),
    })?;
    Ok(dt.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn parse_rfc3339_utc(input: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(input).map_err(|e| AppError::ParseCert {
        reason: e.to_string(),
    })?;
    Ok(parsed.with_timezone(&Utc))
}
