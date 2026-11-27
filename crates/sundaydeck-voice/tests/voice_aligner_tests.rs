use std::sync::Arc;
use std::time::Duration;

use sundaydeck_core::defaults::{sample_amazing_grace, sample_it_is_well};
use sundaydeck_core::{Slide, reflow_song};
use sundaydeck_voice::{
    DeckVoiceAligner, VoiceCommand, VoiceConfig, VoiceDecision, VoiceEngineHandle, VoiceEvent,
    VoiceSyncMode, encode_wav_pcm16,
};

#[test]
fn sequential_singing_with_asr_typos_tracks_progress_and_advances() {
    let song = sample_amazing_grace();
    let slides = reflow_song(&song, song.default_arrangement_id, None);
    let mut aligner = DeckVoiceAligner::new();
    aligner.update_deck(&slides, 0, 1_000);

    let cfg = VoiceConfig {
        mode: VoiceSyncMode::Auto,
        cooldown_ms: 1_500,
        ..VoiceConfig::default()
    };

    // First half of Verse 1 with realistic church ASR drift ("amazin", "retch")
    let u1 = aligner.ingest_transcript(
        "oh amazin grace how sweet the sound that saved a retch like me",
        3_000,
        &cfg,
    );
    assert!(
        u1.slide_progress > 0.35 && u1.slide_progress < 0.72,
        "expected mid-slide progress, got {}",
        u1.slide_progress
    );
    assert_eq!(u1.decision, VoiceDecision::Hold);

    // End of Verse 1 triggers smooth auto-advance to Verse 2
    let u2 = aligner.ingest_transcript(
        "i once was lost but now am found was blind but now i see",
        5_000,
        &cfg,
    );
    assert!(
        matches!(
            u2.decision,
            VoiceDecision::Advance {
                target_index: 1,
                ..
            }
        ),
        "expected auto-advance to Verse 2, got {:?}",
        u2.decision
    );
    assert_eq!(aligner.active_index(), 1);

    // Cooldown prevents accidental double-advance immediately after transitioning
    let u3 = aligner.ingest_transcript("was blind but now i see", 5_400, &cfg);
    assert_eq!(u3.decision, VoiceDecision::Hold);
}

#[test]
fn suggest_mode_never_auto_advances_and_override_lock_blocks_advances() {
    let song = sample_amazing_grace();
    let slides = reflow_song(&song, song.default_arrangement_id, None);
    let mut aligner = DeckVoiceAligner::new();
    aligner.update_deck(&slides, 0, 1_000);

    let suggest_cfg = VoiceConfig {
        mode: VoiceSyncMode::Suggest,
        ..VoiceConfig::default()
    };

    let _ = aligner.ingest_transcript(
        "amazing grace how sweet the sound that saved a wretch like me",
        3_000,
        &suggest_cfg,
    );
    let u = aligner.ingest_transcript(
        "i once was lost but now am found was blind but now i see",
        5_000,
        &suggest_cfg,
    );
    assert!(
        matches!(
            u.decision,
            VoiceDecision::Suggest {
                target_index: 1,
                ..
            }
        ),
        "Suggest mode must emit Suggest, never Advance: {:?}",
        u.decision
    );
    assert_eq!(aligner.active_index(), 0);

    // Lock during Black Screen override suppresses Auto-Advance completely
    let auto_cfg = VoiceConfig {
        mode: VoiceSyncMode::Auto,
        lock_during_override: true,
        ..VoiceConfig::default()
    };
    aligner.set_override_locked(true);
    let locked =
        aligner.ingest_transcript("twas grace that taught my heart to fear", 8_000, &auto_cfg);
    assert_eq!(locked.decision, VoiceDecision::Hold);
    assert_eq!(aligner.active_index(), 0);
}

#[test]
fn spontaneous_chorus_repeat_is_detected_out_of_sequence() {
    let song = sample_it_is_well();
    // Arrangement: [Verse 1 (0), Chorus (1), Verse 2 (2), Chorus (3)]
    let slides = reflow_song(&song, song.default_arrangement_id, None);
    let mut aligner = DeckVoiceAligner::new();
    aligner.update_deck(&slides, 2, 1_000); // Currently on Verse 2

    let cfg = VoiceConfig {
        mode: VoiceSyncMode::Auto,
        ..VoiceConfig::default()
    };

    // Unrelated congregation chatter must never advance
    let noise = aligner.ingest_transcript(
        "welcome everybody thank you so much for being here",
        3_500,
        &cfg,
    );
    assert_eq!(noise.decision, VoiceDecision::Hold);

    // Worship leader jumps spontaneously into the Chorus
    let jump = aligner.ingest_transcript(
        "it is well with my soul it is well it is well with my soul",
        6_000,
        &cfg,
    );
    assert!(
        matches!(
            jump.decision,
            VoiceDecision::Advance {
                target_index: 1 | 3,
                ..
            }
        ),
        "expected jump to Chorus slide (1 or 3), got {:?}",
        jump.decision
    );
}

#[test]
fn wav_encoder_emits_valid_riff_header_and_worker_thread_advances_live() {
    let wav = encode_wav_pcm16(&[0i16, 1200, -1200, 500], 16_000);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(wav.len(), 44 + 8);

    let song = sample_amazing_grace();
    let slides: Arc<[Slide]> = Arc::from(reflow_song(&song, song.default_arrangement_id, None));
    let handle = VoiceEngineHandle::spawn(VoiceConfig::default());
    handle
        .send(VoiceCommand::UpdateLiveDeck {
            slides,
            active_index: 0,
        })
        .expect("worker accepts the deck");
    handle
        .send(VoiceCommand::SetMode(VoiceSyncMode::Auto))
        .expect("worker accepts the mode");
    std::thread::sleep(Duration::from_millis(60));
    handle
        .send(VoiceCommand::InjectTranscript {
            text: "twas grace that taught my heart to fear and grace my fears relieved".to_owned(),
            mic_level: 0.72,
        })
        .expect("worker accepts the transcript");

    // Poll briefly for the background worker to emit live telemetry with non-zero mic bars.
    let mut events = Vec::new();
    for _ in 0..75 {
        events.extend(handle.drain_events());
        if events
            .iter()
            .any(|e| matches!(e, VoiceEvent::Telemetry(t) if t.meter_bars >= 2))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        events
            .iter()
            .any(|e| matches!(e, VoiceEvent::Telemetry(t) if t.meter_bars >= 2)),
        "expected live telemetry from background voice worker, got {events:?}"
    );
    handle
        .send(VoiceCommand::Shutdown)
        .expect("worker accepts shutdown");
}
