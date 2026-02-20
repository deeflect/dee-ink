use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use image::Luma;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::Serialize;
use thiserror::Error;

#[derive(Parser, Debug)]
#[command(
    name = "dee-qr",
    version,
    about = "dee-qr - QR Code Generate & Decode CLI",
    long_about = "dee-qr - QR Code Generate & Decode CLI\n\nUSAGE:\n  dee-qr <command> [options]",
    after_help = "COMMANDS:\n  generate   Generate a QR code from text\n  decode     Decode a QR code from an image\n\nOPTIONS:\n  -j, --json       Output as JSON\n  -q, --quiet      Suppress decorative output\n  -v, --verbose    Debug output to stderr\n  -h, --help       Show this help\n  -V, --version    Show version\n\nEXAMPLES:\n  dee-qr generate \"https://example.com\" --out qr.png\n  dee-qr generate \"hello\" --format svg --out qr.svg --json\n  dee-qr generate \"terminal demo\" --format terminal\n  dee-qr decode qr.png\n  dee-qr decode qr.png --json"
)]
struct Cli {
    #[command(flatten)]
    global: GlobalFlags,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug, Clone)]
struct GlobalFlags {
    #[arg(short = 'j', long = "json", global = true)]
    json: bool,

    #[arg(short = 'q', long = "quiet", global = true)]
    quiet: bool,

    #[arg(short = 'v', long = "verbose", global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a QR code from text
    Generate(GenerateArgs),
    /// Decode a QR code from an image
    Decode(DecodeArgs),
}

#[derive(Args, Debug)]
#[command(
    about = "Generate a QR code from text",
    after_help = "EXAMPLES:\n  dee-qr generate \"https://example.com\" --out qr.png\n  dee-qr generate \"hello\" --format svg --out qr.svg --json\n  dee-qr generate \"scan me\" --format terminal\n  echo \"https://example.com\" | dee-qr generate --stdin --format terminal"
)]
struct GenerateArgs {
    /// Text content to encode (omit when using --stdin)
    #[arg(required_unless_present = "stdin")]
    text: Option<String>,

    /// Read text to encode from stdin
    #[arg(long)]
    stdin: bool,

    /// Output path for png/svg (optional for terminal)
    #[arg(long)]
    out: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = OutputFormat::Png)]
    format: OutputFormat,
}

#[derive(Args, Debug)]
#[command(
    about = "Decode a QR code from an image",
    after_help = "EXAMPLES:\n  dee-qr decode qr.png\n  dee-qr decode qr.png --json\n  dee-qr decode qr.png --quiet"
)]
struct DecodeArgs {
    /// Path to image file containing QR code
    image: PathBuf,
}

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
enum OutputFormat {
    Png,
    Svg,
    Terminal,
}

#[derive(Debug, Error)]
enum AppError {
    #[error("Missing required argument: --out for format {0}")]
    MissingOut(String),

    #[error("No QR code found in image")]
    QrNotFound,

    #[error("Failed to decode QR payload")]
    DecodeFailed,

    #[error("Unsupported image format for path: {0}")]
    UnsupportedImage(String),

    #[error("Image file not found: {0}")]
    FileNotFound(String),
}

#[derive(Serialize)]
struct JsonError {
    ok: bool,
    error: String,
    code: &'static str,
}

#[derive(Serialize)]
struct GenerateJson {
    ok: bool,
    message: String,
    path: String,
    data: String,
    format: OutputFormat,
}

#[derive(Serialize)]
struct DecodeItem {
    data: String,
    format: String,
    version: i32,
}

#[derive(Serialize)]
struct DecodeJson {
    ok: bool,
    item: DecodeItem,
}

fn main() {
    if let Err(err) = run() {
        // Check if json mode was requested from raw args (run() may have failed during parse)
        let json = std::env::args().any(|a| a == "--json" || a == "-j");
        if json {
            let payload = JsonError {
                ok: false,
                error: err.to_string(),
                code: "INTERNAL_ERROR",
            };
            if let Ok(out) = serde_json::to_string_pretty(&payload) {
                println!("{out}");
            } else {
                println!(
                    "{{\"ok\":false,\"error\":\"internal error\",\"code\":\"INTERNAL_ERROR\"}}"
                );
            }
        } else {
            eprintln!("{err}");
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.global.verbose {
        eprintln!("debug: parsed command");
    }

    let result = match cli.command {
        Commands::Generate(args) => handle_generate(args, &cli.global),
        Commands::Decode(args) => handle_decode(args, &cli.global),
    };

    if let Err(err) = result {
        if cli.global.json {
            let (message, code) = classify_error(&err);
            let payload = JsonError {
                ok: false,
                error: message,
                code,
            };
            let json = serde_json::to_string_pretty(&payload)?;
            println!("{json}");
            std::process::exit(1);
        }

        return Err(err);
    }

    Ok(())
}

fn handle_generate(args: GenerateArgs, global: &GlobalFlags) -> Result<()> {
    let text = if args.stdin {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("failed to read from stdin")?;
        buf.trim_end_matches('\n').to_string()
    } else {
        args.text.unwrap_or_default()
    };

    let qr = QrCode::new(text.as_bytes())?;

    match args.format {
        OutputFormat::Png => {
            let out = require_out(args.out, "png")?;
            let img = qr.render::<Luma<u8>>().build();
            img.save(&out)?;
            let abs = absolute_path(&out)?;
            emit_generate_output(&text, OutputFormat::Png, &abs, global)?;
        }
        OutputFormat::Svg => {
            let out = require_out(args.out, "svg")?;
            let rendered = qr
                .render::<svg::Color<'_>>()
                .min_dimensions(256, 256)
                .build();
            fs::write(&out, rendered)?;
            let abs = absolute_path(&out)?;
            emit_generate_output(&text, OutputFormat::Svg, &abs, global)?;
        }
        OutputFormat::Terminal => {
            let rendered = qr
                .render::<qrcode::render::unicode::Dense1x2>()
                .dark_color(qrcode::render::unicode::Dense1x2::Dark)
                .light_color(qrcode::render::unicode::Dense1x2::Light)
                .build();

            if global.json {
                let payload = GenerateJson {
                    ok: true,
                    message: "QR code rendered to terminal".to_string(),
                    path: "terminal".to_string(),
                    data: text,
                    format: OutputFormat::Terminal,
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("{rendered}");
            }
        }
    }

    Ok(())
}

fn handle_decode(args: DecodeArgs, global: &GlobalFlags) -> Result<()> {
    ensure_supported_image(&args.image)?;

    if !args.image.exists() {
        return Err(AppError::FileNotFound(args.image.display().to_string()).into());
    }

    let image = image::open(&args.image)?;
    let gray = image.to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(gray);
    let grids = prepared.detect_grids();

    if grids.is_empty() {
        return Err(AppError::QrNotFound.into());
    }

    let mut decoded_data = String::new();
    let mut version = 0;

    for grid in grids {
        match grid.decode() {
            Ok((meta, content)) => {
                decoded_data = content;
                version = i32::try_from(meta.version.0)?;
                break;
            }
            Err(_) => continue,
        }
    }

    if decoded_data.is_empty() {
        return Err(AppError::DecodeFailed.into());
    }

    if global.json {
        let payload = DecodeJson {
            ok: true,
            item: DecodeItem {
                data: decoded_data,
                format: "QR_CODE".to_string(),
                version,
            },
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if global.quiet {
        println!("{decoded_data}");
    } else {
        println!("Data: {decoded_data}");
        println!("Format: QR_CODE");
        println!("Version: {version}");
    }

    Ok(())
}

fn emit_generate_output(
    text: &str,
    format: OutputFormat,
    abs_path: &Path,
    global: &GlobalFlags,
) -> Result<()> {
    let path_str = abs_path.display().to_string();

    if global.json {
        let payload = GenerateJson {
            ok: true,
            message: format!("QR code saved to {path_str}"),
            path: path_str,
            data: text.to_string(),
            format,
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if global.quiet {
        println!("{}", abs_path.display());
    } else {
        println!("Saved {:?} QR to {}", format, abs_path.display());
    }

    Ok(())
}

fn require_out(out: Option<PathBuf>, format_name: &str) -> Result<PathBuf> {
    match out {
        Some(path) => Ok(path),
        None => Err(AppError::MissingOut(format_name.to_string()).into()),
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(std::env::current_dir()?.join(path))
}

fn ensure_supported_image(path: &Path) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();

    let supported = ["png", "jpg", "jpeg", "gif", "bmp", "webp", "tiff", "tif"];
    if supported.contains(&ext.as_str()) {
        Ok(())
    } else {
        Err(AppError::UnsupportedImage(path.display().to_string()).into())
    }
}

fn classify_error(err: &anyhow::Error) -> (String, &'static str) {
    if let Some(app) = err.downcast_ref::<AppError>() {
        match app {
            AppError::MissingOut(_) => (app.to_string(), "MISSING_ARGUMENT"),
            AppError::QrNotFound => ("No QR code found in image".to_string(), "NOT_FOUND"),
            AppError::DecodeFailed => ("Failed to decode QR payload".to_string(), "DECODE_FAILED"),
            AppError::UnsupportedImage(_) => (app.to_string(), "UNSUPPORTED_FORMAT"),
            AppError::FileNotFound(_) => (app.to_string(), "NOT_FOUND"),
        }
    } else {
        ("Command failed".to_string(), "INTERNAL_ERROR")
    }
}
