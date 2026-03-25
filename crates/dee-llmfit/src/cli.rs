use clap::{Args, Parser, Subcommand, ValueEnum};

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

#[derive(Debug, Clone, ValueEnum)]
pub enum UseCaseArg {
    General,
    Coding,
    Reasoning,
    Chat,
    Multimodal,
    Embedding,
}

#[derive(Debug, Parser)]
#[command(
    name = "dee-llmfit",
    version,
    about = "Detect your hardware and find LLMs that fit well",
    long_about = "dee-llmfit - Hardware-aware local LLM fit and planning CLI.",
    after_help = "EXAMPLES:\n  dee-llmfit system\n  dee-llmfit fit\n  dee-llmfit fit --perfect -n 10\n  dee-llmfit fit --use-case coding --json\n  dee-llmfit search \"qwen 14b\"\n  dee-llmfit info \"Qwen/Qwen2.5-Coder-14B-Instruct\" --json\n  dee-llmfit recommend\n  dee-llmfit recommend --use-case coding --json\n  dee-llmfit plan \"Qwen/Qwen2.5-Coder-14B-Instruct\" --context 8192"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Show detected hardware specs
    System,
    /// Rank models by fit score on this machine
    Fit(FitArgs),
    /// Search models by name/provider
    Search(SearchArgs),
    /// Show detailed model info + local fit analysis
    Info(InfoArgs),
    /// Top recommendations (opinionated)
    Recommend(RecommendArgs),
    /// Estimate hardware needed for a model/context
    Plan(PlanArgs),
}

#[derive(Debug, Clone, Args)]
pub struct FitArgs {
    /// Only show perfectly fitting models
    #[arg(long)]
    pub perfect: bool,

    /// Include non-runnable models (default hides too_tight)
    #[arg(long)]
    pub all: bool,

    /// Filter by use case
    #[arg(long)]
    pub use_case: Option<UseCaseArg>,

    /// Max rows to return
    #[arg(short = 'n', long, default_value_t = 20)]
    pub limit: usize,
}

#[derive(Debug, Clone, Args)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Max rows to return
    #[arg(short = 'n', long, default_value_t = 20)]
    pub limit: usize,
}

#[derive(Debug, Clone, Args)]
pub struct InfoArgs {
    /// Exact or partial model selector
    pub model: String,
}

#[derive(Debug, Clone, Args)]
pub struct RecommendArgs {
    /// Filter by use case
    #[arg(long)]
    pub use_case: Option<UseCaseArg>,

    /// Number of recommendations
    #[arg(short = 'n', long, default_value_t = 5)]
    pub limit: usize,
}

#[derive(Debug, Clone, Args)]
pub struct PlanArgs {
    /// Exact or partial model selector
    pub model: String,

    /// Target context window for estimate
    #[arg(long, default_value_t = 4096)]
    pub context: u32,

    /// Optional forced quantization
    #[arg(long)]
    pub quant: Option<String>,

    /// Optional target tokens/second
    #[arg(long = "target-tps")]
    pub target_tps: Option<f64>,
}
