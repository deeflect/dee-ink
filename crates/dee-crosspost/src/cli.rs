use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "dee-crosspost",
    version,
    about = "Cross-post and schedule posts across social platforms",
    long_about = "dee-crosspost - publish now or schedule content across X, LinkedIn, Bluesky, Threads, and Reddit.",
    after_help = "EXAMPLES:\n  dee-crosspost auth set-token --platform bluesky --token \"$TOKEN\" --json\n  dee-crosspost post --to bluesky,reddit --text \"Launching today\" --title \"Launch\" --subreddit startups --json\n  dee-crosspost schedule --at 2026-02-27T15:00:00Z --to x,linkedin --text \"Weekly update\" --json\n  dee-crosspost run --once --json\n  dee-crosspost queue list --status pending --json"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalFlags,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Args)]
pub struct GlobalFlags {
    #[arg(short = 'j', long, global = true)]
    pub json: bool,

    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Post(PostArgs),
    Schedule(ScheduleArgs),
    Queue(QueueArgs),
    Run(RunArgs),
    Auth(AuthArgs),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ValueEnum)]
pub enum Platform {
    X,
    Linkedin,
    Bluesky,
    Threads,
    Reddit,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Linkedin => "linkedin",
            Self::Bluesky => "bluesky",
            Self::Threads => "threads",
            Self::Reddit => "reddit",
        }
    }
}

#[derive(Debug, Args)]
pub struct PostArgs {
    #[arg(long, value_delimiter = ',')]
    pub to: Vec<Platform>,

    #[arg(long)]
    pub text: String,

    #[arg(long)]
    pub media: Option<String>,

    #[arg(long)]
    pub title: Option<String>,

    #[arg(long)]
    pub subreddit: Option<String>,
}

#[derive(Debug, Args)]
pub struct ScheduleArgs {
    #[arg(long)]
    pub at: String,

    #[arg(long, value_delimiter = ',')]
    pub to: Vec<Platform>,

    #[arg(long)]
    pub text: String,

    #[arg(long)]
    pub media: Option<String>,

    #[arg(long)]
    pub title: Option<String>,

    #[arg(long)]
    pub subreddit: Option<String>,
}

#[derive(Debug, Args)]
pub struct QueueArgs {
    #[command(subcommand)]
    pub command: QueueCommand,
}

#[derive(Debug, Subcommand)]
pub enum QueueCommand {
    List(QueueListArgs),
    Show(QueueShowArgs),
    Cancel(QueueCancelArgs),
}

#[derive(Debug, Args)]
pub struct QueueListArgs {
    #[arg(long)]
    pub status: Option<JobStatus>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
    Canceled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }
}

#[derive(Debug, Args)]
pub struct QueueShowArgs {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct QueueCancelArgs {
    pub id: String,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    #[arg(long)]
    pub once: bool,

    #[arg(long)]
    pub daemon: bool,

    #[arg(long, default_value_t = 30)]
    pub interval: u64,
}

#[derive(Debug, Args)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommand,
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Login(AuthLoginArgs),
    SetToken(AuthSetTokenArgs),
    Status,
    Logout(AuthLogoutArgs),
}

#[derive(Debug, Args)]
pub struct AuthLoginArgs {
    #[arg(long)]
    pub platform: Platform,
}

#[derive(Debug, Args)]
pub struct AuthSetTokenArgs {
    #[arg(long)]
    pub platform: Platform,

    #[arg(long)]
    pub token: String,
}

#[derive(Debug, Args)]
pub struct AuthLogoutArgs {
    #[arg(long)]
    pub platform: Platform,
}
