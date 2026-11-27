//! Pure workspace/store state regressions: clamped drag-resize reports no change (plan S9,
//! AC1.8) and an empty bootstrap chapter starts with no verse selection (plan C3).
mod support;

use sundaydeck_ui::store::PresentationStore;
use sundaydeck_ui::views::layout_handles::{
    DEFAULT_LEFT_WIDTH, DEFAULT_LIBRARY_HEIGHT, DEFAULT_RIGHT_WIDTH, DragResizeAxis,
    MAX_LEFT_WIDTH, MAX_LIBRARY_HEIGHT, MAX_RIGHT_WIDTH, MIN_LEFT_WIDTH, MIN_LIBRARY_HEIGHT,
    MIN_RIGHT_WIDTH, WorkspaceLayout,
};

/// Mouse delta along the dragged axis that grows the pane (the right deck and the library split
/// grow when the pointer moves left/up).
fn growth_sign(axis: DragResizeAxis) -> f32 {
    match axis {
        DragResizeAxis::LeftRail => 1.0,
        DragResizeAxis::RightDeck | DragResizeAxis::LibrarySplit => -1.0,
    }
}

fn pane_size(layout: &WorkspaceLayout, axis: DragResizeAxis) -> f32 {
    match axis {
        DragResizeAxis::LeftRail => layout.left_width,
        DragResizeAxis::RightDeck => layout.right_width,
        DragResizeAxis::LibrarySplit => layout.library_height,
    }
}

fn move_mouse(layout: &mut WorkspaceLayout, axis: DragResizeAxis, delta: f32) -> bool {
    const ORIGIN: f32 = 1_000.0;
    let along = ORIGIN + growth_sign(axis) * delta;
    match axis {
        DragResizeAxis::LibrarySplit => layout.apply_drag(ORIGIN, along),
        DragResizeAxis::LeftRail | DragResizeAxis::RightDeck => layout.apply_drag(along, ORIGIN),
    }
}

#[test]
fn drag_resize_reports_a_change_only_when_the_pane_size_changes() {
    let cases = [
        (
            DragResizeAxis::LeftRail,
            DEFAULT_LEFT_WIDTH,
            MIN_LEFT_WIDTH,
            MAX_LEFT_WIDTH,
        ),
        (
            DragResizeAxis::RightDeck,
            DEFAULT_RIGHT_WIDTH,
            MIN_RIGHT_WIDTH,
            MAX_RIGHT_WIDTH,
        ),
        (
            DragResizeAxis::LibrarySplit,
            DEFAULT_LIBRARY_HEIGHT,
            MIN_LIBRARY_HEIGHT,
            MAX_LIBRARY_HEIGHT,
        ),
    ];
    for (axis, default, min, max) in cases {
        let mut layout = WorkspaceLayout::default();
        assert!(
            !layout.apply_drag(5_000.0, 5_000.0),
            "{axis:?}: a mouse move without an active drag must not redraw"
        );
        layout.start_drag(axis, 1_000.0, 1_000.0);

        // (growth delta, expected redraw, expected size afterwards)
        let moves = [
            (10_000.0, true, max),
            (20_000.0, false, max),
            (10_001.0, false, max),
            (-10_000.0, true, min),
            (-20_000.0, false, min),
            (0.0, true, default),
            (0.0, false, default),
        ];
        for (delta, redraw, size) in moves {
            assert_eq!(
                move_mouse(&mut layout, axis, delta),
                redraw,
                "{axis:?}: move by {delta} reported the wrong change flag"
            );
            assert_eq!(pane_size(&layout, axis), size, "{axis:?}: move by {delta}");
        }
        assert!(layout.end_drag());
    }
}

#[test]
fn verse_selection_starts_empty_when_the_bootstrap_chapter_has_no_verses() {
    let seeded = support::seeded_bootstrap();
    assert!(
        !seeded.initial_chapter_verses.is_empty(),
        "fixture: the seeded translation ships Psalm 23"
    );
    let with_verses = PresentationStore::from_bootstrap(seeded);
    assert_eq!(with_verses.selected_verse_range, Some((0, 0)));

    let mut empty = support::seeded_bootstrap();
    empty.initial_chapter_verses.clear();
    let store = PresentationStore::from_bootstrap(empty);
    assert!(store.bible_verses.is_empty());
    assert_eq!(
        store.selected_verse_range, None,
        "a (0, 0) selection over zero verses makes `Add next verse` index out of bounds"
    );
}
