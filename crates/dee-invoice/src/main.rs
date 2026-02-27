use std::fs;
use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use printpdf::{BuiltinFont, Mm, Op, PdfDocument, PdfPage, PdfSaveOptions, Point, Pt, TextItem};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(
    name = "dee-invoice",
    version,
    about = "Generate invoices from JSON or YAML",
    long_about = "dee-invoice - Validate invoice inputs and generate JSON summaries or PDF invoices.",
    after_help = "EXAMPLES:\n  dee-invoice template --format yaml\n  dee-invoice calc invoice.yaml --json\n  dee-invoice generate invoice.yaml --format pdf --output invoice-001.pdf\n  dee-invoice generate invoice.json --format json --json"
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
    /// Calculate totals from an invoice file
    Calc(CalcArgs),
    /// Generate invoice artifact (PDF or JSON)
    Generate(GenerateArgs),
    /// Print starter template
    Template(TemplateArgs),
}

#[derive(Debug, Args)]
struct CalcArgs {
    /// Input invoice file (.json, .yaml, .yml)
    input: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Pdf,
    Json,
}

#[derive(Debug, Args)]
struct GenerateArgs {
    /// Input invoice file (.json, .yaml, .yml)
    input: String,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Pdf)]
    format: OutputFormat,

    /// Output path. Required for PDF, optional for JSON.
    #[arg(long)]
    output: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TemplateFormat {
    Json,
    Yaml,
}

#[derive(Debug, Args)]
struct TemplateArgs {
    #[arg(long, value_enum, default_value_t = TemplateFormat::Yaml)]
    format: TemplateFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Party {
    name: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    phone: Option<String>,
    #[serde(default)]
    address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LineItem {
    description: String,
    quantity: f64,
    unit_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InvoiceInput {
    invoice_number: String,
    issue_date: String,
    due_date: String,
    currency: String,
    seller: Party,
    buyer: Party,
    items: Vec<LineItem>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    tax_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
struct ComputedLineItem {
    description: String,
    quantity: f64,
    unit_price: f64,
    line_total: f64,
}

#[derive(Debug, Clone, Serialize)]
struct InvoiceComputed {
    invoice_number: String,
    issue_date: String,
    due_date: String,
    currency: String,
    seller: Party,
    buyer: Party,
    items: Vec<ComputedLineItem>,
    subtotal: f64,
    tax_rate: f64,
    tax_amount: f64,
    total: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
}

#[derive(Debug, Serialize)]
struct ItemResponse<T> {
    ok: bool,
    item: T,
}

#[derive(Debug, Serialize)]
struct ActionResponse {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    ok: bool,
    error: String,
    code: String,
}

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Input file could not be read")]
    Io,
    #[error("Input parse failed")]
    Parse,
    #[error("PDF generation failed")]
    Pdf,
}

impl AppError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "INVALID_ARGUMENT",
            Self::Io => "IO_ERROR",
            Self::Parse => "PARSE_FAILED",
            Self::Pdf => "PDF_ERROR",
        }
    }
}

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(&cli) {
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
    match &cli.command {
        Commands::Template(args) => cmd_template(args, &cli.global),
        Commands::Calc(args) => {
            let input = load_input(&args.input)?;
            let computed = compute_invoice(input)?;
            if cli.global.json {
                print_json(&ItemResponse {
                    ok: true,
                    item: computed,
                });
            } else if cli.global.quiet {
                println!("1");
            } else {
                println!(
                    "{} total: {:.2} {}",
                    computed.invoice_number, computed.total, computed.currency
                );
            }
            Ok(())
        }
        Commands::Generate(args) => {
            let input = load_input(&args.input)?;
            let computed = compute_invoice(input)?;

            match args.format {
                OutputFormat::Json => {
                    if let Some(path) = &args.output {
                        let out =
                            serde_json::to_string_pretty(&computed).map_err(|_| AppError::Parse)?;
                        fs::write(path, out).map_err(|_| AppError::Io)?;
                        print_action(
                            "Invoice JSON generated",
                            Some(path.to_string()),
                            &cli.global,
                        )
                    } else if cli.global.json {
                        print_json(&ItemResponse {
                            ok: true,
                            item: computed,
                        });
                        Ok(())
                    } else if cli.global.quiet {
                        println!("1");
                        Ok(())
                    } else {
                        let out =
                            serde_json::to_string_pretty(&computed).map_err(|_| AppError::Parse)?;
                        println!("{out}");
                        Ok(())
                    }
                }
                OutputFormat::Pdf => {
                    let output = args
                        .output
                        .clone()
                        .unwrap_or_else(|| format!("{}.pdf", computed.invoice_number));
                    write_pdf(&computed, &output)?;
                    print_action("Invoice PDF generated", Some(output), &cli.global)
                }
            }
        }
    }
}

fn cmd_template(args: &TemplateArgs, global: &GlobalFlags) -> Result<(), AppError> {
    let sample = sample_invoice();
    match args.format {
        TemplateFormat::Json => {
            let out = serde_json::to_string_pretty(&sample).map_err(|_| AppError::Parse)?;
            println!("{out}");
        }
        TemplateFormat::Yaml => {
            let out = serde_yaml::to_string(&sample).map_err(|_| AppError::Parse)?;
            println!("{out}");
        }
    }

    if global.quiet {
        eprintln!("[dee-invoice] template printed");
    }

    Ok(())
}

fn load_input(path: &str) -> Result<InvoiceInput, AppError> {
    let content = fs::read_to_string(path).map_err(|_| AppError::Io)?;
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();

    match ext.as_str() {
        "json" => serde_json::from_str(&content).map_err(|_| AppError::Parse),
        "yaml" | "yml" => serde_yaml::from_str(&content).map_err(|_| AppError::Parse),
        _ => {
            if let Ok(v) = serde_json::from_str::<InvoiceInput>(&content) {
                Ok(v)
            } else {
                serde_yaml::from_str(&content).map_err(|_| AppError::Parse)
            }
        }
    }
}

fn compute_invoice(input: InvoiceInput) -> Result<InvoiceComputed, AppError> {
    validate_input(&input)?;

    let items: Vec<ComputedLineItem> = input
        .items
        .into_iter()
        .map(|it| ComputedLineItem {
            line_total: round2(it.quantity * it.unit_price),
            description: it.description,
            quantity: it.quantity,
            unit_price: it.unit_price,
        })
        .collect();

    let subtotal = round2(items.iter().map(|i| i.line_total).sum());
    let tax_rate = input.tax_rate.unwrap_or(0.0);
    let tax_amount = round2(subtotal * (tax_rate / 100.0));
    let total = round2(subtotal + tax_amount);

    Ok(InvoiceComputed {
        invoice_number: input.invoice_number,
        issue_date: input.issue_date,
        due_date: input.due_date,
        currency: input.currency,
        seller: input.seller,
        buyer: input.buyer,
        items,
        subtotal,
        tax_rate,
        tax_amount,
        total,
        notes: input.notes,
    })
}

fn validate_input(input: &InvoiceInput) -> Result<(), AppError> {
    if input.invoice_number.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "invoice_number must not be empty".to_string(),
        ));
    }
    if input.currency.trim().len() != 3 {
        return Err(AppError::InvalidArgument(
            "currency must be a 3-letter code".to_string(),
        ));
    }
    if input.items.is_empty() {
        return Err(AppError::InvalidArgument(
            "items must contain at least one line item".to_string(),
        ));
    }
    for item in &input.items {
        if item.description.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "line item description must not be empty".to_string(),
            ));
        }
        if item.quantity <= 0.0 || item.unit_price < 0.0 {
            return Err(AppError::InvalidArgument(
                "line items require quantity > 0 and unit_price >= 0".to_string(),
            ));
        }
    }
    Ok(())
}

fn write_pdf(invoice: &InvoiceComputed, path: &str) -> Result<(), AppError> {
    let mut doc = PdfDocument::new(&format!("Invoice {}", invoice.invoice_number));

    let mut ops = vec![
        Op::StartTextSection,
        Op::SetFontSizeBuiltinFont {
            size: Pt(14.0),
            font: BuiltinFont::HelveticaBold,
        },
        Op::SetTextCursor {
            pos: Point {
                x: Pt(28.0),
                y: Pt(800.0),
            },
        },
        Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(format!(
                "Invoice {}",
                invoice.invoice_number
            ))],
            font: BuiltinFont::HelveticaBold,
        },
        Op::SetFontSizeBuiltinFont {
            size: Pt(10.0),
            font: BuiltinFont::Helvetica,
        },
    ];

    let mut y = 780.0_f32;
    for line in render_lines(invoice) {
        ops.push(Op::SetTextCursor {
            pos: Point {
                x: Pt(28.0),
                y: Pt(y),
            },
        });
        ops.push(Op::WriteTextBuiltinFont {
            items: vec![TextItem::Text(line)],
            font: BuiltinFont::Helvetica,
        });
        y -= 14.0;
    }
    ops.push(Op::EndTextSection);

    let page = PdfPage::new(Mm(210.0), Mm(297.0), ops);
    doc.with_pages(vec![page]);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    fs::write(PathBuf::from(path), bytes).map_err(|_| AppError::Pdf)
}

fn render_lines(invoice: &InvoiceComputed) -> Vec<String> {
    let mut lines = vec![
        format!("Issue date: {}", invoice.issue_date),
        format!("Due date: {}", invoice.due_date),
        format!("Currency: {}", invoice.currency),
        format!("Seller: {}", invoice.seller.name),
        format!("Buyer: {}", invoice.buyer.name),
        "Items:".to_string(),
    ];

    for item in &invoice.items {
        lines.push(format!(
            "- {} | qty {:.2} x {:.2} = {:.2}",
            item.description, item.quantity, item.unit_price, item.line_total
        ));
    }

    lines.push(format!("Subtotal: {:.2}", invoice.subtotal));
    lines.push(format!(
        "Tax ({}%): {:.2}",
        invoice.tax_rate, invoice.tax_amount
    ));
    lines.push(format!("Total: {:.2}", invoice.total));

    if let Some(notes) = &invoice.notes {
        lines.push(format!("Notes: {notes}"));
    }

    lines
}

fn sample_invoice() -> InvoiceInput {
    InvoiceInput {
        invoice_number: "INV-001".to_string(),
        issue_date: "2026-02-26".to_string(),
        due_date: "2026-03-12".to_string(),
        currency: "USD".to_string(),
        seller: Party {
            name: "Dee Agency".to_string(),
            email: Some("billing@dee.ink".to_string()),
            phone: None,
            address: Some("123 Market St".to_string()),
        },
        buyer: Party {
            name: "Client Co".to_string(),
            email: Some("ap@client.co".to_string()),
            phone: None,
            address: Some("42 Broad Ave".to_string()),
        },
        items: vec![
            LineItem {
                description: "Design sprint".to_string(),
                quantity: 8.0,
                unit_price: 120.0,
            },
            LineItem {
                description: "Implementation".to_string(),
                quantity: 12.0,
                unit_price: 140.0,
            },
        ],
        notes: Some("Payment due in 14 days".to_string()),
        tax_rate: Some(8.5),
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn print_action(message: &str, path: Option<String>, global: &GlobalFlags) -> Result<(), AppError> {
    if global.json {
        print_json(&ActionResponse {
            ok: true,
            message: message.to_string(),
            path,
        });
    } else if global.quiet {
        println!("{}", path.unwrap_or_else(|| "1".to_string()));
    } else if let Some(path) = path {
        println!("{}: {}", message, path);
    } else {
        println!("{message}");
    }
    Ok(())
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
