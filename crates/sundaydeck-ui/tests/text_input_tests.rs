//! Keyboard routing and text-editing regressions (plan C7, AC1.8 song search, UTF-8 stress).
mod support;

use std::time::Duration;

use gpui::ClipboardItem;
use sundaydeck_ui::components::TextInput;
use support::{Rig, rig};

fn focus_song_search(r: &mut Rig) -> gpui::Entity<TextInput> {
    let input = r
        .app
        .update(|cx| r.workspace.read(cx).song_search_input.clone());
    let operator = r.operator;
    r.focus(operator, &input);
    input
}

fn input_value(r: &mut Rig, input: &gpui::Entity<TextInput>) -> String {
    r.app.update(|cx| input.read(cx).value().to_owned())
}

#[test]
fn navigation_keys_typed_into_a_text_input_never_step_the_live_projector() {
    let mut r = rig();
    r.go_live_with_first_song();
    r.dispatch(sundaydeck_core::PresentationCommand::NextLiveSlide);
    let live_before = r.live_index();
    assert_eq!(
        live_before,
        Some(1),
        "fixture: start mid-deck so both directions show"
    );

    let input = focus_song_search(&mut r);
    let operator = r.operator;
    for key in [
        "a", "space", "b", "right", "down", "pagedown", "left", "up", "pageup",
    ] {
        r.press(operator, key);
        assert_eq!(
            r.live_index(),
            live_before,
            "`{key}` typed into the song search moved the live slide"
        );
    }
    assert_eq!(input_value(&mut r, &input), "a b");
}

#[test]
fn utf8_editing_keeps_text_and_caret_on_char_boundaries() {
    let mut r = rig();
    let input = focus_song_search(&mut r);
    let operator = r.operator;

    for text in ["é", "ñ", "ã", "🙏", "e", "\u{301}"] {
        r.type_char(operator, text);
    }
    assert_eq!(input_value(&mut r, &input), "éñã🙏e\u{301}");

    // Each step: keys pressed, expected value afterwards.
    let steps: [(&[&str], &str); 6] = [
        (&["backspace"], "éñã🙏e"),
        (&["left", "left", "delete"], "éñãe"),
        (&["home", "delete"], "ñãe"),
        (&["end", "backspace"], "ñã"),
        (&["right", "left"], "ñã"),
        (&["home", "right"], "ñã"),
    ];
    for (keys, expected) in steps {
        for key in keys {
            r.press(operator, key);
        }
        assert_eq!(input_value(&mut r, &input), expected, "after {keys:?}");
    }
    r.type_char(operator, "x");
    assert_eq!(input_value(&mut r, &input), "ñxã", "caret sits after ñ");

    r.app.update(|cx| {
        cx.write_to_clipboard(ClipboardItem::new_string("Señor\nGraça 🙏".to_owned()))
    });
    r.press(operator, "ctrl-v");
    assert_eq!(
        input_value(&mut r, &input),
        "ñxSeñor Graça 🙏ã",
        "a multi-line paste into a single-line input is flattened at the caret"
    );
    // Caret sits before `ã`: delete the `x` and the whole paste, char by char.
    for _ in 0.."xSeñor Graça 🙏".chars().count() {
        r.press(operator, "backspace");
    }
    assert_eq!(input_value(&mut r, &input), "ñã");
}

/// Lets the debounce timer fire and the DB worker thread reply.
fn settle_song_search(r: &mut Rig) {
    r.app.allow_parking();
    for _ in 0..10 {
        r.app.advance_clock(Duration::from_millis(100));
        r.app.run_until_parked();
        std::thread::sleep(Duration::from_millis(20));
        r.app.run_until_parked();
    }
}

fn song_filter(r: &mut Rig) -> (String, Option<usize>) {
    r.app.update(|cx| {
        let s = r.store.read(cx);
        (
            s.song_search_query.clone(),
            s.filtered_song_ids().map(|ids| ids.len()),
        )
    })
}

#[test]
fn clearing_the_song_search_is_not_overwritten_by_a_late_reply() {
    let mut r = rig();
    focus_song_search(&mut r);
    let operator = r.operator;
    r.press(operator, "a");
    r.press(operator, "backspace");
    settle_song_search(&mut r);
    assert_eq!(
        song_filter(&mut r),
        (String::new(), None),
        "the stale reply for `a` re-applied a filter after the box was cleared"
    );
}

#[test]
fn song_search_applies_only_the_latest_query() {
    let mut r = rig();
    focus_song_search(&mut r);
    let operator = r.operator;
    for key in ["g", "r", "a", "c", "e"] {
        r.press(operator, key);
    }
    settle_song_search(&mut r);
    let (query, hits) = song_filter(&mut r);
    assert_eq!(query, "grace");
    assert!(
        hits.is_some_and(|n| n >= 1),
        "`grace` must match the seeded Amazing Grace, got {hits:?}"
    );
}
