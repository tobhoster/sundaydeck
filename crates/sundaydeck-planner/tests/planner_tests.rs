use sundaydeck_core::{PlanItemKind, ServiceItemPayload};
use sundaydeck_planner::benchmark_corpus::{
    benchmark_songs, benchmark_themes, build_benchmark_corpus, lookup_starter_web, score_plan,
};
use sundaydeck_planner::prompt::{extract_json_object, materialize_llm_plan};
use sundaydeck_planner::{LocalLlmConfig, PlannerConfig, PlannerEngineKind, run_planner};

#[test]
fn deterministic_baseline_scores_clean_docs_high_and_exposes_implicit_gap() {
    let corpus = build_benchmark_corpus();
    let songs = benchmark_songs();
    let themes = benchmark_themes();
    let cfg = PlannerConfig::default();

    let mut clean_scores = Vec::new();
    let mut messy_scores = Vec::new();
    for case in &corpus {
        let outcome = run_planner(&cfg, &case.doc, &songs, &themes, "WEB", lookup_starter_web)
            .expect("deterministic plan");
        assert!(!outcome.metrics.used_fallback);
        assert!(outcome.metrics.raw_json_valid);
        let score = score_plan(case, &outcome.plan);
        if case.id.starts_with("01") || case.id.starts_with("02") {
            clean_scores.push(score.quality_score_pct);
        } else {
            messy_scores.push(score.quality_score_pct);
        }
    }

    assert!(
        clean_scores.iter().all(|&s| s >= 95.0),
        "clean OOXML files should score >= 95% deterministically, got {clean_scores:?}"
    );
    assert!(
        messy_scores[0] < 70.0,
        "messy implicit case should expose the rules gap that LLM engines can close, got {messy_scores:?}"
    );
}

#[test]
fn llm_materializer_links_real_sqlite_songs_and_never_hallucinates_scripture_text() {
    let corpus = build_benchmark_corpus();
    let messy_case = &corpus[2];
    let songs = benchmark_songs();
    let themes = benchmark_themes();

    // Simulated fenced LLM response resolving implicit cues from messy notes.
    let raw_llm = r#"```json
{
  "sermon_title": "Finding Rest When Life Feels Overwhelming",
  "items": [
    {
      "kind": "song_match",
      "title": "Opening Worship",
      "song_title": "Amazing Grace",
      "scripture_ref": null,
      "slides": [],
      "speaker_note": null
    },
    {
      "kind": "scripture",
      "title": "Shepherd Psalm",
      "song_title": null,
      "scripture_ref": "Psalms 23:1-4",
      "slides": ["Ignore any LLM verse text"],
      "speaker_note": null
    },
    {
      "kind": "sermon_point",
      "title": "Trust the Shepherd, Not Control",
      "song_title": null,
      "scripture_ref": null,
      "slides": [
        "We chase control because we forget who actually leads us.",
        "Peace comes from trusting the Shepherd who walks beside us in the valley."
      ],
      "speaker_note": "Band softly bring keys pad back in during this reflection."
    },
    {
      "kind": "song_match",
      "title": "Closing Hymn",
      "song_title": "It Is Well with My Soul",
      "scripture_ref": null,
      "slides": [],
      "speaker_note": null
    }
  ]
}
```"#;

    assert!(extract_json_object(raw_llm).is_some());
    let plan = materialize_llm_plan(
        raw_llm,
        &messy_case.doc,
        &songs,
        &themes,
        "WEB",
        lookup_starter_web,
    )
    .expect("materialize llm json");

    assert_eq!(plan.stats.songs_matched, 2);
    assert_eq!(plan.stats.scriptures_resolved, 1);
    assert_eq!(plan.stats.stage_notes, 1);
    assert_eq!(plan.items[0].kind, PlanItemKind::SongMatch);
    assert_eq!(plan.items[0].title, "Amazing Grace");
    assert!(matches!(
        &plan.items[1].item.payload,
        ServiceItemPayload::Scripture { cached_verses, .. }
            if cached_verses.len() == 4 && cached_verses[0].1.contains("Yahweh is my shepherd")
    ));

    let score = score_plan(messy_case, &plan);
    assert!(
        score.quality_score_pct >= 99.0,
        "resolved implicit plan should score ~100%, got {:.1}%",
        score.quality_score_pct
    );
}

#[test]
fn offline_local_llm_falls_back_cleanly_to_deterministic_rules() {
    let corpus = build_benchmark_corpus();
    let songs = benchmark_songs();
    let themes = benchmark_themes();
    let cfg = PlannerConfig {
        engine: PlannerEngineKind::LocalLlm,
        local_llm: LocalLlmConfig {
            endpoint_url: "http://127.0.0.1:1/v1/chat/completions".to_owned(),
            timeout_secs: 2,
            ..LocalLlmConfig::default()
        },
        allow_fallback: true,
        ..PlannerConfig::default()
    };

    let outcome = run_planner(
        &cfg,
        &corpus[0].doc,
        &songs,
        &themes,
        "WEB",
        lookup_starter_web,
    )
    .expect("fallback must succeed cleanly");

    assert!(outcome.metrics.used_fallback);
    assert!(!outcome.metrics.raw_json_valid);
    assert!(outcome.metrics.fallback_reason.is_some());
    assert_eq!(outcome.plan.stats.songs_matched, 1);
}
