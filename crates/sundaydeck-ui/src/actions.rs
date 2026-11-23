use gpui::{App, KeyBinding, actions};

actions!(
    sundaydeck,
    [
        GoLive,
        NextLiveSlide,
        PrevLiveSlide,
        ToggleClearText,
        ToggleBlack,
        ToggleLogo,
        HardCutClearText,
        HardCutBlack,
        HardCutLogo,
        ToggleAudienceWindow,
        ToggleStageWindow,
        ToggleDirectClickLive,
        ToggleQuickEdit,
        ClearStageMessage,
        QuitApp,
    ]
);

/// Registers global recovery hotkeys and workspace navigation bindings.
/// Recovery overrides (`F1`-`F3`) stay active globally (`Shift+F1`-`F3` perform instant hard cuts);
/// slide stepping is scoped to `OperatorWorkspace` so typing into text fields never advances slides.
pub fn register_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("f1", ToggleClearText, None),
        KeyBinding::new("f2", ToggleBlack, None),
        KeyBinding::new("f3", ToggleLogo, None),
        KeyBinding::new("shift-f1", HardCutClearText, None),
        KeyBinding::new("shift-f2", HardCutBlack, None),
        KeyBinding::new("shift-f3", HardCutLogo, None),
        KeyBinding::new("f5", GoLive, None),
        KeyBinding::new("cmd-enter", GoLive, None),
        KeyBinding::new("ctrl-enter", GoLive, None),
        KeyBinding::new("right", NextLiveSlide, Some("OperatorWorkspace")),
        KeyBinding::new("down", NextLiveSlide, Some("OperatorWorkspace")),
        KeyBinding::new("pagedown", NextLiveSlide, Some("OperatorWorkspace")),
        KeyBinding::new("space", NextLiveSlide, Some("OperatorWorkspace")),
        KeyBinding::new("left", PrevLiveSlide, Some("OperatorWorkspace")),
        KeyBinding::new("up", PrevLiveSlide, Some("OperatorWorkspace")),
        KeyBinding::new("pageup", PrevLiveSlide, Some("OperatorWorkspace")),
        KeyBinding::new("cmd-e", ToggleQuickEdit, Some("OperatorWorkspace")),
        KeyBinding::new("ctrl-e", ToggleQuickEdit, Some("OperatorWorkspace")),
        KeyBinding::new("cmd-q", QuitApp, None),
        KeyBinding::new("ctrl-q", QuitApp, None),
    ]);
}
