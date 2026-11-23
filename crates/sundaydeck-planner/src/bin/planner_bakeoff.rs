use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sundaydeck_planner::benchmark_corpus::{
    CaseScore, benchmark_songs, benchmark_themes, build_benchmark_corpus, lookup_starter_web,
    score_plan,
};
use sundaydeck_planner::{
    EmbeddedLlmConfig, LocalLlmConfig, PlanRunMetrics, PlannerConfig, PlannerEngineKind,
    default_embedded_model_path, read_process_peak_rss_mb, run_planner,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaseRecord {
    metrics: PlanRunMetrics,
    score: CaseScore,
    plan_titles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EngineSummary {
    engine: PlannerEngineKind,
    engine_badge: String,
    model_label: String,
    cold_load_ms: u64,
    avg_warm_ms: f64,
    peak_rss_mb: f64,
    valid_json_cases: usize,
    fallback_cases: usize,
    avg_quality_pct: f64,
    clean_quality_pct: f64,
    messy_quality_pct: f64,
    cases: Vec<CaseRecord>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let engines = parse_engines(&args);
    let local_url = arg_value(&args, "--local-url")
        .unwrap_or_else(|| "http://127.0.0.1:8080/v1/chat/completions".to_owned());
    let local_model =
        arg_value(&args, "--local-model").unwrap_or_else(|| "qwen2.5-3b-instruct".to_owned());
    let embedded_path = arg_value(&args, "--embedded-model")
        .map(PathBuf::from)
        .unwrap_or_else(default_embedded_model_path);
    let out_md = arg_value(&args, "--out-md").map(PathBuf::from);
    let out_json = arg_value(&args, "--out-json").map(PathBuf::from);

    let corpus = build_benchmark_corpus();
    let songs = benchmark_songs();
    let themes = benchmark_themes();
    let mut summaries = Vec::new();

    for engine in engines {
        let cfg = PlannerConfig {
            engine,
            local_llm: LocalLlmConfig {
                endpoint_url: local_url.clone(),
                model: local_model.clone(),
                ..LocalLlmConfig::default()
            },
            embedded_llm: EmbeddedLlmConfig {
                model_path: embedded_path.clone(),
                ..EmbeddedLlmConfig::default()
            },
            allow_fallback: true,
        };

        eprintln!(
            "\n=== Running engine: {} ({}) ===",
            engine.short_badge(),
            engine.description()
        );

        let mut records = Vec::new();
        for case in &corpus {
            match run_planner(&cfg, &case.doc, &songs, &themes, "WEB", lookup_starter_web) {
                Ok(outcome) => {
                    let score = score_plan(case, &outcome.plan);
                    let plan_titles: Vec<String> = outcome
                        .plan
                        .items
                        .iter()
                        .map(|i| format!("[{}] {}", i.kind.badge(), i.title))
                        .collect();
                    eprintln!(
                        "  {:<15} | {:>5}ms (load {:>4}ms) | quality {:>5.1}% | songs {}/{} | bible {}/{} | notes {}/{} | fallback={}",
                        case.id,
                        outcome.metrics.total_ms,
                        outcome.metrics.cold_load_ms,
                        score.quality_score_pct,
                        score.songs_matched_expected,
                        score.songs_expected,
                        score.scriptures_matched_expected,
                        score.scriptures_expected,
                        score.stage_notes_found,
                        score.stage_notes_expected_min,
                        outcome.metrics.used_fallback
                    );
                    if let Some(reason) = &outcome.metrics.fallback_reason {
                        eprintln!("    fallback reason: {reason}");
                    }
                    records.push(CaseRecord {
                        metrics: outcome.metrics,
                        score,
                        plan_titles,
                    });
                }
                Err(err) => {
                    eprintln!("  {} FAILED: {err}", case.id);
                }
            }
        }

        summaries.push(aggregate_summary(engine, records));
    }

    let report = render_markdown_report(&summaries);
    println!("{report}");

    if let Some(path) = out_md {
        let _ = std::fs::write(&path, &report);
        eprintln!("Wrote markdown report to {}", path.display());
    }
    if let Some(path) = out_json
        && let Ok(json) = serde_json::to_string_pretty(&summaries)
    {
        let _ = std::fs::write(&path, json);
        eprintln!("Wrote JSON telemetry to {}", path.display());
    }
}

fn aggregate_summary(engine: PlannerEngineKind, cases: Vec<CaseRecord>) -> EngineSummary {
    let cold_load_ms = cases
        .iter()
        .map(|c| c.metrics.cold_load_ms)
        .max()
        .unwrap_or(0);
    let warm_times: Vec<f64> = cases
        .iter()
        .map(|c| c.metrics.inference_ms as f64)
        .collect();
    let avg_warm_ms = if warm_times.is_empty() {
        0.0
    } else {
        warm_times.iter().sum::<f64>() / warm_times.len() as f64
    };
    let valid_json_cases = cases.iter().filter(|c| c.metrics.raw_json_valid).count();
    let fallback_cases = cases.iter().filter(|c| c.metrics.used_fallback).count();
    let avg_quality_pct = avg(&cases
        .iter()
        .map(|c| c.score.quality_score_pct)
        .collect::<Vec<_>>());
    let clean_quality_pct = avg(&cases
        .iter()
        .filter(|c| c.score.case_id.starts_with("01") || c.score.case_id.starts_with("02"))
        .map(|c| c.score.quality_score_pct)
        .collect::<Vec<_>>());
    let messy_quality_pct = avg(&cases
        .iter()
        .filter(|c| c.score.case_id.starts_with("03") || c.score.case_id.starts_with("04"))
        .map(|c| c.score.quality_score_pct)
        .collect::<Vec<_>>());
    let model_label = cases
        .first()
        .map_or_else(|| "n/a".to_owned(), |c| c.metrics.model_label.clone());

    EngineSummary {
        engine,
        engine_badge: engine.short_badge().to_owned(),
        model_label,
        cold_load_ms,
        avg_warm_ms,
        peak_rss_mb: read_process_peak_rss_mb(),
        valid_json_cases,
        fallback_cases,
        avg_quality_pct,
        clean_quality_pct,
        messy_quality_pct,
        cases,
    }
}

fn render_markdown_report(summaries: &[EngineSummary]) -> String {
    let mut md = String::new();
    md.push_str("# SundayDeck Planner Engine Bake-Off (No-LLM vs Local LLM vs Embedded)\n\n");
    md.push_str("| Engine | Model | Cold Load | Avg Inference | Process Peak RSS | Valid JSON | Fallbacks | Clean Docs (T1-T2) | Messy/Noisy (T3-T4) | Overall Quality |\n");
    md.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for s in summaries {
        md.push_str(&format!(
            "| **{}** | `{}` | {} ms | {:.0} ms | {:.1} MiB | {}/{} | {} | **{:.1}%** | **{:.1}%** | **{:.1}%** |\n",
            s.engine_badge,
            s.model_label,
            s.cold_load_ms,
            s.avg_warm_ms,
            s.peak_rss_mb,
            s.valid_json_cases,
            s.cases.len(),
            s.fallback_cases,
            s.clean_quality_pct,
            s.messy_quality_pct,
            s.avg_quality_pct
        ));
    }

    md.push_str("\n## Per-Case Breakdown\n\n");
    md.push_str("| Case | Engine | Latency | Songs | Scriptures | Stage Notes | Sermon Pts | Max Slide Chars | Quality |\n");
    md.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|\n");
    for s in summaries {
        for rec in &s.cases {
            md.push_str(&format!(
                "| `{}` | **{}** | {} ms | {}/{} | {}/{} | {}/{} | {} | {} | **{:.1}%** |\n",
                rec.score.case_id,
                s.engine_badge,
                rec.metrics.inference_ms,
                rec.score.songs_matched_expected,
                rec.score.songs_expected,
                rec.score.scriptures_matched_expected,
                rec.score.scriptures_expected,
                rec.score.stage_notes_found,
                rec.score.stage_notes_expected_min,
                rec.score.sermon_points_found,
                rec.score.max_slide_chars,
                rec.score.quality_score_pct,
            ));
        }
    }

    md.push_str("\n## Generated Plan Items by Engine\n");
    for s in summaries {
        md.push_str(&format!("\n### {} (`{}`)\n", s.engine_badge, s.model_label));
        for rec in &s.cases {
            md.push_str(&format!(
                "- **{}** ({} ms, {:.1}%): {}\n",
                rec.score.case_id,
                rec.metrics.inference_ms,
                rec.score.quality_score_pct,
                rec.plan_titles.join(" -> ")
            ));
        }
    }
    md
}

fn parse_engines(args: &[String]) -> Vec<PlannerEngineKind> {
    let Some(raw) = arg_value(args, "--engines") else {
        return vec![PlannerEngineKind::Deterministic];
    };
    let mut out = Vec::new();
    for part in raw.split(',') {
        match part.trim().to_ascii_lowercase().as_str() {
            "deterministic" | "no-llm" | "nollm" => out.push(PlannerEngineKind::Deterministic),
            "local" | "local-llm" | "llm" => out.push(PlannerEngineKind::LocalLlm),
            "embedded" | "gguf" => out.push(PlannerEngineKind::Embedded),
            _ => {}
        }
    }
    if out.is_empty() {
        vec![PlannerEngineKind::Deterministic]
    } else {
        out
    }
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

fn avg(vals: &[f64]) -> f64 {
    if vals.is_empty() {
        0.0
    } else {
        vals.iter().sum::<f64>() / vals.len() as f64
    }
}
