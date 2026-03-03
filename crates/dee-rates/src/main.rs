mod cli;
mod commands;
mod display;
mod models;

use clap::Parser;
use cli::{Cli, Commands};
use models::{ErrorResponse, ListResponse, SingleResponse};

fn main() {
    let cli = parse_cli();
    let json = cli.global.json;

    let result = match cli.command {
        Commands::Get { from, to } => commands::get_rates(&from, to.as_deref(), cli.global.verbose)
            .map(|item| {
                if json {
                    print_json(&SingleResponse { ok: true, item });
                } else {
                    display::print_get(&item, cli.global.quiet);
                }
            }),
        Commands::Convert { amount, from, to } => {
            commands::convert(amount, &from, &to, cli.global.verbose).map(|item| {
                if json {
                    print_json(&SingleResponse { ok: true, item });
                } else {
                    display::print_convert(&item, cli.global.quiet);
                }
            })
        }
        Commands::List => commands::list_currencies(cli.global.verbose).map(|items| {
            if json {
                print_json(&ListResponse {
                    ok: true,
                    count: items.len(),
                    items,
                });
            } else {
                display::print_list(&items, cli.global.quiet);
            }
        }),
    };

    if let Err(err) = result {
        if json {
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

fn print_json<T: serde::Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(out) => println!("{out}"),
        Err(err) => {
            eprintln!("error: failed to serialize JSON output: {err}");
            std::process::exit(1);
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
