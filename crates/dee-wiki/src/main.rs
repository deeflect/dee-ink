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
    let cli = Cli::parse();

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
