use comfy_table::Table;
use serde::Serialize;

use crate::cli::SearchArgs;
use crate::models::{LlmModel, ModelDatabase};
use crate::output::{AppResult, ListJson, OutputMode, print_json};

#[derive(Debug, Serialize)]
struct SearchItem {
    name: String,
    provider: String,
    parameter_count: String,
    quantization: String,
    context_length: u32,
    use_case: String,
}

pub fn run(args: SearchArgs, output: OutputMode) -> AppResult<()> {
    let db = ModelDatabase::new()?;
    let mut matches = db.search(&args.query);
    matches.sort_by(|a, b| a.name.cmp(&b.name));

    if matches.len() > args.limit {
        matches.truncate(args.limit);
    }

    if output.verbose {
        eprintln!(
            "debug: search query='{}' matches={}",
            args.query,
            matches.len()
        );
    }

    if output.json {
        let items = matches
            .iter()
            .map(|m| to_search_item(m))
            .collect::<Vec<_>>();
        return print_json(&ListJson {
            ok: true,
            count: items.len(),
            items,
        });
    }

    if output.quiet {
        for model in &matches {
            println!("{}", model.name);
        }
        return Ok(());
    }

    if matches.is_empty() {
        println!("No models found for '{}'.", args.query);
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec![
        "Model", "Provider", "Size", "Quant", "Context", "Use Case",
    ]);

    for model in matches {
        table.add_row(vec![
            model.name.clone(),
            model.provider.clone(),
            model.parameter_count.clone(),
            model.quantization.clone(),
            model.context_length.to_string(),
            model.use_case.clone(),
        ]);
    }

    println!("{table}");
    Ok(())
}

fn to_search_item(model: &LlmModel) -> SearchItem {
    SearchItem {
        name: model.name.clone(),
        provider: model.provider.clone(),
        parameter_count: model.parameter_count.clone(),
        quantization: model.quantization.clone(),
        context_length: model.context_length,
        use_case: model.use_case.clone(),
    }
}
