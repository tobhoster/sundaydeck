# SundayDeck

Lightweight, free, open-source, offline-first church presentation desktop application written in **Rust** using **GPUI**.

Designed for small and mid-sized churches run by volunteers—prioritizing instant startup, zero crashes during live services, effortless keyboard-first operation, and low hardware requirements across macOS, Windows, and Linux.

## Workspace Layout

- `crates/sundaydeck-core` — Pure domain models (`Song`, `Arrangement`, `Theme`, `ServiceSchedule`, `Slide`), deterministic lyric reflow engine (`ByStanza`, `1`, `2`, `4` lines per slide), and the single-authority `PresentationEngine` state machine (`Preview`, `Live`, `ClearText`, `Black`, `Logo`).
- `crates/sundaydeck-db` — Local SQLite persistence in WAL mode, versioned SQL migrations, sub-millisecond FTS5 song search, content-addressed media deduplication, and a dedicated off-thread `DbWorker`.
- `crates/sundaydeck-ui` — GPUI desktop application: 3-pane Operator Workspace, fullscreen/windowed Audience Output window with GPU opacity crossfades, high-contrast Stage / Confidence Monitor window, and global keyboard shortcut router.

## Keyboard Shortcuts

| Shortcut | Scope | Action |
|---|---|---|
| `F1` | Global | Toggle **Clear Text** (keeps background active, hides lyrics) |
| `F2` | Global | Toggle **Black** output override |
| `F3` | Global | Toggle **Logo** splash override |
| `F5` / `Cmd+Enter` / `Ctrl+Enter` | Global | **Go Live** (promote Preview to Live) |
| `Right` / `Down` / `PageDown` / `Space` | Operator Workspace | Next live slide |
| `Left` / `Up` / `PageUp` | Operator Workspace | Previous live slide |
| `Cmd+Q` / `Ctrl+Q` | Global | Close all output windows and quit cleanly |

## Running & Testing

```bash
# Run core domain & SQLite FTS5 integration tests
cargo test -p sundaydeck-core -p sundaydeck-db

# Launch the desktop application
cargo run -p sundaydeck-ui --bin sundaydeck

# Run headless Mesa/lavapipe GPUI visual captures on Linux
VK_ICD_FILENAMES=/path/to/lvp_icd.x86_64.json \
  cargo test -p sundaydeck-ui --test ui_review_capture
```
