mod cli;
mod commands;
mod models;

use std::process::ExitCode;

use clap::Parser;

use crate::{
    cli::{Cli, Commands},
    models::{ErrorJson, OutputMode},
};

fn main() -> ExitCode {
    let cli = parse_cli();

    let output_mode = OutputMode {
        json: cli.global.json,
        quiet: cli.global.quiet,
        verbose: cli.global.verbose,
    };

    let result = match cli.command {
        Commands::Search(args) => commands::search(&args, &output_mode),
        Commands::Get(args) => commands::get(&args, &output_mode),
        Commands::Summary(args) => commands::summary(&args, &output_mode),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if output_mode.json {
                let body = ErrorJson {
                    ok: false,
                    error: err.to_string(),
                    code: err.code().to_string(),
                };
                match serde_json::to_string(&body) {
                    Ok(text) => println!("{text}"),
                    Err(_) => println!(
                        r#"{{"ok":false,"error":"Internal serialization error","code":"SERIALIZE"}}"#
                    ),
                }
            } else {
                eprintln!("error: {err}");
            }
            ExitCode::from(1)
        }
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
                    "code": "INVALID_ARGUMENT"
                });
                println!("{payload}");
            } else {
                let _ = err.print();
            }
            std::process::exit(2);
        }
    }
}
