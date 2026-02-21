mod cli;
mod commands;
mod display;
mod models;

use clap::Parser;
use cli::{Cli, Commands};
use models::{ErrorResponse, ListResponse, SingleResponse};

fn main() {
    let cli = Cli::parse();
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
