use comfy_table::Table;
use serde::Serialize;

use crate::cli::{RecommendArgs, UseCaseArg};
use crate::hardware::SystemSpecs;
use crate::models::{ModelDatabase, UseCase};
use crate::output::{AppResult, ListJson, OutputMode, print_json};
use crate::scoring::{FitLevel, ModelFit, backend_compatible, rank_models_by_fit};

#[derive(Debug, Serialize)]
struct RecommendItem {
    name: String,
    provider: String,
    parameter_count: String,
    use_case: String,
    fit_level: String,
    score: f64,
    estimated_tps: f64,
    runtime: String,
    run_mode: String,
    best_quant: String,
}

pub fn run(args: RecommendArgs, output: OutputMode) -> AppResult<()> {
    let system = SystemSpecs::detect();
    let db = ModelDatabase::new()?;
    let use_case_filter = args.use_case.as_ref().map(to_use_case);

    let mut fits: Vec<ModelFit> = db
        .all()
        .iter()
        .filter(|m| backend_compatible(m, &system))
        .map(|m| ModelFit::analyze(m, &system))
        .filter(|f| f.fit_level != FitLevel::TooTight)
        .filter(|f| match use_case_filter {
            Some(target) => f.use_case == target,
            None => true,
        })
        .collect();

    fits = rank_models_by_fit(fits);
    if fits.len() > args.limit {
        fits.truncate(args.limit);
    }

    if output.verbose {
        eprintln!(
            "debug: recommend returned={} use_case={}",
            fits.len(),
            args.use_case
                .as_ref()
                .map(use_case_arg_label)
                .unwrap_or("none")
        );
    }

    if output.json {
        let items = fits.iter().map(to_recommend_item).collect::<Vec<_>>();
        return print_json(&ListJson {
            ok: true,
            count: items.len(),
            items,
        });
    }

    if output.quiet {
        for fit in &fits {
            println!(
                "{}\t{:.1}\t{:.1}",
                fit.model.name, fit.score, fit.estimated_tps
            );
        }
        return Ok(());
    }

    if fits.is_empty() {
        println!("No recommendations matched the current filters.");
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec![
        "Model", "Provider", "Size", "Use", "Fit", "Score", "tok/s", "Runtime", "Mode", "Quant",
    ]);

    for fit in &fits {
        table.add_row(vec![
            fit.model.name.clone(),
            fit.model.provider.clone(),
            fit.model.parameter_count.clone(),
            fit.use_case.label().to_string(),
            fit.fit_level.label().to_string(),
            format!("{:.1}", fit.score),
            format!("{:.1}", fit.estimated_tps),
            fit.runtime.label().to_string(),
            fit.run_mode.label().to_string(),
            fit.best_quant.clone(),
        ]);
    }

    println!("{table}");
    Ok(())
}

fn to_recommend_item(fit: &ModelFit) -> RecommendItem {
    RecommendItem {
        name: fit.model.name.clone(),
        provider: fit.model.provider.clone(),
        parameter_count: fit.model.parameter_count.clone(),
        use_case: fit.use_case.label().to_string(),
        fit_level: fit.fit_level.label().to_string(),
        score: round1(fit.score),
        estimated_tps: round1(fit.estimated_tps),
        runtime: fit.runtime.label().to_string(),
        run_mode: fit.run_mode.label().to_string(),
        best_quant: fit.best_quant.clone(),
    }
}

fn to_use_case(value: &UseCaseArg) -> UseCase {
    match value {
        UseCaseArg::General => UseCase::General,
        UseCaseArg::Coding => UseCase::Coding,
        UseCaseArg::Reasoning => UseCase::Reasoning,
        UseCaseArg::Chat => UseCase::Chat,
        UseCaseArg::Multimodal => UseCase::Multimodal,
        UseCaseArg::Embedding => UseCase::Embedding,
    }
}

fn use_case_arg_label(value: &UseCaseArg) -> &'static str {
    match value {
        UseCaseArg::General => "general",
        UseCaseArg::Coding => "coding",
        UseCaseArg::Reasoning => "reasoning",
        UseCaseArg::Chat => "chat",
        UseCaseArg::Multimodal => "multimodal",
        UseCaseArg::Embedding => "embedding",
    }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
