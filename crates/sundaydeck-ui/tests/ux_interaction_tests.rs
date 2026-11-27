//! Desktop UX interaction tests and v0.0.3 visual review capture:
//! - Click-outside modal & header popover dismissal
//! - Escape (`DismissOverlay`) priority order without quitting
//! - Right-click context menus for hymns, schedule items, preview slides, and Bible verses
//! - 1-click Quick Slide presets (Pastor Quote, Member Birthday, Sermon Points, Announcement)
//! - Day-1 rich offline content invariants (19 hymns, 3 starter Bibles, 12 studio themes, 9 free catalog Bibles)
mod support;

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    AnyWindowHandle, AppContext as _, Modifiers, MouseButton, MouseDownEvent, MouseUpEvent,
    PlatformInput, Point, px, size,
};
use gpui_screenshot::ScreenshotApp;
use sundaydeck_core::{
    CustomSlideLayout, PresentationCommand, ServiceItemPayload, parse_scripture_ref,
};
use sundaydeck_db::{Database, DbWorker};
use sundaydeck_ui::content_downloader::FREE_BIBLE_CATALOG;
use sundaydeck_ui::design::fonts::PROJECTION_FONTS;
use sundaydeck_ui::design::{DeckAssetSource, register_fonts};
use sundaydeck_ui::store::{BibleBrowseMode, PresentationStore};
use sundaydeck_ui::views::context_menu::ContextMenuTarget;
use sundaydeck_ui::views::operator_workspace::{ActiveModal, HeaderPopover, LibraryTab};
use sundaydeck_ui::views::quick_slides::{QuickSlidePreset, quick_slide_preset_spec};
use sundaydeck_ui::views::settings_modal::SettingsTab;
use sundaydeck_ui::views::{AudienceOutputView, OperatorWorkspace};
use sundaydeck_ui::windows::WindowManager;
use support::{Rig, rig, seeded_bootstrap};

fn redraw_window(r: &mut Rig, window: AnyWindowHandle) {
    let _ = r.app.update_window(window, |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    r.app.run_until_parked();
}

fn click_at(r: &mut Rig, window: AnyWindowHandle, x: f32, y: f32, button: MouseButton) {
    redraw_window(r, window);
    let pos = Point::new(px(x), px(y));
    let _ = r.app.update_window(window, |_, window, cx| {
        window.dispatch_event(
            PlatformInput::MouseDown(MouseDownEvent {
                button,
                position: pos,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            PlatformInput::MouseUp(MouseUpEvent {
                button,
                position: pos,
                modifiers: Modifiers::default(),
                click_count: 1,
            }),
            cx,
        );
    });
    redraw_window(r, window);
}

#[test]
fn clicking_modal_backdrop_closes_modal_while_clicking_inside_keeps_it_open() {
    let mut r = rig();
    let operator = r.operator;
    r.app.update(|cx| {
        r.workspace.update(cx, |ws, cx| {
            ws.open_settings_on_tab(SettingsTab::BiblesBundles, cx);
        });
    });
    redraw_window(&mut r, operator);

    assert!(
        r.app
            .update(|cx| r.workspace.read(cx).active_modal.is_some()),
        "settings modal should be open"
    );

    // Clicking inside the centered modal surface must NOT dismiss it.
    click_at(&mut r, operator, 720.0, 450.0, MouseButton::Left);
    assert!(
        r.app
            .update(|cx| r.workspace.read(cx).active_modal.is_some()),
        "clicking inside the modal must keep it open"
    );

    // Clicking the dark backdrop outside the modal must dismiss it immediately.
    click_at(&mut r, operator, 20.0, 20.0, MouseButton::Left);
    assert!(
        r.app
            .update(|cx| r.workspace.read(cx).active_modal.is_none()),
        "clicking outside the modal on its backdrop must close it"
    );
}

#[test]
fn clicking_outside_popover_closes_it_and_escape_dismisses_in_order() {
    let mut r = rig();
    let operator = r.operator;
    let workspace = r.workspace.clone();
    r.focus(operator, &workspace);

    r.app.update(|cx| {
        r.workspace.update(cx, |ws, cx| {
            ws.toggle_popover(HeaderPopover::AutoFollow, cx);
        });
    });
    redraw_window(&mut r, operator);
    assert_eq!(
        r.app.update(|cx| r.workspace.read(cx).active_popover),
        Some(HeaderPopover::AutoFollow)
    );

    // Clicking outside the popover closes it.
    click_at(&mut r, operator, 30.0, 400.0, MouseButton::Left);
    assert_eq!(
        r.app.update(|cx| r.workspace.read(cx).active_popover),
        None,
        "clicking outside the popover must dismiss it"
    );

    // Open a modal, then a context menu on top; Escape closes context menu first, then modal.
    let first_song_id = r.app.update(|cx| r.store.read(cx).songs[0].id);
    r.app.update(|cx| {
        r.workspace.update(cx, |ws, cx| {
            ws.open_quick_slide_preset(QuickSlidePreset::PastorQuote, cx);
            ws.open_context_menu(
                Point::new(px(280.0), px(220.0)),
                ContextMenuTarget::Song(first_song_id),
                cx,
            );
        });
    });
    r.focus(operator, &workspace);
    redraw_window(&mut r, operator);

    r.press(operator, "escape");
    assert!(
        r.app
            .update(|cx| r.workspace.read(cx).active_context_menu.is_none()),
        "first Escape must dismiss context menu"
    );
    assert!(
        r.app
            .update(|cx| r.workspace.read(cx).active_modal.is_some()),
        "modal stays open until the next Escape"
    );

    r.press(operator, "escape");
    assert!(
        r.app
            .update(|cx| r.workspace.read(cx).active_modal.is_none()),
        "second Escape must close the open modal"
    );

    // Extra Escape when nothing is open must keep the operator window alive.
    r.press(operator, "escape");
    assert_eq!(
        r.app.update(|cx| cx.windows().len()),
        1,
        "Escape must never close the main window"
    );
}

#[test]
fn right_click_editing_and_quick_slide_presets_route_cleanly() {
    let mut r = rig();
    let operator = r.operator;

    let (song_item_id, scripture_item_id, first_song_id) = r.app.update(|cx| {
        let store = r.store.read(cx);
        let song_item = store
            .service
            .items
            .iter()
            .find(|i| matches!(i.payload, ServiceItemPayload::Song { .. }))
            .expect("seeded song service item")
            .id;
        let scripture_item = store
            .service
            .items
            .iter()
            .find(|i| matches!(i.payload, ServiceItemPayload::Scripture { .. }))
            .expect("seeded scripture service item")
            .id;
        (song_item, scripture_item, store.songs[0].id)
    });

    // Context menu opens and click-outside dismisses it cleanly.
    r.app.update(|cx| {
        r.workspace.update(cx, |ws, cx| {
            ws.open_context_menu(
                Point::new(px(400.0), px(500.0)),
                ContextMenuTarget::Song(first_song_id),
                cx,
            );
        });
    });
    click_at(&mut r, operator, 15.0, 15.0, MouseButton::Left);
    assert!(
        r.app
            .update(|cx| r.workspace.read(cx).active_context_menu.is_none()),
        "clicking outside the context menu must dismiss it"
    );

    // Editing a song service item opens the Song Editor modal.
    r.app.update(|cx| {
        r.workspace
            .update(cx, |ws, cx| ws.edit_service_item(song_item_id, cx));
    });
    assert!(r.app.update(|cx| matches!(
        r.workspace.read(cx).active_modal,
        Some(ActiveModal::SongEditor(_))
    )));

    // Editing a scripture service item jumps directly to the Bible tab.
    r.app.update(|cx| {
        r.workspace.update(cx, |ws, cx| {
            ws.close_modal(cx);
            ws.edit_service_item(scripture_item_id, cx);
        });
    });
    assert_eq!(
        r.app.update(|cx| r.workspace.read(cx).active_library_tab),
        LibraryTab::Bible
    );

    // 1-click Pastor Quote and Member Birthday presets open CustomSlideBuilder with rich specs.
    let quote = quick_slide_preset_spec(QuickSlidePreset::PastorQuote);
    assert!(matches!(
        quote.layout,
        CustomSlideLayout::BigStatement { .. }
    ));
    let birthday = quick_slide_preset_spec(QuickSlidePreset::MemberBirthday);
    assert!(matches!(
        birthday.layout,
        CustomSlideLayout::ImageTextSplit { .. }
    ));

    r.app.update(|cx| {
        r.workspace.update(cx, |ws, cx| {
            ws.open_quick_slide_preset(QuickSlidePreset::MemberBirthday, cx);
        });
    });
    assert!(r.app.update(|cx| matches!(
        r.workspace.read(cx).active_modal,
        Some(ActiveModal::CustomSlideBuilder(_))
    )));
}

#[test]
fn day_one_library_ships_rich_hymns_bibles_themes_and_fonts() {
    let boot = seeded_bootstrap();
    assert!(
        boot.songs.len() >= 19,
        "Day-1 library must ship >= 19 verified PD hymns, got {}",
        boot.songs.len()
    );
    assert!(
        boot.bible_translations.len() >= 3,
        "Day-1 library must ship BSB, WEB, and KJV offline, got {}",
        boot.bible_translations.len()
    );
    assert!(
        boot.themes.len() >= 12,
        "Day-1 library must ship >= 12 studio themes, got {}",
        boot.themes.len()
    );
    assert!(
        PROJECTION_FONTS.len() >= 7,
        "must bundle >= 7 projection font families"
    );
    assert!(
        FREE_BIBLE_CATALOG.len() >= 9,
        "free offline Bible catalog must offer >= 9 verified PD/CC0 translations"
    );
}

#[test]
fn capture_v003_review_screens_when_requested() {
    let Ok(dir) = std::env::var("SUNDAYDECK_V003_CAPTURE_DIR") else {
        return;
    };
    let out_dir = PathBuf::from(dir);
    std::fs::create_dir_all(&out_dir).expect("create v003 review capture dir");

    let mut app = ScreenshotApp::with_assets(DeckAssetSource).expect("headless lavapipe app");
    app.update(|cx| {
        register_fonts(cx).expect("register bundled OFL fonts");
        sundaydeck_ui::actions::register_keybindings(cx);
    });

    let boot = seeded_bootstrap();
    let db_worker = DbWorker::spawn_in_memory().expect("in-memory db worker");
    let (store, windows) = app.update(|cx| {
        let store = cx.new(|_| {
            let mut s = PresentationStore::from_bootstrap(boot);
            s.clock_override_ms = Some(1_000_000);
            s.fixed_clock_label = Some("10:15 AM".to_owned());
            s
        });
        (store, cx.new(|_| WindowManager::new()))
    });

    let op_win = app
        .open_window(size(px(1440.0), px(900.0)), 1.0, {
            let (s, w, d) = (store.clone(), windows.clone(), db_worker.clone());
            move |_, cx| cx.new(|cx| OperatorWorkspace::new(s, w, Some(d), cx))
        })
        .expect("open operator window");

    let aud_win = app
        .open_window(size(px(1280.0), px(720.0)), 1.0, {
            let s = store.clone();
            move |_, cx| cx.new(|cx| AudienceOutputView::new(s, cx))
        })
        .expect("open audience window");

    let first_song_id = app.update(|cx| {
        let s = store.read(cx);
        let id = s.songs[0].id;
        let item_id = s.service.items[1].id;
        store.update(cx, |st, cx| {
            st.activate_service_item(item_id, cx);
            let _ = st.dispatch(PresentationCommand::GoLive, cx);
        });
        id
    });

    // 1. Populated Day-1 Workspace (19 hymns, 12 studio themes, + Quote / Birthday footer)
    app.capture(op_win)
        .expect("cap 01")
        .image
        .save(out_dir.join("01-day1-populated-hymnal-and-quick-footer.png"))
        .expect("save 01");

    // 2. Right-click Context Menu on Hymn
    let _ = app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |ws, cx| {
                ws.open_context_menu(
                    Point::new(px(470.0), px(640.0)),
                    ContextMenuTarget::Song(first_song_id),
                    cx,
                );
            });
        }
    });
    app.capture(op_win)
        .expect("cap 02")
        .image
        .save(out_dir.join("02-right-click-context-menu-song.png"))
        .expect("save 02");

    // 3. Custom Slides Tab with Quick Slide Bar + Pastor Quote & Member Birthday Cards
    let _ = app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |ws, cx| {
                ws.switch_library_tab(LibraryTab::CustomSlides, cx);
            });
        }
    });
    app.capture(op_win)
        .expect("cap 03")
        .image
        .save(out_dir.join("03-custom-slides-quick-create-and-birthdays.png"))
        .expect("save 03");

    // 4. 1-Click Pastor Quote Modal (with click-outside backdrop)
    let _ = app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |ws, cx| {
                ws.open_quick_slide_preset(QuickSlidePreset::PastorQuote, cx);
            });
        }
    });
    app.capture(op_win)
        .expect("cap 04")
        .image
        .save(out_dir.join("04-quick-slide-pastor-quote-modal.png"))
        .expect("save 04");

    // 5. Media / Studio Themes Tab (12 themes, 7 fonts, 6 photos, 2 motion loops)
    let _ = app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |ws, cx| {
                ws.switch_library_tab(LibraryTab::Themes, cx);
            });
        }
    });
    app.capture(op_win)
        .expect("cap 05")
        .image
        .save(out_dir.join("05-studio-themes-and-fonts-gallery.png"))
        .expect("save 05");

    // 6. Audience Output: Warm Dawn Editorial (Cormorant Garamond) + Starry Clouds Motion (Montserrat)
    app.update(|cx| {
        let starry = store
            .read(cx)
            .themes
            .iter()
            .find(|t| t.name == "Warm Dawn Editorial")
            .cloned()
            .expect("Warm Dawn Editorial theme");
        store.update(cx, |s, cx| {
            let _ = s.dispatch(PresentationCommand::ApplyLiveTheme(Arc::clone(&starry)), cx);
        });
    });
    app.capture(aud_win)
        .expect("cap 06")
        .image
        .save(out_dir.join("06-audience-warm-dawn-cormorant.png"))
        .expect("save 06");

    // 7. Settings -> Bibles & Packs: 9 Free 1-Click Offline Bibles Catalog
    let _ = app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |ws, cx| {
                ws.open_settings_on_tab(SettingsTab::BiblesBundles, cx);
            });
        }
    });
    app.capture(op_win)
        .expect("cap 07")
        .image
        .save(out_dir.join("07-settings-free-offline-bibles-catalog.png"))
        .expect("save 07");

    // 8. Bible Browser on BSB with Right-Click Verse Context Menu ("Create Quote Slide from Verse")
    let mut mem_db = Database::open_in_memory().expect("db");
    sundaydeck_db::seed::seed_defaults_if_empty(mem_db.conn_mut()).expect("seed");
    let jn_ref = parse_scripture_ref("jn 3:16-17").expect("parse jn");
    let bsb_verses =
        sundaydeck_db::bible_repo::get_passage(mem_db.conn(), "BSB", &jn_ref).expect("bsb jn 3");
    app.update(|cx| {
        store.update(cx, |s, cx| {
            s.active_translation_code = "BSB".to_owned();
            s.bible_browse_mode = BibleBrowseMode::Chapter {
                book_number: 43,
                chapter: 3,
            };
            s.bible_verses = bsb_verses.clone();
            s.selected_verse_range = Some((0, 0));
            s.stage_scripture_verses(&bsb_verses, false, cx);
        });
    });
    let _ = app.update_window(op_win.into(), |view, _, cx| {
        if let Ok(op) = view.downcast::<OperatorWorkspace>() {
            op.update(cx, |ws, cx| {
                ws.switch_library_tab(LibraryTab::Bible, cx);
                ws.open_context_menu(
                    Point::new(px(490.0), px(670.0)),
                    ContextMenuTarget::BibleVerse(0),
                    cx,
                );
            });
        }
    });
    app.capture(op_win)
        .expect("cap 08")
        .image
        .save(out_dir.join("08-bible-bsb-right-click-quote-slide.png"))
        .expect("save 08");
}
