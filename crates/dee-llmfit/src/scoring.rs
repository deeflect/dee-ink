use crate::hardware::{GpuBackend, SystemSpecs, gpu_memory_bandwidth_gbps};
use crate::models::{self, LlmModel, UseCase};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum InferenceRuntime {
    LlamaCpp,
    Mlx,
}

impl InferenceRuntime {
    pub fn label(&self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama.cpp",
            Self::Mlx => "MLX",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FitLevel {
    Perfect,
    Good,
    Marginal,
    TooTight,
}

impl FitLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Perfect => "perfect",
            Self::Good => "good",
            Self::Marginal => "marginal",
            Self::TooTight => "too_tight",
        }
    }

    pub fn rank(&self) -> u8 {
        match self {
            Self::Perfect => 4,
            Self::Good => 3,
            Self::Marginal => 2,
            Self::TooTight => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunMode {
    Gpu,
    MoeOffload,
    CpuOffload,
    CpuOnly,
}

impl RunMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Gpu => "gpu",
            Self::MoeOffload => "moe_offload",
            Self::CpuOffload => "cpu_offload",
            Self::CpuOnly => "cpu_only",
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct ScoreComponents {
    pub quality: f64,
    pub speed: f64,
    pub fit: f64,
    pub context: f64,
}

#[derive(Clone, serde::Serialize)]
pub struct ModelFit {
    pub model: LlmModel,
    pub fit_level: FitLevel,
    pub run_mode: RunMode,
    pub memory_required_gb: f64,
    pub memory_available_gb: f64,
    pub utilization_pct: f64,
    pub notes: Vec<String>,
    pub moe_offloaded_gb: Option<f64>,
    pub score: f64,
    pub score_components: ScoreComponents,
    pub estimated_tps: f64,
    pub best_quant: String,
    pub use_case: UseCase,
    pub runtime: InferenceRuntime,
}

impl ModelFit {
    pub fn analyze(model: &LlmModel, system: &SystemSpecs) -> Self {
        Self::analyze_with_context_limit(model, system, None)
    }

    pub fn analyze_with_context_limit(
        model: &LlmModel,
        system: &SystemSpecs,
        context_limit: Option<u32>,
    ) -> Self {
        let mut notes = Vec::new();
        let estimation_ctx = context_limit
            .map(|limit| limit.min(model.context_length))
            .unwrap_or(model.context_length);

        if estimation_ctx < model.context_length {
            notes.push(format!(
                "context capped for estimation: {} -> {}",
                model.context_length, estimation_ctx
            ));
        }

        let use_case = UseCase::from_model(model);
        let runtime = if system.backend == GpuBackend::Metal && system.unified_memory {
            InferenceRuntime::Mlx
        } else {
            InferenceRuntime::LlamaCpp
        };

        let min_vram = model.min_vram_gb.unwrap_or(model.min_ram_gb);
        let default_mem_required =
            model.estimate_memory_gb(model.quantization.as_str(), estimation_ctx);
        let choose_quant =
            |budget: f64| best_quant_for_runtime_budget(model, runtime, budget, estimation_ctx);

        let (run_mode, mem_required, mem_available) = if system.has_gpu {
            if system.unified_memory {
                if let Some(pool) = system.gpu_vram_gb {
                    notes.push("unified memory: cpu and gpu share pool".to_string());
                    if model.is_moe {
                        (RunMode::Gpu, min_vram, pool)
                    } else if let Some((_, best_mem)) = choose_quant(pool) {
                        (RunMode::Gpu, best_mem, pool)
                    } else {
                        (RunMode::Gpu, default_mem_required, pool)
                    }
                } else {
                    cpu_path(model, system, runtime, estimation_ctx, &mut notes)
                }
            } else if let Some(total_vram) = system.total_gpu_vram_gb {
                if model.is_moe && min_vram <= total_vram {
                    notes.push("gpu: model fits in vram".to_string());
                    (RunMode::Gpu, min_vram, total_vram)
                } else if model.is_moe {
                    moe_offload_path(model, system, total_vram, min_vram, runtime, &mut notes)
                } else if let Some((_, best_mem)) = choose_quant(total_vram) {
                    notes.push("gpu: model fits in vram".to_string());
                    (RunMode::Gpu, best_mem, total_vram)
                } else if let Some((_, best_mem)) = choose_quant(system.available_ram_gb) {
                    notes.push("gpu offload: vram insufficient, spilling to ram".to_string());
                    (RunMode::CpuOffload, best_mem, system.available_ram_gb)
                } else {
                    notes.push("insufficient vram and ram".to_string());
                    (RunMode::Gpu, default_mem_required, total_vram)
                }
            } else {
                notes.push("gpu detected but vram unknown".to_string());
                cpu_path(model, system, runtime, estimation_ctx, &mut notes)
            }
        } else {
            cpu_path(model, system, runtime, estimation_ctx, &mut notes)
        };

        let fit_level = score_fit(
            mem_required,
            mem_available,
            model.recommended_ram_gb,
            run_mode,
        );
        let utilization_pct = if mem_available > 0.0 {
            (mem_required / mem_available) * 100.0
        } else {
            f64::INFINITY
        };

        if run_mode == RunMode::CpuOnly {
            notes.push("no gpu acceleration detected".to_string());
        }

        let hierarchy: &[&str] = if runtime == InferenceRuntime::Mlx {
            models::MLX_QUANT_HIERARCHY
        } else {
            models::QUANT_HIERARCHY
        };

        let (best_quant, _) = model
            .best_quant_for_budget_with(mem_available, estimation_ctx, hierarchy)
            .or_else(|| {
                if runtime == InferenceRuntime::Mlx {
                    model.best_quant_for_budget(mem_available, estimation_ctx)
                } else {
                    None
                }
            })
            .unwrap_or((model.quantization.as_str(), mem_required));
        let best_quant = best_quant.to_string();

        let estimated_tps = estimate_tps(model, &best_quant, system, run_mode, runtime);
        let components = compute_scores(
            model,
            &best_quant,
            use_case,
            estimated_tps,
            mem_required,
            mem_available,
        );
        let score = weighted_score(components, use_case);

        if estimated_tps > 0.0 {
            notes.push(format!("estimated speed: {:.1} tok/s", estimated_tps));
        }

        let moe_offloaded_gb = if run_mode == RunMode::MoeOffload {
            model.moe_offloaded_ram_gb()
        } else {
            None
        };

        Self {
            model: model.clone(),
            fit_level,
            run_mode,
            memory_required_gb: mem_required,
            memory_available_gb: mem_available,
            utilization_pct,
            notes,
            moe_offloaded_gb,
            score,
            score_components: components,
            estimated_tps,
            best_quant,
            use_case,
            runtime,
        }
    }
}

fn score_fit(
    mem_required: f64,
    mem_available: f64,
    recommended: f64,
    run_mode: RunMode,
) -> FitLevel {
    if mem_required > mem_available {
        return FitLevel::TooTight;
    }

    match run_mode {
        RunMode::Gpu => {
            if recommended <= mem_available {
                FitLevel::Perfect
            } else if mem_available >= mem_required * 1.2 {
                FitLevel::Good
            } else {
                FitLevel::Marginal
            }
        }
        RunMode::MoeOffload | RunMode::CpuOffload => {
            if mem_available >= mem_required * 1.2 {
                FitLevel::Good
            } else {
                FitLevel::Marginal
            }
        }
        RunMode::CpuOnly => FitLevel::Marginal,
    }
}

fn cpu_path(
    model: &LlmModel,
    system: &SystemSpecs,
    runtime: InferenceRuntime,
    estimation_ctx: u32,
    notes: &mut Vec<String>,
) -> (RunMode, f64, f64) {
    notes.push("cpu-only path".to_string());
    if model.is_moe {
        return (RunMode::CpuOnly, model.min_ram_gb, system.available_ram_gb);
    }

    if let Some((_, best_mem)) =
        best_quant_for_runtime_budget(model, runtime, system.available_ram_gb, estimation_ctx)
    {
        (RunMode::CpuOnly, best_mem, system.available_ram_gb)
    } else {
        (
            RunMode::CpuOnly,
            model.estimate_memory_gb(model.quantization.as_str(), estimation_ctx),
            system.available_ram_gb,
        )
    }
}

fn moe_offload_path(
    model: &LlmModel,
    system: &SystemSpecs,
    system_vram: f64,
    total_vram: f64,
    runtime: InferenceRuntime,
    notes: &mut Vec<String>,
) -> (RunMode, f64, f64) {
    let hierarchy: &[&str] = if runtime == InferenceRuntime::Mlx {
        models::MLX_QUANT_HIERARCHY
    } else {
        models::QUANT_HIERARCHY
    };

    for &quant in hierarchy {
        if let Some((moe_vram, offloaded_gb)) = moe_memory_for_quant(model, quant)
            && moe_vram <= system_vram
            && offloaded_gb <= system.available_ram_gb
        {
            notes.push(format!(
                "moe offload: {:.1}GB active in vram, {:.1}GB offloaded",
                moe_vram, offloaded_gb
            ));
            return (RunMode::MoeOffload, moe_vram, system_vram);
        }
    }

    if model.min_ram_gb <= system.available_ram_gb {
        notes.push("moe offload unavailable, falling back to cpu offload".to_string());
        (
            RunMode::CpuOffload,
            model.min_ram_gb,
            system.available_ram_gb,
        )
    } else {
        notes.push("insufficient vram and ram".to_string());
        (RunMode::Gpu, total_vram, system_vram)
    }
}

fn moe_memory_for_quant(model: &LlmModel, quant: &str) -> Option<(f64, f64)> {
    if !model.is_moe {
        return None;
    }

    let active_params = model.active_parameters? as f64;
    let total_params = model.parameters_raw? as f64;
    let bpp = models::quant_bpp(quant);

    let active_vram = ((active_params * bpp) / (1024.0 * 1024.0 * 1024.0) * 1.1).max(0.5);
    let inactive_params = (total_params - active_params).max(0.0);
    let offloaded_ram = (inactive_params * bpp) / (1024.0 * 1024.0 * 1024.0);

    Some((active_vram, offloaded_ram))
}

fn best_quant_for_runtime_budget(
    model: &LlmModel,
    runtime: InferenceRuntime,
    budget: f64,
    estimation_ctx: u32,
) -> Option<(&'static str, f64)> {
    let hierarchy: &[&str] = if runtime == InferenceRuntime::Mlx {
        models::MLX_QUANT_HIERARCHY
    } else {
        models::QUANT_HIERARCHY
    };

    model
        .best_quant_for_budget_with(budget, estimation_ctx, hierarchy)
        .or_else(|| {
            if runtime == InferenceRuntime::Mlx {
                model.best_quant_for_budget(budget, estimation_ctx)
            } else {
                None
            }
        })
}

pub fn backend_compatible(model: &LlmModel, system: &SystemSpecs) -> bool {
    if model.is_mlx_model() {
        system.backend == GpuBackend::Metal && system.unified_memory
    } else {
        true
    }
}

pub fn rank_models_by_fit(mut fits: Vec<ModelFit>) -> Vec<ModelFit> {
    fits.sort_by(|a, b| {
        let ar = a.fit_level != FitLevel::TooTight;
        let br = b.fit_level != FitLevel::TooTight;

        match (ar, br) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.estimated_tps
                    .partial_cmp(&a.estimated_tps)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    fits
}

pub fn estimate_tps(
    model: &LlmModel,
    quant: &str,
    system: &SystemSpecs,
    run_mode: RunMode,
    runtime: InferenceRuntime,
) -> f64 {
    let params = model
        .active_parameters
        .filter(|_| model.is_moe)
        .map(|p| (p as f64) / 1_000_000_000.0)
        .unwrap_or_else(|| model.params_b())
        .max(0.1);

    if run_mode != RunMode::CpuOnly {
        let gpu_name = system.gpu_name.as_deref().unwrap_or("");
        if let Some(bw) = gpu_memory_bandwidth_gbps(gpu_name) {
            let model_gb = params * models::quant_bytes_per_param(quant);
            let raw_tps = (bw / model_gb) * 0.55;

            let mode_factor = match run_mode {
                RunMode::Gpu => 1.0,
                RunMode::MoeOffload => 0.8,
                RunMode::CpuOffload => 0.5,
                RunMode::CpuOnly => 1.0,
            };

            return (raw_tps * mode_factor).max(0.1);
        }
    }

    let k: f64 = match (system.backend, runtime) {
        (GpuBackend::Metal, InferenceRuntime::Mlx) => 250.0,
        (GpuBackend::Metal, InferenceRuntime::LlamaCpp) => 160.0,
        (GpuBackend::Cuda, _) => 220.0,
        (GpuBackend::Rocm, _) => 180.0,
        (GpuBackend::Vulkan, _) => 150.0,
        (GpuBackend::Sycl, _) => 100.0,
        (GpuBackend::CpuArm, _) => 90.0,
        (GpuBackend::CpuX86, _) => 70.0,
        (GpuBackend::Ascend, _) => 390.0,
    };

    let mut base = (k / params) * models::quant_speed_multiplier(quant);
    if system.total_cpu_cores >= 8 {
        base *= 1.1;
    }

    match run_mode {
        RunMode::Gpu => {}
        RunMode::MoeOffload => base *= 0.8,
        RunMode::CpuOffload => base *= 0.5,
        RunMode::CpuOnly => {
            let cpu_k = if cfg!(target_arch = "aarch64") {
                90.0
            } else {
                70.0
            };
            base = (cpu_k / params) * models::quant_speed_multiplier(quant);
            if system.total_cpu_cores >= 8 {
                base *= 1.1;
            }
        }
    }

    base.max(0.1)
}

fn compute_scores(
    model: &LlmModel,
    quant: &str,
    use_case: UseCase,
    estimated_tps: f64,
    mem_required: f64,
    mem_available: f64,
) -> ScoreComponents {
    ScoreComponents {
        quality: quality_score(model, quant, use_case),
        speed: speed_score(estimated_tps, use_case),
        fit: fit_score(mem_required, mem_available),
        context: context_score(model, use_case),
    }
}

fn quality_score(model: &LlmModel, quant: &str, use_case: UseCase) -> f64 {
    let params = model.params_b();
    let base = if params < 1.0 {
        30.0
    } else if params < 3.0 {
        45.0
    } else if params < 7.0 {
        60.0
    } else if params < 10.0 {
        75.0
    } else if params < 20.0 {
        82.0
    } else if params < 40.0 {
        89.0
    } else {
        95.0
    };

    let name_lower = model.name.to_lowercase();
    let family_bump = if name_lower.contains("qwen") {
        2.0
    } else if name_lower.contains("deepseek") {
        3.0
    } else if name_lower.contains("llama") {
        2.0
    } else if name_lower.contains("mistral") || name_lower.contains("mixtral") {
        1.0
    } else if name_lower.contains("gemma") {
        1.0
    } else if name_lower.contains("starcoder") {
        1.0
    } else {
        0.0
    };

    let task_bump = match use_case {
        UseCase::Coding => {
            if name_lower.contains("code")
                || name_lower.contains("starcoder")
                || name_lower.contains("wizard")
            {
                6.0
            } else {
                0.0
            }
        }
        UseCase::Reasoning => {
            if params >= 13.0 {
                5.0
            } else {
                0.0
            }
        }
        UseCase::Multimodal => {
            if name_lower.contains("vision") || model.use_case.to_lowercase().contains("vision") {
                6.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    };

    (base + family_bump + models::quant_quality_penalty(quant) + task_bump).clamp(0.0, 100.0)
}

fn speed_score(tps: f64, use_case: UseCase) -> f64 {
    let target = match use_case {
        UseCase::General | UseCase::Coding | UseCase::Multimodal | UseCase::Chat => 40.0,
        UseCase::Reasoning => 25.0,
        UseCase::Embedding => 200.0,
    };
    ((tps / target) * 100.0).clamp(0.0, 100.0)
}

fn fit_score(required: f64, available: f64) -> f64 {
    if available <= 0.0 || required > available {
        return 0.0;
    }

    let ratio = required / available;
    if ratio <= 0.5 {
        60.0 + (ratio / 0.5) * 40.0
    } else if ratio <= 0.8 {
        100.0
    } else if ratio <= 0.9 {
        70.0
    } else {
        50.0
    }
}

fn context_score(model: &LlmModel, use_case: UseCase) -> f64 {
    let target: u32 = match use_case {
        UseCase::General | UseCase::Chat => 4096,
        UseCase::Coding | UseCase::Reasoning => 8192,
        UseCase::Multimodal => 4096,
        UseCase::Embedding => 512,
    };

    if model.context_length >= target {
        100.0
    } else if model.context_length >= target / 2 {
        70.0
    } else {
        30.0
    }
}

fn weighted_score(sc: ScoreComponents, use_case: UseCase) -> f64 {
    let (wq, ws, wf, wc) = match use_case {
        UseCase::General => (0.45, 0.30, 0.15, 0.10),
        UseCase::Coding => (0.50, 0.20, 0.15, 0.15),
        UseCase::Reasoning => (0.55, 0.15, 0.15, 0.15),
        UseCase::Chat => (0.40, 0.35, 0.15, 0.10),
        UseCase::Multimodal => (0.50, 0.20, 0.15, 0.15),
        UseCase::Embedding => (0.30, 0.40, 0.20, 0.10),
    };

    let raw = sc.quality * wq + sc.speed * ws + sc.fit * wf + sc.context * wc;
    (raw * 10.0).round() / 10.0
}

const SUPPORTED_QUANTS: &[&str] = &[
    "F32", "F16", "BF16", "Q8_0", "Q6_K", "Q5_K_M", "Q4_K_M", "Q4_0", "Q3_K_M", "Q2_K", "mlx-8bit",
    "mlx-4bit",
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanRequest {
    pub context: u32,
    pub quant: Option<String>,
    pub target_tps: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HardwareEstimate {
    pub vram_gb: Option<f64>,
    pub ram_gb: f64,
    pub cpu_cores: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanRunPath {
    Gpu,
    CpuOffload,
    CpuOnly,
}

impl PlanRunPath {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Gpu => "gpu",
            Self::CpuOffload => "cpu_offload",
            Self::CpuOnly => "cpu_only",
        }
    }

    fn run_mode(self) -> RunMode {
        match self {
            Self::Gpu => RunMode::Gpu,
            Self::CpuOffload => RunMode::CpuOffload,
            Self::CpuOnly => RunMode::CpuOnly,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PathEstimate {
    pub path: PlanRunPath,
    pub feasible: bool,
    pub minimum: Option<HardwareEstimate>,
    pub recommended: Option<HardwareEstimate>,
    pub estimated_tps: Option<f64>,
    pub fit_level: Option<FitLevel>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UpgradeDelta {
    pub resource: String,
    pub add_gb: Option<f64>,
    pub add_cores: Option<usize>,
    pub target_fit: Option<FitLevel>,
    pub path: PlanRunPath,
    pub description: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanCurrentStatus {
    pub fit_level: FitLevel,
    pub run_mode: RunMode,
    pub estimated_tps: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanEstimate {
    pub estimate_notice: String,
    pub model_name: String,
    pub provider: String,
    pub context: u32,
    pub quantization: String,
    pub target_tps: Option<f64>,
    pub minimum: HardwareEstimate,
    pub recommended: HardwareEstimate,
    pub run_paths: Vec<PathEstimate>,
    pub current: PlanCurrentStatus,
    pub upgrade_deltas: Vec<UpgradeDelta>,
}

pub fn normalize_quant(quant: &str) -> Option<String> {
    let trimmed = quant.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.eq_ignore_ascii_case("mlx-4bit") {
        return Some("mlx-4bit".to_string());
    }
    if trimmed.eq_ignore_ascii_case("mlx-8bit") {
        return Some("mlx-8bit".to_string());
    }

    let upper = trimmed.to_uppercase();
    if SUPPORTED_QUANTS.contains(&upper.as_str()) {
        Some(upper)
    } else {
        None
    }
}

fn plan_estimate_tps(
    model: &LlmModel,
    quant: &str,
    backend: GpuBackend,
    path: PlanRunPath,
    cpu_cores: usize,
    gpu_name: Option<&str>,
) -> f64 {
    let params = model.params_b().max(0.1);

    if path != PlanRunPath::CpuOnly
        && let Some(name) = gpu_name
        && let Some(bw) = gpu_memory_bandwidth_gbps(name)
    {
        let model_gb = params * models::quant_bytes_per_param(quant);
        let raw_tps = (bw / model_gb) * 0.55;
        let mode_factor = match path {
            PlanRunPath::Gpu => 1.0,
            PlanRunPath::CpuOffload => 0.5,
            PlanRunPath::CpuOnly => 1.0,
        };
        return (raw_tps * mode_factor).max(0.1);
    }

    let k: f64 = match backend {
        GpuBackend::Metal => 160.0,
        GpuBackend::Cuda => 220.0,
        GpuBackend::Rocm => 180.0,
        GpuBackend::Vulkan => 150.0,
        GpuBackend::Sycl => 100.0,
        GpuBackend::CpuArm => 90.0,
        GpuBackend::CpuX86 => 70.0,
        GpuBackend::Ascend => 390.0,
    };

    let mut base = (k / params) * models::quant_speed_multiplier(quant);
    if cpu_cores >= 8 {
        base *= 1.1;
    }

    match path {
        PlanRunPath::Gpu => {}
        PlanRunPath::CpuOffload => base *= 0.5,
        PlanRunPath::CpuOnly => {
            let cpu_k = if cfg!(target_arch = "aarch64") {
                90.0
            } else {
                70.0
            };
            base = (cpu_k / params) * models::quant_speed_multiplier(quant);
            if cpu_cores >= 8 {
                base *= 1.1;
            }
        }
    }

    base.max(0.1)
}

fn fit_level_for(
    path: PlanRunPath,
    required_gb: f64,
    available_gb: f64,
    recommended_gb: f64,
) -> FitLevel {
    if required_gb > available_gb {
        return FitLevel::TooTight;
    }

    match path {
        PlanRunPath::Gpu => {
            if recommended_gb <= available_gb {
                FitLevel::Perfect
            } else if available_gb >= required_gb * 1.2 {
                FitLevel::Good
            } else {
                FitLevel::Marginal
            }
        }
        PlanRunPath::CpuOffload => {
            if available_gb >= required_gb * 1.2 {
                FitLevel::Good
            } else {
                FitLevel::Marginal
            }
        }
        PlanRunPath::CpuOnly => FitLevel::Marginal,
    }
}

fn minimum_cores_for_target(
    model: &LlmModel,
    quant: &str,
    backend: GpuBackend,
    path: PlanRunPath,
    target_tps: Option<f64>,
    gpu_name: Option<&str>,
) -> Option<usize> {
    if target_tps.is_none() {
        return Some(4);
    }

    let target = target_tps.unwrap_or(0.0);
    for cores in 1..=64 {
        let tps = plan_estimate_tps(model, quant, backend, path, cores, gpu_name);
        if tps >= target {
            return Some(cores);
        }
    }
    None
}

fn default_gpu_backend(system: &SystemSpecs) -> GpuBackend {
    if system.has_gpu {
        system.backend
    } else {
        GpuBackend::Cuda
    }
}

fn evaluate_current(
    model: &LlmModel,
    quant: &str,
    context: u32,
    target_tps: Option<f64>,
    system: &SystemSpecs,
) -> PlanCurrentStatus {
    let model_mem = model.estimate_memory_gb(quant, context);
    let gpu_vram = system
        .total_gpu_vram_gb
        .or(system.gpu_vram_gb)
        .unwrap_or(0.0);
    let gpu_name = system.gpu_name.as_deref();

    let mut candidates: Vec<(FitLevel, PlanRunPath, f64)> = Vec::new();

    if system.has_gpu && gpu_vram > 0.0 {
        let gpu_fit = fit_level_for(
            PlanRunPath::Gpu,
            model_mem,
            gpu_vram,
            model.recommended_ram_gb,
        );
        let gpu_tps = plan_estimate_tps(
            model,
            quant,
            system.backend,
            PlanRunPath::Gpu,
            system.total_cpu_cores,
            gpu_name,
        );
        if target_tps.map(|t| gpu_tps >= t).unwrap_or(true) {
            candidates.push((gpu_fit, PlanRunPath::Gpu, gpu_tps));
        }

        if !system.unified_memory {
            let offload_fit = fit_level_for(
                PlanRunPath::CpuOffload,
                model_mem,
                system.available_ram_gb,
                model.recommended_ram_gb,
            );
            let offload_tps = plan_estimate_tps(
                model,
                quant,
                system.backend,
                PlanRunPath::CpuOffload,
                system.total_cpu_cores,
                gpu_name,
            );
            if target_tps.map(|t| offload_tps >= t).unwrap_or(true) {
                candidates.push((offload_fit, PlanRunPath::CpuOffload, offload_tps));
            }
        }
    }

    let cpu_fit = fit_level_for(
        PlanRunPath::CpuOnly,
        model_mem,
        system.available_ram_gb,
        model.recommended_ram_gb,
    );
    let cpu_tps = plan_estimate_tps(
        model,
        quant,
        default_gpu_backend(system),
        PlanRunPath::CpuOnly,
        system.total_cpu_cores,
        None,
    );
    if target_tps.map(|t| cpu_tps >= t).unwrap_or(true) {
        candidates.push((cpu_fit, PlanRunPath::CpuOnly, cpu_tps));
    }

    candidates.sort_by(|a, b| {
        b.0.rank().cmp(&a.0.rank()).then_with(|| {
            let path_rank = |path: PlanRunPath| match path {
                PlanRunPath::Gpu => 3,
                PlanRunPath::CpuOffload => 2,
                PlanRunPath::CpuOnly => 1,
            };
            path_rank(b.1).cmp(&path_rank(a.1))
        })
    });

    if let Some((fit_level, path, tps)) = candidates.first() {
        PlanCurrentStatus {
            fit_level: *fit_level,
            run_mode: path.run_mode(),
            estimated_tps: *tps,
        }
    } else {
        PlanCurrentStatus {
            fit_level: FitLevel::TooTight,
            run_mode: RunMode::CpuOnly,
            estimated_tps: 0.0,
        }
    }
}

fn build_path_estimate(
    model: &LlmModel,
    quant: &str,
    context: u32,
    target_tps: Option<f64>,
    path: PlanRunPath,
    system: &SystemSpecs,
) -> PathEstimate {
    let model_mem = model.estimate_memory_gb(quant, context);
    let backend = default_gpu_backend(system);
    let gpu_name = system.gpu_name.as_deref();

    let min_cores =
        match minimum_cores_for_target(model, quant, backend, path, target_tps, gpu_name) {
            Some(v) => v,
            None => {
                return PathEstimate {
                    path,
                    feasible: false,
                    minimum: None,
                    recommended: None,
                    estimated_tps: None,
                    fit_level: None,
                    notes: vec!["target tps unreachable with current heuristics".to_string()],
                };
            }
        };

    let recommended_cores = min_cores.max(8);

    match path {
        PlanRunPath::Gpu => {
            let min_vram = model_mem;
            let rec_vram = model.recommended_ram_gb.max(model_mem * 1.2);
            let min_ram = (model_mem * 0.2).max(8.0);
            let rec_ram = (min_ram * 1.25).max(12.0);
            let tps = plan_estimate_tps(model, quant, backend, path, min_cores, gpu_name);
            let fit = fit_level_for(path, min_vram, min_vram, model.recommended_ram_gb);

            PathEstimate {
                path,
                feasible: true,
                minimum: Some(HardwareEstimate {
                    vram_gb: Some(min_vram),
                    ram_gb: min_ram,
                    cpu_cores: min_cores,
                }),
                recommended: Some(HardwareEstimate {
                    vram_gb: Some(rec_vram),
                    ram_gb: rec_ram,
                    cpu_cores: recommended_cores,
                }),
                estimated_tps: Some(tps),
                fit_level: Some(fit),
                notes: vec!["estimate from quant/context fit heuristics".to_string()],
            }
        }
        PlanRunPath::CpuOffload => {
            if system.unified_memory {
                return PathEstimate {
                    path,
                    feasible: false,
                    minimum: None,
                    recommended: None,
                    estimated_tps: None,
                    fit_level: None,
                    notes: vec!["cpu offload skipped on unified-memory systems".to_string()],
                };
            }

            let min_ram = model_mem;
            let rec_ram = model_mem * 1.2;
            let tps = plan_estimate_tps(model, quant, backend, path, min_cores, gpu_name);
            let fit = fit_level_for(path, min_ram, min_ram, model.recommended_ram_gb);

            PathEstimate {
                path,
                feasible: true,
                minimum: Some(HardwareEstimate {
                    vram_gb: Some(2.0),
                    ram_gb: min_ram,
                    cpu_cores: min_cores,
                }),
                recommended: Some(HardwareEstimate {
                    vram_gb: Some(4.0),
                    ram_gb: rec_ram,
                    cpu_cores: recommended_cores,
                }),
                estimated_tps: Some(tps),
                fit_level: Some(fit),
                notes: vec!["ram is primary pool in cpu offload mode".to_string()],
            }
        }
        PlanRunPath::CpuOnly => {
            let min_ram = model_mem;
            let rec_ram = model_mem * 1.2;
            let tps = plan_estimate_tps(model, quant, GpuBackend::CpuX86, path, min_cores, None);
            let fit = fit_level_for(path, min_ram, min_ram, model.recommended_ram_gb);

            PathEstimate {
                path,
                feasible: true,
                minimum: Some(HardwareEstimate {
                    vram_gb: None,
                    ram_gb: min_ram,
                    cpu_cores: min_cores,
                }),
                recommended: Some(HardwareEstimate {
                    vram_gb: None,
                    ram_gb: rec_ram,
                    cpu_cores: recommended_cores,
                }),
                estimated_tps: Some(tps),
                fit_level: Some(fit),
                notes: vec!["cpu-only fit is capped at marginal".to_string()],
            }
        }
    }
}

pub fn estimate_model_plan(
    model: &LlmModel,
    request: &PlanRequest,
    system: &SystemSpecs,
) -> Result<PlanEstimate, String> {
    if request.context == 0 {
        return Err("--context must be greater than 0".to_string());
    }

    if let Some(target) = request.target_tps
        && target <= 0.0
    {
        return Err("--target-tps must be greater than 0".to_string());
    }

    let quant = if let Some(ref q) = request.quant {
        normalize_quant(q).ok_or_else(|| format!("Unsupported quantization '{q}'"))?
    } else {
        model.quantization.clone()
    };

    let context = request.context;
    let run_paths = vec![
        build_path_estimate(
            model,
            &quant,
            context,
            request.target_tps,
            PlanRunPath::Gpu,
            system,
        ),
        build_path_estimate(
            model,
            &quant,
            context,
            request.target_tps,
            PlanRunPath::CpuOffload,
            system,
        ),
        build_path_estimate(
            model,
            &quant,
            context,
            request.target_tps,
            PlanRunPath::CpuOnly,
            system,
        ),
    ];

    let current = evaluate_current(model, &quant, context, request.target_tps, system);

    let preferred = run_paths
        .iter()
        .find(|p| p.path == PlanRunPath::Gpu && p.feasible)
        .or_else(|| {
            run_paths
                .iter()
                .find(|p| p.path == PlanRunPath::CpuOffload && p.feasible)
        })
        .or_else(|| {
            run_paths
                .iter()
                .find(|p| p.path == PlanRunPath::CpuOnly && p.feasible)
        })
        .ok_or_else(|| "No feasible run path found".to_string())?;

    let minimum = preferred
        .minimum
        .clone()
        .ok_or_else(|| "Missing minimum estimate".to_string())?;
    let recommended = preferred
        .recommended
        .clone()
        .ok_or_else(|| "Missing recommended estimate".to_string())?;

    let mut upgrade_deltas = Vec::new();

    let current_vram = system
        .total_gpu_vram_gb
        .or(system.gpu_vram_gb)
        .unwrap_or(0.0);
    if let Some(gpu_path) = run_paths.iter().find(|p| p.path == PlanRunPath::Gpu)
        && let Some(min_hw) = &gpu_path.minimum
    {
        let add_good = (min_hw.vram_gb.unwrap_or(0.0) - current_vram).max(0.0);
        upgrade_deltas.push(UpgradeDelta {
            resource: "vram_gb".to_string(),
            add_gb: Some(add_good),
            add_cores: None,
            target_fit: Some(FitLevel::Good),
            path: PlanRunPath::Gpu,
            description: format!("+{add_good:.1} GB VRAM -> good"),
        });
    }

    if let Some(gpu_path) = run_paths.iter().find(|p| p.path == PlanRunPath::Gpu)
        && let Some(rec_hw) = &gpu_path.recommended
    {
        let add_perfect = (rec_hw.vram_gb.unwrap_or(0.0) - current_vram).max(0.0);
        upgrade_deltas.push(UpgradeDelta {
            resource: "vram_gb".to_string(),
            add_gb: Some(add_perfect),
            add_cores: None,
            target_fit: Some(FitLevel::Perfect),
            path: PlanRunPath::Gpu,
            description: format!("+{add_perfect:.1} GB VRAM -> perfect"),
        });
    }

    let current_ram = system.available_ram_gb;
    if minimum.ram_gb > current_ram {
        let add_ram = minimum.ram_gb - current_ram;
        upgrade_deltas.push(UpgradeDelta {
            resource: "ram_gb".to_string(),
            add_gb: Some(add_ram),
            add_cores: None,
            target_fit: Some(FitLevel::Marginal),
            path: preferred.path,
            description: format!("+{add_ram:.1} GB RAM -> runnable"),
        });
    }

    if minimum.cpu_cores > system.total_cpu_cores {
        let add_cores = minimum.cpu_cores - system.total_cpu_cores;
        upgrade_deltas.push(UpgradeDelta {
            resource: "cpu_cores".to_string(),
            add_gb: None,
            add_cores: Some(add_cores),
            target_fit: None,
            path: preferred.path,
            description: format!("+{add_cores} CPU cores -> target tps"),
        });
    }

    Ok(PlanEstimate {
        estimate_notice: "Estimate-based output using llmfit heuristics; not an exact benchmark."
            .to_string(),
        model_name: model.name.clone(),
        provider: model.provider.clone(),
        context,
        quantization: quant,
        target_tps: request.target_tps,
        minimum,
        recommended,
        run_paths,
        current,
        upgrade_deltas,
    })
}
