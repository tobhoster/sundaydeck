use gpui::{App, Menu, MenuItem, OsAction, SystemMenuType};

use crate::actions::{
    CheckForUpdates, CloseActiveWindow, EditCopy, EditCut, EditPaste, EditSelectAll, GoLive,
    ImportPastorPlan, NewCustomSlide, NewService, NewSong, NextLiveSlide, OpenAlertsTimers,
    OpenBackgroundStudio, OpenBiblesBundles, OpenCheatSheetHelp, OpenGithubIssues, OpenSettings,
    OpenStarterPacksHelp, OpenWelcomeSetup, PrevLiveSlide, QuitApp, ResetWorkspaceLayout,
    SwitchTabBible, SwitchTabCustomSlides, SwitchTabSongs, SwitchTabThemes, ToggleAudienceWindow,
    ToggleBlack, ToggleClearText, ToggleLeftPane, ToggleLibraryDrawer, ToggleLogo,
    ToggleObsOverlay, ToggleQuickEdit, ToggleRightPane, ToggleStageWindow,
};
use crate::store::AppServices;

const CHEAT_SHEET_URL: &str = "https://tobhoster.github.io/sundaydeck/cheat-sheet.html";
const STARTER_PACKS_URL: &str = "https://tobhoster.github.io/sundaydeck/packs.html";
const GITHUB_ISSUES_URL: &str = "https://github.com/tobhoster/sundaydeck/issues";

/// Registers native OS application menus (`SundayDeck`, `File`, `Edit`, `View`, `Window`, `Help`)
/// and binds global app-level handlers (Quit, Help links, and Output Window toggles) so menu items
/// remain responsive regardless of which window or input is currently focused.
pub fn register_app_menus(cx: &mut App) {
    // Voice shutdown and output-window teardown run in the `on_app_quit` hook (see `main.rs`).
    cx.on_action(|_: &QuitApp, cx| cx.quit());
    cx.on_action(|_: &ToggleAudienceWindow, cx| {
        if let Some((store, windows)) = cx
            .try_global::<AppServices>()
            .map(|s| (s.store.clone(), s.windows.clone()))
        {
            windows.update(cx, |wm, cx| wm.toggle_audience(store, cx));
        }
    });
    cx.on_action(|_: &ToggleStageWindow, cx| {
        if let Some((store, windows)) = cx
            .try_global::<AppServices>()
            .map(|s| (s.store.clone(), s.windows.clone()))
        {
            windows.update(cx, |wm, cx| wm.toggle_stage(store, cx));
        }
    });
    cx.on_action(|_: &ToggleObsOverlay, cx| {
        if let Some(store) = cx.try_global::<AppServices>().map(|s| s.store.clone()) {
            store.update(cx, |s, cx| s.toggle_obs_overlay(cx));
        }
    });
    cx.on_action(|_: &OpenCheatSheetHelp, cx| cx.open_url(CHEAT_SHEET_URL));
    cx.on_action(|_: &OpenStarterPacksHelp, cx| cx.open_url(STARTER_PACKS_URL));
    cx.on_action(|_: &OpenGithubIssues, cx| cx.open_url(GITHUB_ISSUES_URL));

    cx.set_menus(build_app_menus());
}

/// Builds the complete native desktop menu hierarchy for SundayDeck.
pub fn build_app_menus() -> Vec<Menu> {
    vec![
        Menu::new("SundayDeck").items([
            MenuItem::action("About SundayDeck & Setup Wizard", OpenWelcomeSetup),
            MenuItem::action("Check for Updates…", CheckForUpdates),
            MenuItem::separator(),
            MenuItem::action("Settings…", OpenSettings),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Quit SundayDeck", QuitApp),
        ]),
        Menu::new("File").items([
            MenuItem::action("New Sunday Service", NewService),
            MenuItem::action("New Song…", NewSong),
            MenuItem::action("New Custom Slide…", NewCustomSlide),
            MenuItem::separator(),
            MenuItem::action("Import Pastor's Plan (.pptx / .docx)…", ImportPastorPlan),
            MenuItem::action("Migrate Songs, Bibles & .sdeck Packs…", OpenBiblesBundles),
            MenuItem::separator(),
            MenuItem::action("Add Sanctuary Background…", OpenBackgroundStudio),
            MenuItem::separator(),
            MenuItem::action("Close Window", CloseActiveWindow),
        ]),
        Menu::new("Edit").items([
            MenuItem::action("Quick Edit Live Slide…", ToggleQuickEdit),
            MenuItem::separator(),
            MenuItem::os_action("Cut", EditCut, OsAction::Cut),
            MenuItem::os_action("Copy", EditCopy, OsAction::Copy),
            MenuItem::os_action("Paste", EditPaste, OsAction::Paste),
            MenuItem::os_action("Select All", EditSelectAll, OsAction::SelectAll),
        ]),
        Menu::new("View").items([
            MenuItem::action("Go Live (Show Preview)", GoLive),
            MenuItem::action("Next Slide", NextLiveSlide),
            MenuItem::action("Previous Slide", PrevLiveSlide),
            MenuItem::separator(),
            MenuItem::action("Hide Words (Keep Background)", ToggleClearText),
            MenuItem::action("Black Screen", ToggleBlack),
            MenuItem::action("Show Church Logo", ToggleLogo),
            MenuItem::separator(),
            MenuItem::action("Alerts & Countdown Timers…", OpenAlertsTimers),
            MenuItem::separator(),
            MenuItem::action("Toggle Service Order Pane", ToggleLeftPane),
            MenuItem::action("Toggle Live Deck Pane", ToggleRightPane),
            MenuItem::action("Toggle Library Drawer", ToggleLibraryDrawer),
            MenuItem::action("Reset Workspace Layout", ResetWorkspaceLayout),
            MenuItem::separator(),
            MenuItem::action("Songs Library", SwitchTabSongs),
            MenuItem::action("Bible Browser", SwitchTabBible),
            MenuItem::action("Custom Slides", SwitchTabCustomSlides),
            MenuItem::action("Media & Themes", SwitchTabThemes),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Audience Projector Output", ToggleAudienceWindow),
            MenuItem::action("Stage Confidence Monitor", ToggleStageWindow),
            MenuItem::separator(),
            MenuItem::action("Toggle OBS Livestream Overlay", ToggleObsOverlay),
        ]),
        Menu::new("Help").items([
            MenuItem::action("Volunteer Cheat Sheet", OpenCheatSheetHelp),
            MenuItem::action("Public Domain Starter Packs", OpenStarterPacksHelp),
            MenuItem::separator(),
            MenuItem::action("Check for Updates…", CheckForUpdates),
            MenuItem::action("Report an Issue on GitHub…", OpenGithubIssues),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::build_app_menus;

    #[test]
    fn exposes_standard_desktop_menu_bar() {
        let menus = build_app_menus();
        let names: Vec<&str> = menus.iter().map(|m| m.name.as_ref()).collect();
        assert_eq!(
            names,
            vec!["SundayDeck", "File", "Edit", "View", "Window", "Help"]
        );
        for menu in &menus {
            assert!(
                !menu.items.is_empty(),
                "menu {} should have items",
                menu.name
            );
        }
    }
}
