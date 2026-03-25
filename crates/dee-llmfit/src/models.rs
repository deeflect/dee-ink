use anyhow::Context;
use serde::{Deserialize, Serialize};

pub const QUANT_HIERARCHY: &[&str] = &["Q8_0", "Q6_K", "Q5_K_M", "Q4_K_M", "Q3_K_M", "Q2_K"];
pub const MLX_QUANT_HIERARCHY: &[&str] = &["mlx-8bit", "mlx-4bit"];

pub fn quant_bpp(quant: &str) -> f64 {
    match quant {
        "F32" => 4.0,
        "F16" | "BF16" => 2.0,
        "Q8_0" => 1.05,
        "Q6_K" => 0.80,
        "Q5_K_M" => 0.68,
        "Q4_K_M" | "Q4_0" => 0.58,
        "Q3_K_M" => 0.48,
        "Q2_K" => 0.37,
        "mlx-4bit" => 0.55,
        "mlx-8bit" => 1.0,
        _ => 0.58,
    }
}

pub fn quant_speed_multiplier(quant: &str) -> f64 {
    match quant {
        "F16" | "BF16" => 0.6,
        "Q8_0" => 0.8,
        "Q6_K" => 0.95,
        "Q5_K_M" => 1.0,
        "Q4_K_M" | "Q4_0" => 1.15,
        "Q3_K_M" => 1.25,
        "Q2_K" => 1.35,
        "mlx-4bit" => 1.15,
        "mlx-8bit" => 0.85,
        _ => 1.0,
    }
}

pub fn quant_bytes_per_param(quant: &str) -> f64 {
    match quant {
        "F16" | "BF16" => 2.0,
        "Q8_0" => 1.0,
        "Q6_K" => 0.75,
        "Q5_K_M" => 0.625,
        "Q4_K_M" | "Q4_0" => 0.5,
        "Q3_K_M" => 0.375,
        "Q2_K" => 0.25,
        "mlx-4bit" => 0.5,
        "mlx-8bit" => 1.0,
        _ => 0.5,
    }
}

pub fn quant_quality_penalty(quant: &str) -> f64 {
    match quant {
        "F16" | "BF16" => 0.0,
        "Q8_0" => 0.0,
        "Q6_K" => -1.0,
        "Q5_K_M" => -2.0,
        "Q4_K_M" | "Q4_0" => -5.0,
        "Q3_K_M" => -8.0,
        "Q2_K" => -12.0,
        "mlx-4bit" => -4.0,
        "mlx-8bit" => 0.0,
        _ => -5.0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UseCase {
    General,
    Coding,
    Reasoning,
    Chat,
    Multimodal,
    Embedding,
}

impl UseCase {
    pub fn label(&self) -> &'static str {
        match self {
            UseCase::General => "general",
            UseCase::Coding => "coding",
            UseCase::Reasoning => "reasoning",
            UseCase::Chat => "chat",
            UseCase::Multimodal => "multimodal",
            UseCase::Embedding => "embedding",
        }
    }

    pub fn from_str_label(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "general" => Some(Self::General),
            "coding" => Some(Self::Coding),
            "reasoning" => Some(Self::Reasoning),
            "chat" => Some(Self::Chat),
            "multimodal" => Some(Self::Multimodal),
            "embedding" => Some(Self::Embedding),
            _ => None,
        }
    }

    pub fn from_model(model: &LlmModel) -> Self {
        let name = model.name.to_lowercase();
        let use_case = model.use_case.to_lowercase();

        if use_case.contains("embedding") || name.contains("embed") || name.contains("bge") {
            Self::Embedding
        } else if name.contains("code") || use_case.contains("code") {
            Self::Coding
        } else if use_case.contains("vision") || use_case.contains("multimodal") {
            Self::Multimodal
        } else if use_case.contains("reason")
            || use_case.contains("chain-of-thought")
            || name.contains("deepseek-r1")
        {
            Self::Reasoning
        } else if use_case.contains("chat") || use_case.contains("instruction") {
            Self::Chat
        } else {
            Self::General
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModel {
    pub name: String,
    pub provider: String,
    pub parameter_count: String,
    #[serde(default)]
    pub parameters_raw: Option<u64>,
    pub min_ram_gb: f64,
    pub recommended_ram_gb: f64,
    pub min_vram_gb: Option<f64>,
    pub quantization: String,
    pub context_length: u32,
    pub use_case: String,
    #[serde(default)]
    pub is_moe: bool,
    #[serde(default)]
    pub num_experts: Option<u32>,
    #[serde(default)]
    pub active_experts: Option<u32>,
    #[serde(default)]
    pub active_parameters: Option<u64>,
    #[serde(default)]
    pub release_date: Option<String>,
}

impl LlmModel {
    pub fn is_mlx_model(&self) -> bool {
        let name_lower = self.name.to_lowercase();
        name_lower.contains("-mlx-") || name_lower.ends_with("-mlx")
    }

    fn quant_bpp(&self) -> f64 {
        quant_bpp(&self.quantization)
    }

    pub fn params_b(&self) -> f64 {
        if let Some(raw) = self.parameters_raw {
            raw as f64 / 1_000_000_000.0
        } else {
            let s = self.parameter_count.trim().to_uppercase();
            if let Some(num_str) = s.strip_suffix('B') {
                num_str.parse::<f64>().unwrap_or(7.0)
            } else if let Some(num_str) = s.strip_suffix('M') {
                num_str.parse::<f64>().unwrap_or(0.0) / 1000.0
            } else {
                7.0
            }
        }
    }

    pub fn estimate_memory_gb(&self, quant: &str, ctx: u32) -> f64 {
        let bpp = quant_bpp(quant);
        let params = self.params_b();
        let model_mem = params * bpp;
        let kv_cache = 0.000008 * params * ctx as f64;
        let overhead = 0.5;
        model_mem + kv_cache + overhead
    }

    pub fn best_quant_for_budget(&self, budget_gb: f64, ctx: u32) -> Option<(&'static str, f64)> {
        self.best_quant_for_budget_with(budget_gb, ctx, QUANT_HIERARCHY)
    }

    pub fn best_quant_for_budget_with(
        &self,
        budget_gb: f64,
        ctx: u32,
        hierarchy: &[&'static str],
    ) -> Option<(&'static str, f64)> {
        for &q in hierarchy {
            let mem = self.estimate_memory_gb(q, ctx);
            if mem <= budget_gb {
                return Some((q, mem));
            }
        }

        let half_ctx = ctx / 2;
        if half_ctx >= 1024 {
            for &q in hierarchy {
                let mem = self.estimate_memory_gb(q, half_ctx);
                if mem <= budget_gb {
                    return Some((q, mem));
                }
            }
        }

        None
    }

    pub fn moe_active_vram_gb(&self) -> Option<f64> {
        if !self.is_moe {
            return None;
        }
        let active_params = self.active_parameters? as f64;
        let bpp = self.quant_bpp();
        let size_gb = (active_params * bpp) / (1024.0 * 1024.0 * 1024.0);
        Some((size_gb * 1.1).max(0.5))
    }

    pub fn moe_offloaded_ram_gb(&self) -> Option<f64> {
        if !self.is_moe {
            return None;
        }
        let active = self.active_parameters? as f64;
        let total = self.parameters_raw? as f64;
        let inactive = total - active;
        if inactive <= 0.0 {
            return Some(0.0);
        }
        let bpp = self.quant_bpp();
        Some((inactive * bpp) / (1024.0 * 1024.0 * 1024.0))
    }
}

const CURATED_MODELS_JSON: &str = include_str!("../data/models.json");

pub struct ModelDatabase {
    models: Vec<LlmModel>,
}

impl ModelDatabase {
    pub fn new() -> anyhow::Result<Self> {
        let models: Vec<LlmModel> = serde_json::from_str(CURATED_MODELS_JSON)
            .context("failed parsing embedded model database")?;
        Ok(Self { models })
    }

    pub fn all(&self) -> &[LlmModel] {
        &self.models
    }

    pub fn search(&self, query: &str) -> Vec<&LlmModel> {
        let needle = query.trim().to_lowercase();
        self.models
            .iter()
            .filter(|m| {
                m.name.to_lowercase().contains(&needle)
                    || m.provider.to_lowercase().contains(&needle)
                    || m.parameter_count.to_lowercase().contains(&needle)
                    || m.use_case.to_lowercase().contains(&needle)
            })
            .collect()
    }

    pub fn resolve_model_selector(&self, selector: &str) -> Result<&LlmModel, String> {
        let needle = selector.trim().to_lowercase();
        if needle.is_empty() {
            return Err("Model selector cannot be empty".to_string());
        }

        let exact: Vec<&LlmModel> = self
            .models
            .iter()
            .filter(|m| m.name.to_lowercase() == needle)
            .collect();
        if exact.len() == 1 {
            return Ok(exact[0]);
        }

        let partial: Vec<&LlmModel> = self
            .models
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&needle))
            .collect();

        match partial.len() {
            0 => Err(format!("No model found matching '{selector}'.")),
            1 => Ok(partial[0]),
            _ => {
                let suggestions = partial
                    .iter()
                    .take(10)
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                Err(format!(
                    "Model selector '{selector}' is ambiguous. Matches: {suggestions}"
                ))
            }
        }
    }
}
