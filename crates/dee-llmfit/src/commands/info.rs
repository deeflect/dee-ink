use comfy_table::Table;
use serde::Serialize;

use crate::cli::InfoArgs;
use crate::hardware::SystemSpecs;
use crate::models::ModelDatabase;
use crate::output::{AppError, AppResult, ItemJson, OutputMode, print_json};
use crate::scoring::{ModelFit, backend_compatible};

#[derive(Debug, Serialize)]
struct InfoItem {
    name: String,
    provider: String,
    parameter_count: String,
    use_case: String,
    quantization: String,
    context_length: u32,
    fit_level: String,
    run_mode: String,
    runtime: String,
    score: f64,
    estimated_tps: f64,
    best_quant: String,
    memory_required_gb: f64,
    memory_available_gb: f64,
    utilization_pct: f64,
    backend_compatible: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    compatibility_note: String,
    notes: Vec<String>,
}

pub fn run(args: InfoArgs, output: OutputMode) -> AppResult<()> {
    let db = ModelDatabase::new()?;
    let system = SystemSpecs::detect();
    let model = db
        .resolve_model_selector(&args.model)
        .map_err(map_selector_error)?;

    let fit = ModelFit::analyze(model, &system);
    let compatible = backend_compatible(model, &system);
    let compatibility_note = if compatible {
        String::new()
    } else {
        "Model backend is incompatible with this system (for example MLX model on non-Metal)."
            .to_string()
    };

    if output.verbose {
        eprintln!(
            "debug: info model='{}' fit={} runtime={} compatible={}",
            model.name,
            fit.fit_level.label(),
            fit.runtime.label(),
            compatible
        );
    }

    if output.json {
        return print_json(&ItemJson {
            ok: true,
            item: InfoItem {
                name: fit.model.name.clone(),
                provider: fit.model.provider.clone(),
                parameter_count: fit.model.parameter_count.clone(),
                use_case: fit.use_case.label().to_string(),
                quantization: fit.model.quantization.clone(),
                context_length: fit.model.context_length,
                fit_level: fit.fit_level.label().to_string(),
                run_mode: fit.run_mode.label().to_string(),
                runtime: fit.runtime.label().to_string(),
                score: round1(fit.score),
                estimated_tps: round1(fit.estimated_tps),
                best_quant: fit.best_quant.clone(),
                memory_required_gb: round2(fit.memory_required_gb),
                memory_available_gb: round2(fit.memory_available_gb),
                utilization_pct: round1(fit.utilization_pct),
                backend_compatible: compatible,
                compatibility_note,
                notes: fit.notes.clone(),
            },
        });
    }

    if output.quiet {
        println!(
            "{}\t{}\t{}\t{:.1}\t{:.1}",
            fit.model.name,
            fit.fit_level.label(),
            fit.run_mode.label(),
            fit.score,
            fit.estimated_tps
        );
        return Ok(());
    }

    let mut model_table = Table::new();
    model_table.set_header(vec!["Field", "Value"]);
    model_table.add_row(vec!["Model".to_string(), fit.model.name.clone()]);
    model_table.add_row(vec!["Provider".to_string(), fit.model.provider.clone()]);
    model_table.add_row(vec![
        "Parameters".to_string(),
        fit.model.parameter_count.clone(),
    ]);
    model_table.add_row(vec!["Use Case".to_string(), fit.model.use_case.clone()]);
    model_table.add_row(vec![
        "Context Length".to_string(),
        fit.model.context_length.to_string(),
    ]);
    model_table.add_row(vec![
        "Default Quant".to_string(),
        fit.model.quantization.clone(),
    ]);
    if let Some(release_date) = &fit.model.release_date {
        model_table.add_row(vec!["Release Date".to_string(), release_date.clone()]);
    }

    let mut fit_table = Table::new();
    fit_table.set_header(vec!["Metric", "Value"]);
    fit_table.add_row(vec![
        "Fit Level".to_string(),
        fit.fit_level.label().to_string(),
    ]);
    fit_table.add_row(vec![
        "Run Mode".to_string(),
        fit.run_mode.label().to_string(),
    ]);
    fit_table.add_row(vec!["Runtime".to_string(), fit.runtime.label().to_string()]);
    fit_table.add_row(vec!["Score".to_string(), format!("{:.1}", fit.score)]);
    fit_table.add_row(vec![
        "Estimated tok/s".to_string(),
        format!("{:.1}", fit.estimated_tps),
    ]);
    fit_table.add_row(vec!["Best Quant".to_string(), fit.best_quant.clone()]);
    fit_table.add_row(vec![
        "Memory Usage".to_string(),
        format!(
            "{:.2}/{:.2} GB ({:.1}%)",
            fit.memory_required_gb, fit.memory_available_gb, fit.utilization_pct
        ),
    ]);

    println!("{model_table}");
    println!();
    println!("{fit_table}");

    if !compatible {
        println!();
        println!("{compatibility_note}");
    }

    if !fit.notes.is_empty() {
        println!();
        println!("Notes:");
        for note in &fit.notes {
            println!("- {note}");
        }
    }

    Ok(())
}

fn map_selector_error(message: String) -> AppError {
    let lower = message.to_ascii_lowercase();
    if lower.contains("ambiguous") {
        AppError::Ambiguous(message)
    } else if lower.contains("empty") {
        AppError::InvalidArgument(message)
    } else {
        AppError::NotFound(message)
    }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
