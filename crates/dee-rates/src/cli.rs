use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "dee-rates",
    version,
    about = "Currency exchange rates and conversions",
    after_help = "EXAMPLES:\n  dee-rates get USD\n  dee-rates get USD EUR --json\n  dee-rates convert 100 USD EUR\n  dee-rates convert 100 USD EUR --json\n  dee-rates list --json"
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
