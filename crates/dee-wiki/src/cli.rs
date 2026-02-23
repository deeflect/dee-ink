use clap::{Args, Parser, Subcommand};

#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    /// Output as JSON
    #[arg(short = 'j', long, global = true)]
    pub json: bool,

    /// Suppress decorative output
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Debug output to stderr
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,
}

#[derive(Debug, Parser)]
#[command(
    name = "dee-wiki",
    version,
    about = "Wikipedia lookup CLI",
    long_about = "dee-wiki - Search Wikipedia and fetch article summaries.",
    after_help = "EXAMPLES:\n  dee-wiki search \"rust programming\" --limit 5\n  dee-wiki search \"tokio\" --lang en --json\n  dee-wiki get \"Rust (programming language)\" --lang en --json\n  dee-wiki summary \"Berlin\" --lang de\n  dee-wiki summary \"Taylor Swift\" -j"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Search Wikipedia articles
    Search(SearchArgs),
    /// Get full summary-style page payload
    Get(GetArgs),
    /// Get concise summary payload
    Summary(GetArgs),
}

#[derive(Debug, Clone, Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Maximum number of search results
    #[arg(long, default_value_t = 5)]
    pub limit: usize,

    /// Wikipedia language code
    #[arg(long, default_value = "en")]
    pub lang: String,
}

#[derive(Debug, Clone, Args)]
pub struct GetArgs {
    /// Exact page title
    pub title: String,

    /// Wikipedia language code
    #[arg(long, default_value = "en")]
    pub lang: String,
}
