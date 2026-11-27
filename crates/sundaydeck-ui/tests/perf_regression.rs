//! Redraw-cadence and throughput regression harness (plan AC1.1–AC1.3, AC1.5, AC3).
//!
//! Counts are deterministic and asserted. Timings are printed as `PERF <metric>=<value>` with the
//! research probe's metric names so the regression lane can compare medians across runs:
//! `cargo test --release -p sundaydeck-ui --test perf_regression -- --nocapture --test-threads=1`.
mod support;

use std::time::{Duration, Instant};

use sundaydeck_core::{OutputOverride, PresentationCommand, ServiceTimer, SongId};
use sundaydeck_db::Database;
use sundaydeck_ui::design::MotionMode;
use sundaydeck_ui::design::fonts::resolve_font_family;
use sundaydeck_ui::design::tokens::FONT_BODY;
use sundaydeck_ui::views::{AudienceOutputView, StageMonitorView};
use sundaydeck_voice::{VoiceCommand, VoiceConfig, VoiceEngineHandle};
use support::{Rig, RigOptions, rig, rig_with, windows_of};

fn set_motion(r: &mut Rig, mode: MotionMode) {
    r.app.update(|cx| {
        r.store.update(cx, |s, cx| {
            s.motion_mode = mode;
            cx.notify();
        })
    });
}

#[test]
fn idle_and_live_redraw_cadence_stays_calm() {
    let mut r = rig();
    let idle = r.draws_over_secs(2);
    println!("PERF draws_2s_idle_nothing_live={idle}");
    assert!(idle <= 1, "idle operator redrew {idle} times in 2 s");

    r.go_live_with_first_song();
    // Let the go-live crossfade finish; the criterion is the steady live state.
    r.draws_over_secs(1);
    let live = r.draws_over_secs(2);
    println!("PERF draws_2s_live_motion_full={live}");
    assert!(
        live <= 26,
        "live slide + Full motion redrew {live} times in 2 s (base 120, limit 26)"
    );

    set_motion(&mut r, MotionMode::Instant);
    r.draws_over_secs(1);
    let instant = r.draws_over_secs(2);
    println!("PERF draws_2s_live_motion_instant={instant}");
    assert!(
        instant <= 1,
        "live + Instant motion redrew {instant} times in 2 s"
    );
}

#[test]
fn running_timer_redraws_about_once_per_second() {
    let mut r = rig_with(RigOptions {
        simulated_clock: true,
        ..RigOptions::default()
    });
    r.go_live_with_first_song();
    set_motion(&mut r, MotionMode::Instant);
    r.app.update(|cx| {
        r.store.update(cx, |s, cx| {
            let now = s.now_epoch_ms();
            let timer = ServiceTimer::countdown("Service", 600, false);
            let _ = s.dispatch(PresentationCommand::SetTimer(Some(timer)), cx);
            let _ = s.dispatch(PresentationCommand::StartTimer { now_ms: now }, cx);
        })
    });
    r.draws_over_secs(1);

    let (notifies, _sub) = r.count_store_notifies();
    let draws = r.draws_over_secs(2);
    println!("PERF draws_2s_timer_running_motion_instant={draws}");
    println!("PERF store_notifies_2s_timer_running={}", notifies.get());
    // The countdown text changes twice in 2 s; each change must still reach the windows.
    assert!(
        (1..=3).contains(&notifies.get()),
        "running timer notified the store {} times in 2 s (base 13; expected 1..=3)",
        notifies.get()
    );
    assert!(
        (1..=3).contains(&draws),
        "running timer redrew the operator {draws} times in 2 s (base 13; expected 1..=3)"
    );

    let (later, _sub2) = r.count_store_notifies();
    r.draws_over_secs(4);
    assert!(
        (3..=5).contains(&later.get()),
        "a running timer must refresh once per visible second: {} notifies in 4 s",
        later.get()
    );
}

#[test]
fn calm_motion_video_loop_keeps_its_blend_cadence() {
    // The motion-loop frame follows the store clock (the heartbeat redraws when the ~6 Hz loop
    // tick of `now_ms` changes), so the clock must advance with the simulated frames.
    let mut r = rig_with(RigOptions {
        simulated_clock: true,
        ..RigOptions::default()
    });
    r.app.update(|cx| {
        r.store.update(cx, |s, cx| {
            s.motion_mode = MotionMode::CrossfadeOnly;
            let motion = s
                .themes
                .iter()
                .find(|t| t.name == "Golden Light Motion")
                .cloned()
                .expect("seeded Golden Light Motion theme");
            let _ = s.dispatch(PresentationCommand::GoLive, cx);
            let _ = s.dispatch(PresentationCommand::ApplyLiveTheme(motion), cx);
        })
    });
    // Warm-up: first paint schedules keyframe decodes; measure the steady loop.
    r.draws_over_secs(1);
    let draws = r.draws_over_secs(2);
    println!("PERF draws_2s_video_live_motion_calm={draws}");
    assert!(
        (11..=13).contains(&draws),
        "motion-loop blend cadence changed: {draws} draws in 2 s (base 12 ± 1)"
    );
}

/// Steady-state draws sampled per library size (the cold first draw is excluded).
const DRAW_SAMPLES: usize = 9;
/// Load-resilient bound for the 2,003-song vs 3-song draw ratio: HEAD is ≈1.0×, base ≈110×.
const MAX_LARGE_LIBRARY_DRAW_RATIO: f64 = 6.0;
/// Timed rounds of 1,000 font lookups; the median absorbs a load spike in any one round.
const FONT_ROUNDS: usize = 3;

fn draw_ms(r: &mut Rig) -> f64 {
    let t0 = Instant::now();
    let _ = r
        .app
        .update_window(r.operator, |_, window, _| window.refresh());
    r.app.run_until_parked();
    t0.elapsed().as_secs_f64() * 1000.0
}

fn median(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn ms_for_1000_calls(mut call: impl FnMut()) -> f64 {
    let t0 = Instant::now();
    for _ in 0..1000 {
        call();
    }
    t0.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn large_song_library_draw_cost_stays_flat() {
    let mut small = rig();
    let mut large = rig_with(RigOptions {
        extra_songs: 2000,
        ..RigOptions::default()
    });
    draw_ms(&mut small);
    draw_ms(&mut large);
    // Interleaved, so a burst of machine load lands on both libraries alike.
    let (mut small_samples, mut large_samples) = (Vec::new(), Vec::new());
    for _ in 0..DRAW_SAMPLES {
        small_samples.push(draw_ms(&mut small));
        large_samples.push(draw_ms(&mut large));
    }
    let (small_ms, large_ms) = (median(small_samples), median(large_samples));
    let small_songs = small.app.update(|cx| small.store.read(cx).songs.len());
    let large_songs = large.app.update(|cx| large.store.read(cx).songs.len());
    println!("PERF draw_ms_songs_{small_songs}={small_ms:.2}");
    println!("PERF draw_ms_songs_{large_songs}={large_ms:.2}");
    assert_eq!(large_songs, small_songs + 2000);
    assert!(
        large_ms <= small_ms * MAX_LARGE_LIBRARY_DRAW_RATIO,
        "draw with {large_songs} songs took {large_ms:.2} ms vs {small_ms:.2} ms with \
         {small_songs} (base ≈110×, limit {MAX_LARGE_LIBRARY_DRAW_RATIO}×)"
    );
}

#[test]
fn font_resolution_is_cheap_and_keeps_its_fallback_order() {
    let mut r = rig();
    r.app.update(|cx| {
        assert_eq!(resolve_font_family(FONT_BODY, cx).as_ref(), FONT_BODY);
        assert_eq!(
            resolve_font_family("No Such Family 1982", cx).as_ref(),
            FONT_BODY,
            "a missing user font must fall back to the bundled body font"
        );

        let (mut resolve_rounds, mut has_rounds) = (Vec::new(), Vec::new());
        for _ in 0..FONT_ROUNDS {
            resolve_rounds.push(ms_for_1000_calls(|| {
                let _ = resolve_font_family(FONT_BODY, cx);
            }));
            has_rounds.push(ms_for_1000_calls(|| {
                let _ = cx.text_system().has_font_family(FONT_BODY);
            }));
        }
        let (resolve_ms, has_ms) = (median(resolve_rounds), median(has_rounds));
        println!("PERF resolve_font_family_1000x_ms={resolve_ms:.3}");
        println!("PERF has_font_family_1000x_ms={has_ms:.3}");
        // resolve = at most two has_font_family lookups + one SharedString (base ≈35× with only
        // 8 headless fonts, because it collects, sorts and dedups every family per call).
        assert!(
            resolve_ms <= has_ms * 8.0 + 0.05,
            "resolve_font_family 1000× took {resolve_ms:.3} ms vs has_font_family {has_ms:.3} ms"
        );
    });
}

#[test]
fn rapid_slide_stepping_with_override_toggles_stays_consistent() {
    let mut r = rig();
    r.go_live_with_first_song();
    set_motion(&mut r, MotionMode::CrossfadeOnly);
    r.toggle_audience();
    r.toggle_stage();
    let start_index = r.live_index().expect("live deck");
    let overrides = [
        OutputOverride::Black,
        OutputOverride::Logo,
        OutputOverride::ClearText,
    ];
    let iterations = 300usize;
    let t0 = Instant::now();
    for i in 0..iterations {
        r.app.update(|cx| {
            r.store.update(cx, |s, cx| {
                // Stepping past a deck end returns Err by design.
                let _ = s.dispatch(PresentationCommand::NextLiveSlide, cx);
                let _ = s.dispatch(PresentationCommand::ToggleOverride(overrides[i % 3]), cx);
                let _ = s.dispatch(PresentationCommand::PrevLiveSlide, cx);
            })
        });
        r.app.run_until_parked();
    }
    let per_ms = t0.elapsed().as_secs_f64() * 1000.0 / iterations as f64;
    println!("PERF rapid_step_override_iter_ms={per_ms:.3}");

    assert_eq!(r.live_index(), Some(start_index));
    let (override_now, audience, stage) = r.app.update(|cx| {
        (
            r.store.read(cx).snapshot().output_override,
            windows_of::<AudienceOutputView>(cx).len(),
            windows_of::<StageMonitorView>(cx).len(),
        )
    });
    // Overrides are exclusive: the last toggle (ClearText over Logo) leaves ClearText latched.
    assert_eq!(override_now, OutputOverride::ClearText);
    assert_eq!(
        (audience, stage),
        (1, 1),
        "output windows must survive the stress"
    );
}

#[test]
fn slide_dispatch_throughput() {
    let mut r = rig();
    r.go_live_with_first_song();
    let start_index = r.live_index();
    let steps = 10_000u32;
    let per_us = r.app.update(|cx| {
        r.store.update(cx, |s, cx| {
            let t0 = Instant::now();
            for i in 0..steps {
                let command = if i % 2 == 0 {
                    PresentationCommand::NextLiveSlide
                } else {
                    PresentationCommand::PrevLiveSlide
                };
                let _ = s.dispatch(command, cx);
            }
            t0.elapsed().as_secs_f64() * 1e6 / f64::from(steps)
        })
    });
    println!("PERF dispatch_step_us={per_us:.3}");
    assert_eq!(r.live_index(), start_index);
}

#[test]
fn bootstrap_with_large_library_loads_every_song() {
    let mut db = Database::open_in_memory().expect("in-memory sqlite");
    sundaydeck_db::seed::seed_defaults_if_empty(db.conn_mut()).expect("seed defaults");
    let seeded = sundaydeck_db::worker::load_bootstrap_sync(&db).expect("bootstrap");
    let template = seeded.loaded_songs.first().cloned().expect("seeded song");
    for i in 0..2000 {
        let mut song = template.clone();
        song.id = SongId::new_v7();
        song.title = format!("Library Hymn {i:05}");
        for section in &mut song.sections {
            let old = section.id;
            section.id = sundaydeck_core::SectionId::new_v7();
            for arrangement in &mut song.arrangements {
                for id in &mut arrangement.section_ids {
                    if *id == old {
                        *id = section.id;
                    }
                }
            }
        }
        for arrangement in &mut song.arrangements {
            arrangement.id = sundaydeck_core::ArrangementId::new_v7();
        }
        song.default_arrangement_id = song.arrangements.first().map(|a| a.id);
        sundaydeck_db::song_repo::upsert_song(db.conn_mut(), &song).expect("insert song");
    }
    let t0 = Instant::now();
    let boot = sundaydeck_db::worker::load_bootstrap_sync(&db).expect("bootstrap");
    println!(
        "PERF bootstrap_2000_songs_ms={:.1}",
        t0.elapsed().as_secs_f64() * 1000.0
    );
    assert_eq!(boot.loaded_songs.len(), seeded.loaded_songs.len() + 2000);
}

/// Nothing listens on the discard port, so the voice worker's Whisper probe fails fast offline.
const OFFLINE_ASR_URL: &str = "http://127.0.0.1:9/inference";
/// Real time for the voice worker thread (75 ms command loop) to handle queued commands.
const VOICE_WORKER_SETTLE: Duration = Duration::from_millis(400);
/// A sung line with no Bible reference, so no scripture event rides along with the telemetry.
const SUNG_LINE: &str = "amazing grace how sweet the sound";
/// Raw mic levels that all show two meter bars: only the undisplayed `mic_level` differs.
const TWO_BAR_MIC_LEVELS: [f32; 5] = [0.31, 0.32, 0.33, 0.34, 0.35];

/// Lets the real voice worker handle its queued commands, then runs one simulated second of
/// heartbeat ticks, each of which drains the worker's events into the store.
fn settle_voice_worker(r: &mut Rig) {
    std::thread::sleep(VOICE_WORKER_SETTLE);
    r.draws_over_secs(1);
}

fn inject_sung_line(commands: &VoiceEngineHandle, mic_level: f32) {
    commands
        .send(VoiceCommand::InjectTranscript {
            text: SUNG_LINE.to_owned(),
            mic_level,
        })
        .expect("worker accepts the transcript");
}

#[test]
fn voice_telemetry_that_changes_only_the_mic_level_never_notifies_the_store() {
    // Simulated clock: the heartbeat itself stays quiet, so any notify comes from voice events.
    let mut r = rig_with(RigOptions {
        simulated_clock: true,
        ..RigOptions::default()
    });
    let engine = VoiceEngineHandle::spawn(VoiceConfig {
        asr_endpoint_url: OFFLINE_ASR_URL.to_owned(),
        ..VoiceConfig::default()
    });
    let commands = engine.clone();
    r.app
        .update(|cx| r.store.update(cx, |s, _| s.attach_voice_engine(engine)));
    // Startup telemetry and the first transcript (text, meter, status) are visible changes.
    inject_sung_line(&commands, 0.30);
    settle_voice_worker(&mut r);

    let (notifies, _sub) = r.count_store_notifies();
    // The same line again at other raw levels: every telemetry event differs only in mic_level.
    for mic_level in TWO_BAR_MIC_LEVELS {
        inject_sung_line(&commands, mic_level);
    }
    settle_voice_worker(&mut r);
    assert_eq!(
        notifies.get(),
        0,
        "telemetry that changes only the raw mic level must not notify the store (AC1.6)"
    );
    let last_level = r
        .app
        .update(|cx| r.store.read(cx).voice_telemetry.mic_level);
    assert!(
        (last_level - 0.35).abs() < f32::EPSILON,
        "the mic-level-only events were delivered and applied silently (got {last_level})"
    );

    // The same pipe still delivers a visible change (no microphone is opened by rehearsal).
    commands
        .send(VoiceCommand::StartRehearsalDemo)
        .expect("worker accepts the rehearsal");
    settle_voice_worker(&mut r);
    assert!(
        notifies.get() >= 1,
        "a visible telemetry change must still reach the store"
    );
    commands
        .send(VoiceCommand::Shutdown)
        .expect("worker accepts shutdown");
}
