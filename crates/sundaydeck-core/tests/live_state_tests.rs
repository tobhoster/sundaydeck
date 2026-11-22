use std::sync::Arc;

use sundaydeck_core::defaults::{default_themes, sample_amazing_grace, sample_come_thou_fount};
use sundaydeck_core::{
    AudienceRenderState, DeckCursor, OutputOverride, PresentationCommand, PresentationEngine,
    PresentationError, TransitionKind, reflow_song,
};

fn deck_for_grace() -> DeckCursor {
    let song = sample_amazing_grace();
    let theme = Arc::new(default_themes().remove(0));
    DeckCursor::try_new(
        None,
        song.title.clone(),
        reflow_song(&song, None, None),
        theme,
    )
    .expect("sample deck must not be empty")
}

fn deck_for_fount() -> DeckCursor {
    let song = sample_come_thou_fount();
    let theme = Arc::new(default_themes().remove(0));
    DeckCursor::try_new(
        None,
        song.title.clone(),
        reflow_song(&song, None, None),
        theme,
    )
    .expect("sample deck must not be empty")
}

#[test]
fn preview_changes_do_not_interrupt_live_output() {
    let mut engine = PresentationEngine::new(None, "Grace Fellowship");
    engine
        .dispatch(PresentationCommand::StagePreview(deck_for_grace()))
        .expect("stage grace");
    engine
        .dispatch(PresentationCommand::GoLive)
        .expect("go live");

    let live_before = engine.snapshot().audience.clone();
    engine
        .dispatch(PresentationCommand::StagePreview(deck_for_fount()))
        .expect("stage fount in preview");
    engine
        .dispatch(PresentationCommand::SelectPreviewSlide(1))
        .expect("browse preview slides");

    assert_eq!(engine.snapshot().audience, live_before);
    assert_eq!(
        engine.snapshot().preview.as_ref().map(|p| p.active_index),
        Some(1)
    );
}

#[test]
fn override_precedence_and_black_stickiness_on_go_live() {
    let mut engine = PresentationEngine::new(None, "Grace Fellowship");
    engine
        .dispatch(PresentationCommand::GoLiveDirect(deck_for_grace()))
        .expect("go live direct");

    engine
        .dispatch(PresentationCommand::ToggleOverride(
            OutputOverride::ClearText,
        ))
        .expect("clear text");
    assert!(matches!(
        engine.snapshot().audience,
        AudienceRenderState::Slide {
            show_text: false,
            ..
        }
    ));

    engine
        .dispatch(PresentationCommand::StagePreview(deck_for_fount()))
        .expect("stage fount");
    engine
        .dispatch(PresentationCommand::GoLive)
        .expect("go live releases clear text");
    assert_eq!(engine.snapshot().output_override, OutputOverride::None);
    assert!(matches!(
        engine.snapshot().audience,
        AudienceRenderState::Slide {
            show_text: true,
            ..
        }
    ));

    engine
        .dispatch(PresentationCommand::ToggleOverride(OutputOverride::Black))
        .expect("engage black");
    engine
        .dispatch(PresentationCommand::StagePreview(deck_for_grace()))
        .expect("stage grace during black");
    engine
        .dispatch(PresentationCommand::GoLive)
        .expect("go live during black");
    assert_eq!(engine.snapshot().output_override, OutputOverride::Black);
    assert!(matches!(
        engine.snapshot().audience,
        AudienceRenderState::Black { .. }
    ));
}

#[test]
fn live_text_patch_updates_in_place_without_crossfade() {
    let mut engine = PresentationEngine::new(None, "Grace Fellowship");
    engine
        .dispatch(PresentationCommand::GoLiveDirect(deck_for_grace()))
        .expect("go live");
    let slide_id = engine
        .snapshot()
        .live
        .as_ref()
        .and_then(|c| c.active_slide())
        .expect("active slide")
        .id;

    let transition = engine
        .dispatch(PresentationCommand::PatchLiveSlideText {
            slide_id,
            new_body: "Amazing grace! How sweet the sound\nCorrected live.".to_owned(),
        })
        .expect("patch must succeed");

    assert_eq!(transition.kind, TransitionKind::LiveInPlacePatch);
    match &transition.snapshot.audience {
        AudienceRenderState::Slide { slide, .. } => {
            assert!(slide.content.primary_text().contains("Corrected live."));
        }
        other => panic!("expected live slide, got {other:?}"),
    }
}

#[test]
fn stage_message_never_appears_on_audience_state_and_errors_are_typed() {
    let mut engine = PresentationEngine::new(None, "Grace Fellowship");
    assert_eq!(
        engine.dispatch(PresentationCommand::GoLive).unwrap_err(),
        PresentationError::NothingStagedInPreview
    );

    engine
        .dispatch(PresentationCommand::GoLiveDirect(deck_for_grace()))
        .expect("go live");
    assert!(matches!(
        engine
            .dispatch(PresentationCommand::SelectLiveSlide(99))
            .unwrap_err(),
        PresentationError::SlideOutOfBounds { index: 99, len: 4 }
    ));

    let tr = engine
        .dispatch(PresentationCommand::SetStageMessage(Some(
            "Wrap up in 2 mins".to_owned(),
        )))
        .expect("stage message");
    assert_eq!(tr.kind, TransitionKind::StageOnly);
    assert_eq!(
        tr.snapshot.stage.stage_message.as_deref(),
        Some("Wrap up in 2 mins")
    );
}
