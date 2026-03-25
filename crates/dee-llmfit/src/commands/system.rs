use comfy_table::Table;
use serde::Serialize;

use crate::hardware::SystemSpecs;
use crate::output::{AppResult, ItemJson, OutputMode, print_json};

#[derive(Debug, Serialize)]
struct GpuItem {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    vram_gb: Option<f64>,
    backend: String,
    count: u32,
    unified_memory: bool,
}

#[derive(Debug, Serialize)]
struct SystemItem {
    total_ram_gb: f64,
    available_ram_gb: f64,
    cpu_cores: usize,
    cpu_name: String,
    has_gpu: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    gpu_vram_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_gpu_vram_gb: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gpu_name: Option<String>,
    gpu_count: u32,
    unified_memory: bool,
    backend: String,
    gpus: Vec<GpuItem>,
}

pub fn run(output: OutputMode) -> AppResult<()> {
    let system = SystemSpecs::detect();

    if output.verbose {
        eprintln!(
            "debug: detected cpu={} cores={} gpu_count={} backend={}",
            system.cpu_name,
            system.total_cpu_cores,
            system.gpu_count,
            system.backend.label()
        );
    }

    if output.json {
        return print_json(&ItemJson {
            ok: true,
            item: to_system_item(&system),
        });
    }

    if output.quiet {
        println!("cpu_name={}", system.cpu_name);
        println!("cpu_cores={}", system.total_cpu_cores);
        println!("total_ram_gb={:.2}", system.total_ram_gb);
        println!("available_ram_gb={:.2}", system.available_ram_gb);
        println!("backend={}", system.backend.label());
        if let Some(name) = &system.gpu_name {
            println!("gpu_name={name}");
        }
        if let Some(vram) = system.gpu_vram_gb {
            println!("gpu_vram_gb={vram:.2}");
        }
        println!("gpu_count={}", system.gpu_count);
        println!("unified_memory={}", system.unified_memory);
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec!["Field", "Value"]);
    table.add_row(vec!["CPU".to_string(), system.cpu_name.clone()]);
    table.add_row(vec![
        "CPU Cores".to_string(),
        system.total_cpu_cores.to_string(),
    ]);
    table.add_row(vec![
        "Total RAM (GB)".to_string(),
        format!("{:.2}", system.total_ram_gb),
    ]);
    table.add_row(vec![
        "Available RAM (GB)".to_string(),
        format!("{:.2}", system.available_ram_gb),
    ]);
    table.add_row(vec![
        "Backend".to_string(),
        system.backend.label().to_string(),
    ]);
    table.add_row(vec![
        "Unified Memory".to_string(),
        if system.unified_memory {
            "yes".to_string()
        } else {
            "no".to_string()
        },
    ]);
    println!("{table}");

    if system.gpus.is_empty() {
        println!("No GPU detected.");
        return Ok(());
    }

    let mut gpu_table = Table::new();
    gpu_table.set_header(vec!["GPU", "VRAM (GB)", "Backend", "Count", "Unified"]);
    for gpu in &system.gpus {
        gpu_table.add_row(vec![
            gpu.name.clone(),
            gpu.vram_gb
                .map(|v| format!("{v:.2}"))
                .unwrap_or_else(|| "unknown".to_string()),
            gpu.backend.label().to_string(),
            gpu.count.to_string(),
            if gpu.unified_memory {
                "yes".to_string()
            } else {
                "no".to_string()
            },
        ]);
    }

    println!();
    println!("{gpu_table}");
    Ok(())
}

fn to_system_item(system: &SystemSpecs) -> SystemItem {
    SystemItem {
        total_ram_gb: round2(system.total_ram_gb),
        available_ram_gb: round2(system.available_ram_gb),
        cpu_cores: system.total_cpu_cores,
        cpu_name: system.cpu_name.clone(),
        has_gpu: system.has_gpu,
        gpu_vram_gb: system.gpu_vram_gb.map(round2),
        total_gpu_vram_gb: system.total_gpu_vram_gb.map(round2),
        gpu_name: system.gpu_name.clone(),
        gpu_count: system.gpu_count,
        unified_memory: system.unified_memory,
        backend: system.backend.label().to_string(),
        gpus: system
            .gpus
            .iter()
            .map(|g| GpuItem {
                name: g.name.clone(),
                vram_gb: g.vram_gb.map(round2),
                backend: g.backend.label().to_string(),
                count: g.count,
                unified_memory: g.unified_memory,
            })
            .collect(),
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
