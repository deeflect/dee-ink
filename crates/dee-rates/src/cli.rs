use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "dee-rates",
    version,
    about = "Currency exchange rates and conversions",
    long_about = "dee-rates - Get live currency exchange rates and convert amounts\n\nUSAGE:\n  dee-rates <command> [options]\n\nCOMMANDS:\n  get        Get rates for a base currency\n  convert    Convert amount between currencies\n  list       List all available currency codes\n\nOPTIONS:\n  -j, --json       Output as JSON\n  -q, --quiet      Suppress decorative output\n  -v, --verbose    Debug output to stderr\n  -h, --help       Show this help\n  -V, --version    Show version\n\nEXAMPLES:\n  dee-rates get USD\n  dee-rates get USD EUR --json\n  dee-rates convert 100 USD EUR\n  dee-rates convert 100 USD EUR --json\n  dee-rates list --json"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[command(flatten)]
    pub global: GlobalFlags,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalFlags {
    #[arg(short, long, global = true)]
    pub json: bool,

    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Get rates for a base currency, optionally only one target currency
    Get {
        /// Base currency code, e.g. USD
        from: String,
        /// Optional target currency code, e.g. EUR
        to: Option<String>,
    },
    /// Convert amount between currencies
    Convert {
        /// Amount to convert
        amount: f64,
        /// Source currency code
        from: String,
        /// Target currency code
        to: String,
    },
    /// List all available currency codes
    List,
}
