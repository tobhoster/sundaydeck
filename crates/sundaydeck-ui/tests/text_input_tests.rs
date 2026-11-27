//! Keyboard routing and text-editing regressions (plan C7, AC1.8 song search, UTF-8 stress).
mod support;

use std::time::Duration;

use gpui::{
    ClipboardItem, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, PlatformInput, Point, px,
};
use sundaydeck_core::{BibleVerse, OutputOverride, ScriptureRef};
use sundaydeck_ui::components::TextInput;
use sundaydeck_ui::store::BibleBrowseMode;
use sundaydeck_ui::views::AudienceOutputView;
use sundaydeck_ui::views::operator_workspace::LibraryTab;
use support::{Rig, rig, windows_of};

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

#[test]
fn wireless_powerpoint_clicker_works_across_windows_and_never_fires_in_text_inputs() {
    let mut r = rig();
    r.go_live_with_first_song();
    let operator = r.operator;
    let workspace = r.workspace.clone();
    r.focus(operator, &workspace);

    // 1. Standard Logitech / PowerPoint clicker keys work in OperatorWorkspace.
    r.press(operator, "pagedown");
    assert_eq!(r.live_index(), Some(1), "PageDown advances slide");
    r.press(operator, "pageup");
    assert_eq!(r.live_index(), Some(0), "PageUp steps back");
    r.press(operator, ".");
    assert_eq!(
        r.app
            .update(|cx| r.store.read(cx).snapshot().output_override),
        OutputOverride::Black,
        "`.` clicker blackout button toggles Black Screen"
    );
    r.press(operator, "b");
    assert_eq!(
        r.app
            .update(|cx| r.store.read(cx).snapshot().output_override),
        OutputOverride::None,
        "`b` clicker blackout button clears Black Screen"
    );

    // 2. Typing clicker letters (`b`, `.`, `w`, `,`) inside a TextInput NEVER triggers overrides.
    let input = focus_song_search(&mut r);
    for key in ["b", ".", "w", ","] {
        r.press(operator, key);
        assert_eq!(
            r.app
                .update(|cx| r.store.read(cx).snapshot().output_override),
            OutputOverride::None,
            "`{key}` typed in a TextInput must never trigger an override"
        );
    }
    assert_eq!(input_value(&mut r, &input), "b.w,");

    // 3. Clicker keeps working even when the Audience Projector window has OS focus.
    r.toggle_audience();
    let aud_win = r.app.update(|cx| windows_of::<AudienceOutputView>(cx)[0]);
    r.press(aud_win, "pagedown");
    assert_eq!(
        r.live_index(),
        Some(1),
        "PageDown from a focused Audience window must still advance slides"
    );
    r.press(aud_win, ".");
    assert_eq!(
        r.app
            .update(|cx| r.store.read(cx).snapshot().output_override),
        OutputOverride::Black,
        "`.` blackout button from a focused Audience window must toggle Black Screen"
    );
    r.toggle_audience();
}

#[test]
fn bible_verse_double_click_goes_live_and_shift_click_selects_range() {
    let mut r = rig();
    let operator = r.operator;
    let verses = vec![
        BibleVerse {
            translation_code: "BSB".to_owned(),
            book_number: 43,
            book_name: "John".to_owned(),
            chapter: 3,
            verse: 16,
            text: "For God so loved the world that He gave His one and only Son.".to_owned(),
        },
        BibleVerse {
            translation_code: "BSB".to_owned(),
            book_number: 43,
            book_name: "John".to_owned(),
            chapter: 3,
            verse: 17,
            text: "For God did not send His Son into the world to condemn the world.".to_owned(),
        },
    ];
    r.app.update(|cx| {
        r.store.update(cx, |s, cx| {
            s.active_translation_code = "BSB".to_owned();
            s.bible_browse_mode = BibleBrowseMode::Chapter {
                book_number: 43,
                chapter: 3,
            };
            s.bible_verses = verses;
            s.selected_verse_range = None;
            cx.notify();
        });
        r.workspace.update(cx, |ws, cx| {
            ws.switch_library_tab(LibraryTab::Bible, cx);
        });
    });

    let dispatch_click = |r: &mut Rig, x: f32, y: f32, click_count: usize, shift: bool| {
        let _ = r.app.update_window(operator, |_, w, cx| {
            w.refresh();
            w.draw(cx).clear(cx);
        });
        r.app.run_until_parked();
        let pos = Point::new(px(x), px(y));
        let mods = Modifiers {
            shift,
            ..Modifiers::default()
        };
        let _ = r.app.update_window(operator, |_, w, cx| {
            w.dispatch_event(
                PlatformInput::MouseDown(MouseDownEvent {
                    button: MouseButton::Left,
                    position: pos,
                    modifiers: mods,
                    click_count,
                    first_mouse: false,
                }),
                cx,
            );
            w.dispatch_event(
                PlatformInput::MouseUp(MouseUpEvent {
                    button: MouseButton::Left,
                    position: pos,
                    modifiers: mods,
                    click_count,
                }),
                cx,
            );
            w.refresh();
            w.draw(cx).clear(cx);
        });
        r.app.run_until_parked();
    };

    // Single-clicking verse 0 stages preview safely without going live.
    dispatch_click(&mut r, 450.0, 760.0, 1, false);
    assert_eq!(
        r.app.update(|cx| r.store.read(cx).selected_verse_range),
        Some((0, 0))
    );
    assert!(
        r.app
            .update(|cx| r.store.read(cx).snapshot().live.is_none()),
        "single-clicking a Bible verse must stay in safe preview"
    );

    // Shift-clicking verse 1 extends the selected passage to verses 0..=1 without going live.
    dispatch_click(&mut r, 450.0, 800.0, 1, true);
    assert_eq!(
        r.app.update(|cx| r.store.read(cx).selected_verse_range),
        Some((0, 1)),
        "Shift-click must extend Bible verse selection range"
    );
    assert!(
        r.app
            .update(|cx| r.store.read(cx).snapshot().live.is_none()),
        "Shift-click must stay in preview until double-clicked or Show Live"
    );

    // Double-clicking verse 0 immediately projects it Live on screen.
    dispatch_click(&mut r, 450.0, 760.0, 2, false);
    let live_title = r.app.update(|cx| {
        r.store
            .read(cx)
            .snapshot()
            .live
            .as_ref()
            .map(|l| l.title.clone())
    });
    assert!(
        live_title
            .as_deref()
            .is_some_and(|t| t.contains("John 3:16")),
        "double-clicking a Bible verse must project it live immediately, got {live_title:?}"
    );

    // Double-clicking a queued spoken Scripture (go_live=true) also projects it straight live.
    let ps23 = ScriptureRef {
        book_number: 19,
        chapter: 23,
        verse_start: Some(1),
        verse_end: Some(1),
    };
    r.app.update(|cx| {
        r.workspace
            .update(cx, |ws, cx| ws.stage_queued_scripture(ps23, true, cx));
    });
    settle_song_search(&mut r);
    let queued_live = r.app.update(|cx| {
        r.store
            .read(cx)
            .snapshot()
            .live
            .as_ref()
            .map(|l| l.title.clone())
    });
    assert!(
        queued_live
            .as_deref()
            .is_some_and(|t| t.contains("Psalm") || t.contains("23")),
        "queued scripture double-click must project live, got {queued_live:?}"
    );
}
