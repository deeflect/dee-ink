use comfy_table::Table;

use crate::cli::PlanArgs;
use crate::hardware::SystemSpecs;
use crate::models::ModelDatabase;
use crate::output::{AppError, AppResult, ItemJson, OutputMode, print_json};
use crate::scoring::{PlanRequest, estimate_model_plan};

pub fn run(args: PlanArgs, output: OutputMode) -> AppResult<()> {
    let db = ModelDatabase::new()?;
    let system = SystemSpecs::detect();
    let model = db
        .resolve_model_selector(&args.model)
        .map_err(map_selector_error)?;

    let request = PlanRequest {
        context: args.context,
        quant: args.quant.clone(),
        target_tps: args.target_tps,
    };

    let estimate =
        estimate_model_plan(model, &request, &system).map_err(AppError::InvalidArgument)?;

    if output.verbose {
        eprintln!(
            "debug: plan model='{}' context={} quant='{}' target_tps={}",
            estimate.model_name,
            estimate.context,
            estimate.quantization,
            estimate
                .target_tps
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "none".to_string())
        );
    }

    if output.json {
        let mut value = serde_json::to_value(ItemJson {
            ok: true,
            item: estimate,
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
        strip_nulls(&mut value);
        return print_json(&value);
    }

    if output.quiet {
        println!("model={}", estimate.model_name);
        println!("context={}", estimate.context);
        println!("quantization={}", estimate.quantization);
        println!(
            "minimum=ram:{:.2}GB,vram:{},cpu:{}",
            estimate.minimum.ram_gb,
            estimate
                .minimum
                .vram_gb
                .map(|v| format!("{v:.2}GB"))
                .unwrap_or_else(|| "none".to_string()),
            estimate.minimum.cpu_cores
        );
        println!(
            "recommended=ram:{:.2}GB,vram:{},cpu:{}",
            estimate.recommended.ram_gb,
            estimate
                .recommended
                .vram_gb
                .map(|v| format!("{v:.2}GB"))
                .unwrap_or_else(|| "none".to_string()),
            estimate.recommended.cpu_cores
        );
        return Ok(());
    }

    let mut summary = Table::new();
    summary.set_header(vec!["Field", "Value"]);
    summary.add_row(vec!["Model".to_string(), estimate.model_name.clone()]);
    summary.add_row(vec!["Provider".to_string(), estimate.provider.clone()]);
    summary.add_row(vec!["Context".to_string(), estimate.context.to_string()]);
    summary.add_row(vec![
        "Quantization".to_string(),
        estimate.quantization.clone(),
    ]);
    summary.add_row(vec![
        "Target tok/s".to_string(),
        estimate
            .target_tps
            .map(|t| format!("{t:.1}"))
            .unwrap_or_else(|| "none".to_string()),
    ]);
    summary.add_row(vec![
        "Current Status".to_string(),
        format!(
            "{} ({}, {:.1} tok/s)",
            estimate.current.fit_level.label(),
            estimate.current.run_mode.label(),
            estimate.current.estimated_tps
        ),
    ]);
    println!("{summary}");

    let mut hw = Table::new();
    hw.set_header(vec!["Tier", "RAM (GB)", "VRAM (GB)", "CPU Cores"]);
    hw.add_row(vec![
        "Minimum".to_string(),
        format!("{:.2}", estimate.minimum.ram_gb),
        estimate
            .minimum
            .vram_gb
            .map(|v| format!("{v:.2}"))
            .unwrap_or_else(|| "none".to_string()),
        estimate.minimum.cpu_cores.to_string(),
    ]);
    hw.add_row(vec![
        "Recommended".to_string(),
        format!("{:.2}", estimate.recommended.ram_gb),
        estimate
            .recommended
            .vram_gb
            .map(|v| format!("{v:.2}"))
            .unwrap_or_else(|| "none".to_string()),
        estimate.recommended.cpu_cores.to_string(),
    ]);
    println!();
    println!("{hw}");

    let mut paths = Table::new();
    paths.set_header(vec!["Path", "Feasible", "Fit", "tok/s", "Notes"]);
    for path in &estimate.run_paths {
        let notes = if path.notes.is_empty() {
            "-".to_string()
        } else {
            path.notes.join("; ")
        };
        paths.add_row(vec![
            path.path.label().to_string(),
            if path.feasible {
                "yes".to_string()
            } else {
                "no".to_string()
            },
            path.fit_level
                .map(|f| f.label().to_string())
                .unwrap_or_else(|| "-".to_string()),
            path.estimated_tps
                .map(|t| format!("{t:.1}"))
                .unwrap_or_else(|| "-".to_string()),
            notes,
        ]);
    }
    println!();
    println!("{paths}");

    if !estimate.upgrade_deltas.is_empty() {
        println!();
        println!("Upgrade Deltas:");
        for delta in &estimate.upgrade_deltas {
            println!("- {}", delta.description);
        }
    }

    println!();
    println!("{}", estimate.estimate_notice);
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

fn strip_nulls(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            let keys: Vec<String> = map
                .iter()
                .filter_map(|(k, v)| if v.is_null() { Some(k.clone()) } else { None })
                .collect();
            for key in keys {
                map.remove(&key);
            }
            for child in map.values_mut() {
                strip_nulls(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                strip_nulls(item);
            }
        }
        _ => {}
    }
}
