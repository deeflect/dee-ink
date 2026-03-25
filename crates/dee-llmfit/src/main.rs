mod cli;
mod commands;
mod hardware;
mod models;
mod output;
mod scoring;

use clap::Parser;
use cli::{Cli, Commands};
use output::{OutputMode, print_error};

fn main() {
    let cli = parse_cli();
    let output = OutputMode {
        json: cli.global.json,
        quiet: cli.global.quiet,
        verbose: cli.global.verbose,
    };

    let result = match cli.command {
        Commands::System => commands::system::run(output),
        Commands::Fit(args) => commands::fit::run(args, output),
        Commands::Search(args) => commands::search::run(args, output),
        Commands::Info(args) => commands::info::run(args, output),
        Commands::Recommend(args) => commands::recommend::run(args, output),
        Commands::Plan(args) => commands::plan::run(args, output),
    };

    if let Err(err) = result {
        print_error(&err, output.json);
        std::process::exit(1);
    }
}

fn parse_cli() -> Cli {
    match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => handle_clap_parse_error(err),
    }
}

fn handle_clap_parse_error(err: clap::Error) -> ! {
    use clap::error::ErrorKind;

    match err.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            let _ = err.print();
            std::process::exit(0);
        }
        _ => {
            let wants_json = std::env::args().any(|arg| arg == "--json" || arg == "-j");
            if wants_json {
                let payload = serde_json::json!({
                    "ok": false,
                    "error": err.to_string().trim(),
                    "code": "INVALID_ARGUMENT",
                });
                println!("{payload}");
            } else {
                let _ = err.print();
            }
            std::process::exit(1);
        }
    }
}
