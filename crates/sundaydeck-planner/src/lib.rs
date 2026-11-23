pub mod benchmark_corpus;
pub mod embedded_llm;
pub mod error;
pub mod local_llm;
pub mod prompt;

use std::time::Instant;

use serde::{Deserialize, Serialize};
use sundaydeck_core::{
    BibleVerse, ExtractedDocument, GeneratedServicePlan, ScriptureRef, Song, Theme,
    build_service_plan,
};

pub use crate::embedded_llm::{
    EmbeddedLlmConfig, default_embedded_model_path, is_embedded_runtime_compiled,
};
pub use crate::error::PlannerError;
pub use crate::local_llm::LocalLlmConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerEngineKind {
    #[default]
    Deterministic,
    LocalLlm,
    Embedded,
}

impl PlannerEngineKind {
    pub const ALL: [Self; 3] = [Self::Deterministic, Self::LocalLlm, Self::Embedded];

    pub const fn short_badge(self) -> &'static str {
        match self {
            Self::Deterministic => "NO-LLM",
            Self::LocalLlm => "LOCAL LLM",
            Self::Embedded => "EMBEDDED",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Deterministic => "Pure-Rust rules + 66-book Bible & song matcher",
            Self::LocalLlm => "Local OpenAI-compatible server (llama-server / Ollama)",
            Self::Embedded => "In-process GGUF model via llama.cpp (zero daemon)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannerConfig {
    pub engine: PlannerEngineKind,
    pub local_llm: LocalLlmConfig,
    pub embedded_llm: EmbeddedLlmConfig,
    /// When true, falls back cleanly to [`PlannerEngineKind::Deterministic`] if an LLM engine
    /// is offline, missing weights, or returns malformed JSON.
    pub allow_fallback: bool,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            engine: PlannerEngineKind::Deterministic,
            local_llm: LocalLlmConfig::default(),
            embedded_llm: EmbeddedLlmConfig::default(),
            allow_fallback: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRunMetrics {
    pub engine: PlannerEngineKind,
    pub model_label: String,
    pub cold_load_ms: u64,
    pub inference_ms: u64,
    pub total_ms: u64,
    pub rss_before_mb: f64,
    pub rss_after_mb: f64,
    pub rss_delta_mb: f64,
    pub used_fallback: bool,
    pub fallback_reason: Option<String>,
    pub raw_json_valid: bool,
    pub prompt_chars: usize,
    pub output_chars: usize,
}

impl PlanRunMetrics {
    pub fn telemetry_summary(&self) -> String {
        let timing = if self.cold_load_ms > 0 {
            format!("{}ms (LOAD {}ms)", self.total_ms, self.cold_load_ms)
        } else if self.total_ms == 0 {
            "<1ms".to_owned()
        } else {
            format!("{}ms", self.total_ms)
        };
        if self.used_fallback {
            format!(
                "{} -> FALLBACK NO-LLM // {timing}",
                self.engine.short_badge()
            )
        } else {
            format!(
                "{} ({}) // {timing}",
                self.engine.short_badge(),
                self.model_label
            )
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanRunOutcome {
    pub plan: GeneratedServicePlan,
    pub metrics: PlanRunMetrics,
}

/// Runs the selected planner engine (`Deterministic`, `LocalLlm`, or `Embedded`) against an
/// extracted document, measuring real latency and RSS delta, with optional deterministic fallback.
pub fn run_planner<F>(
    config: &PlannerConfig,
    doc: &ExtractedDocument,
    songs: &[Song],
    themes: &[Theme],
    translation_code: &str,
    mut lookup_verses: F,
) -> Result<PlanRunOutcome, PlannerError>
where
    F: FnMut(&ScriptureRef) -> Vec<BibleVerse>,
{
    let total_start = Instant::now();
    let rss_before_mb = read_process_rss_mb();

    if config.engine == PlannerEngineKind::Deterministic {
        let plan = build_service_plan(doc, songs, themes, translation_code, &mut lookup_verses);
        let total_ms = total_start.elapsed().as_millis() as u64;
        let rss_after_mb = read_process_rss_mb();
        return Ok(PlanRunOutcome {
            plan,
            metrics: PlanRunMetrics {
                engine: PlannerEngineKind::Deterministic,
                model_label: "pure-rust-rules".to_owned(),
                cold_load_ms: 0,
                inference_ms: total_ms,
                total_ms,
                rss_before_mb,
                rss_after_mb,
                rss_delta_mb: (rss_after_mb - rss_before_mb).max(0.0),
                used_fallback: false,
                fallback_reason: None,
                raw_json_valid: true,
                prompt_chars: 0,
                output_chars: 0,
            },
        });
    }

    let user_prompt = prompt::build_user_prompt(doc, songs, themes, translation_code);
    let prompt_chars = prompt::SYSTEM_PROMPT.len() + user_prompt.len();
    let model_label = match config.engine {
        PlannerEngineKind::Deterministic => "pure-rust-rules".to_owned(),
        PlannerEngineKind::LocalLlm => config.local_llm.model.clone(),
        PlannerEngineKind::Embedded => config
            .embedded_llm
            .model_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("embedded.gguf")
            .to_owned(),
    };

    let call_result: Result<(String, u64, u64), PlannerError> = match config.engine {
        PlannerEngineKind::LocalLlm => {
            let t0 = Instant::now();
            local_llm::call_local_llm(&config.local_llm, prompt::SYSTEM_PROMPT, &user_prompt)
                .map(|raw| (raw, 0, t0.elapsed().as_millis() as u64))
        }
        PlannerEngineKind::Embedded => embedded_llm::call_embedded_llm(
            &config.embedded_llm,
            prompt::SYSTEM_PROMPT,
            &user_prompt,
        )
        .map(|out| (out.raw_text, out.cold_load_ms, out.inference_ms)),
        PlannerEngineKind::Deterministic => unreachable!(),
    };

    match call_result {
        Ok((raw_text, cold_load_ms, inference_ms)) => {
            let output_chars = raw_text.len();
            match prompt::materialize_llm_plan(
                &raw_text,
                doc,
                songs,
                themes,
                translation_code,
                &mut lookup_verses,
            ) {
                Ok(plan) => {
                    let total_ms = total_start.elapsed().as_millis() as u64;
                    let rss_after_mb = read_process_rss_mb();
                    Ok(PlanRunOutcome {
                        plan,
                        metrics: PlanRunMetrics {
                            engine: config.engine,
                            model_label,
                            cold_load_ms,
                            inference_ms,
                            total_ms,
                            rss_before_mb,
                            rss_after_mb,
                            rss_delta_mb: (rss_after_mb - rss_before_mb).max(0.0),
                            used_fallback: false,
                            fallback_reason: None,
                            raw_json_valid: true,
                            prompt_chars,
                            output_chars,
                        },
                    })
                }
                Err(err) if config.allow_fallback => Ok(fallback_outcome(
                    config.engine,
                    model_label,
                    cold_load_ms,
                    inference_ms,
                    total_start,
                    rss_before_mb,
                    prompt_chars,
                    output_chars,
                    err.to_string(),
                    doc,
                    songs,
                    themes,
                    translation_code,
                    lookup_verses,
                )),
                Err(err) => Err(err),
            }
        }
        Err(err) if config.allow_fallback => Ok(fallback_outcome(
            config.engine,
            model_label,
            0,
            0,
            total_start,
            rss_before_mb,
            prompt_chars,
            0,
            err.to_string(),
            doc,
            songs,
            themes,
            translation_code,
            lookup_verses,
        )),
        Err(err) => Err(err),
    }
}

#[allow(clippy::too_many_arguments)]
fn fallback_outcome<F>(
    engine: PlannerEngineKind,
    model_label: String,
    cold_load_ms: u64,
    inference_ms: u64,
    total_start: Instant,
    rss_before_mb: f64,
    prompt_chars: usize,
    output_chars: usize,
    reason: String,
    doc: &ExtractedDocument,
    songs: &[Song],
    themes: &[Theme],
    translation_code: &str,
    lookup_verses: F,
) -> PlanRunOutcome
where
    F: FnMut(&ScriptureRef) -> Vec<BibleVerse>,
{
    let plan = build_service_plan(doc, songs, themes, translation_code, lookup_verses);
    let total_ms = total_start.elapsed().as_millis() as u64;
    let rss_after_mb = read_process_rss_mb();
    PlanRunOutcome {
        plan,
        metrics: PlanRunMetrics {
            engine,
            model_label,
            cold_load_ms,
            inference_ms,
            total_ms,
            rss_before_mb,
            rss_after_mb,
            rss_delta_mb: (rss_after_mb - rss_before_mb).max(0.0),
            used_fallback: true,
            fallback_reason: Some(reason),
            raw_json_valid: false,
            prompt_chars,
            output_chars,
        },
    }
}

/// Reads current process resident set size (`VmRSS`) in MiB from Linux `/proc/self/status`.
pub fn read_process_rss_mb() -> f64 {
    read_proc_status_kb("VmRSS:") / 1024.0
}

/// Reads peak process resident set size (`VmHWM`) in MiB from Linux `/proc/self/status`.
pub fn read_process_peak_rss_mb() -> f64 {
    read_proc_status_kb("VmHWM:") / 1024.0
}

fn read_proc_status_kb(field: &str) -> f64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return 0.0;
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix(field)
            && let Some(num) = rest.split_whitespace().next()
            && let Ok(kb) = num.parse::<f64>()
        {
            return kb;
        }
    }
    0.0
}
