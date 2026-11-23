use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{
    AppContext as _, Context, Entity, EventEmitter, IntoElement, PathPromptOptions, Render, Window,
};
use sundaydeck_core::{
    BackgroundSpec, HorizontalAlign, ImageFit, MediaAssetId, Rgba8, SafeMargins, TextShadow, Theme,
    ThemeId, ThemeTarget, VerticalAlign,
};
use sundaydeck_db::DbWorker;

use crate::components::TextInput;
use crate::store::PresentationStore;
use crate::views::background_picker_ui::render_background_picker_modal;
use crate::views::quick_edit_modal::ModalClosed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundTab {
    UnsplashGallery,
    LocalMedia,
}

#[derive(Debug, Clone, Copy)]
pub struct UnsplashPreset {
    pub slug: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub url: &'static str,
    pub embedded_preview: Option<&'static str>,
    pub default_font: &'static str,
    pub target: ThemeTarget,
}

pub const UNSPLASH_PRESETS: &[UnsplashPreset] = &[
    UnsplashPreset {
        slug: "sanctuary-mountains",
        title: "Twilight Mountains",
        subtitle: "Misty pines & river glow · Bundled offline",
        url: "https://images.unsplash.com/photo-1506744038136-46273834b3fb?auto=format&fit=crop&w=1920&h=1080&q=85",
        embedded_preview: Some("backgrounds/sanctuary-mountains.jpg"),
        default_font: "Manrope",
        target: ThemeTarget::Song,
    },
    UnsplashPreset {
        slug: "golden-highlands",
        title: "Golden Light Motion",
        subtitle: "Warm sunbeams over misty valley · Video loop",
        url: "https://images.unsplash.com/photo-1470071459604-3b5ec3a7fe05?auto=format&fit=crop&w=1920&h=1080&q=85",
        embedded_preview: Some("backgrounds/golden-light.jpg"),
        default_font: "Manrope",
        target: ThemeTarget::Song,
    },
    UnsplashPreset {
        slug: "still-waters",
        title: "Still Waters Dusk",
        subtitle: "Calm dark teal horizon · Scripture ready",
        url: "https://images.unsplash.com/photo-1518837695005-2083093ee35b?auto=format&fit=crop&w=1920&h=1080&q=85",
        embedded_preview: Some("backgrounds/still-waters.jpg"),
        default_font: "Newsreader",
        target: ThemeTarget::Scripture,
    },
    UnsplashPreset {
        slug: "starry-night-ridge",
        title: "Starry Night Ridge",
        subtitle: "Deep cosmos over mountain silhouette · Unsplash",
        url: "https://images.unsplash.com/photo-1519681393784-d120267933ba?auto=format&fit=crop&w=1920&h=1080&q=85",
        embedded_preview: Some("backgrounds/starry-night-ridge.jpg"),
        default_font: "Manrope",
        target: ThemeTarget::Song,
    },
    UnsplashPreset {
        slug: "autumn-forest-mist",
        title: "Morning Forest Light",
        subtitle: "Quiet golden rays through pines · Unsplash",
        url: "https://images.unsplash.com/photo-1448375240586-882707db888b?auto=format&fit=crop&w=1920&h=1080&q=85",
        embedded_preview: Some("backgrounds/autumn-forest-mist.jpg"),
        default_font: "Space Grotesk",
        target: ThemeTarget::General,
    },
    UnsplashPreset {
        slug: "warm-dawn-shore",
        title: "Warm Dawn Horizon",
        subtitle: "Soft sunrise over quiet waters · Prayer & Scripture",
        url: "https://images.unsplash.com/photo-1507525428034-b723cf961d3e?auto=format&fit=crop&w=1920&h=1080&q=85",
        embedded_preview: Some("backgrounds/warm-dawn-shore.jpg"),
        default_font: "Newsreader",
        target: ThemeTarget::Scripture,
    },
];

pub const FONT_CHOICES: &[(&str, &str)] = &[
    ("Manrope", "Manrope Bold"),
    ("Newsreader", "Newsreader Serif"),
    ("Space Grotesk", "Space Grotesk"),
    ("Syne", "Syne Display"),
];

pub const SCRIM_CHOICES: &[(&str, f32)] = &[
    ("Light 35%", 0.35),
    ("Balanced 48%", 0.48),
    ("Deep 62%", 0.62),
];

/// Modal for creating & applying new themes backed by Unsplash photography or local looping videos
/// (`.mp4`, `.mov`, `.webm`) and custom sanctuary images (`.jpg`, `.png`, `.webp`).
pub struct BackgroundPickerModal {
    store: Entity<PresentationStore>,
    db: Option<DbWorker>,
    pub(crate) active_tab: BackgroundTab,
    pub(crate) selected_preset_idx: usize,
    pub(crate) theme_name_input: Entity<TextInput>,
    pub(crate) url_input: Entity<TextInput>,
    pub(crate) file_path_input: Entity<TextInput>,
    pub(crate) selected_font: String,
    pub(crate) scrim_opacity: f32,
    pub(crate) busy: bool,
    pub(crate) error_message: Option<String>,
}

impl EventEmitter<ModalClosed> for BackgroundPickerModal {}

impl BackgroundPickerModal {
    pub fn new(
        store: Entity<PresentationStore>,
        db: Option<DbWorker>,
        cx: &mut Context<Self>,
    ) -> Self {
        let first = &UNSPLASH_PRESETS[3];
        let theme_name_input =
            cx.new(|cx| TextInput::single_line("bg-theme-name", "Theme name", first.title, cx));
        let url_input = cx.new(|cx| {
            TextInput::single_line(
                "bg-unsplash-url",
                "https://images.unsplash.com/photo-...",
                first.url,
                cx,
            )
        });
        let file_path_input = cx.new(|cx| {
            TextInput::single_line(
                "bg-local-file",
                "/path/to/worship-loop.mp4 or background.jpg",
                "",
                cx,
            )
        });
        Self {
            store,
            db,
            active_tab: BackgroundTab::UnsplashGallery,
            selected_preset_idx: 3,
            theme_name_input,
            url_input,
            file_path_input,
            selected_font: first.default_font.to_owned(),
            scrim_opacity: 0.48,
            busy: false,
            error_message: None,
        }
    }

    pub(crate) fn select_preset(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(preset) = UNSPLASH_PRESETS.get(idx) else {
            return;
        };
        self.selected_preset_idx = idx;
        self.selected_font = preset.default_font.to_owned();
        self.theme_name_input
            .update(cx, |inp, cx| inp.set_value(preset.title, cx));
        self.url_input
            .update(cx, |inp, cx| inp.set_value(preset.url, cx));
        self.error_message = None;
        cx.notify();
    }

    pub(crate) fn browse_local_media(&mut self, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(
                "Select Sanctuary Background Video (.mp4/.mov/.webm) or Photo (.jpg/.png)".into(),
            ),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = rx.await
                && let Some(path) = paths.into_iter().next()
            {
                let path_str = path.display().to_string();
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Custom Background")
                    .replace(['-', '_'], " ");
                let _ = this.update(cx, |modal, cx| {
                    modal.active_tab = BackgroundTab::LocalMedia;
                    modal
                        .file_path_input
                        .update(cx, |inp, cx| inp.set_value(&path_str, cx));
                    modal
                        .theme_name_input
                        .update(cx, |inp, cx| inp.set_value(&stem, cx));
                    modal.error_message = None;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(crate) fn create_and_apply(&mut self, cx: &mut Context<Self>) {
        let raw_name = self.theme_name_input.read(cx).value().trim().to_owned();
        let name = if raw_name.is_empty() {
            "Custom Sanctuary Background".to_owned()
        } else {
            raw_name
        };
        let font = self.selected_font.clone();
        let scrim = self.scrim_opacity;

        match self.active_tab {
            BackgroundTab::LocalMedia => {
                let raw_path = self.file_path_input.read(cx).value().trim().to_owned();
                if raw_path.is_empty() {
                    self.error_message = Some(
                        "Choose or enter a local .mp4, .mov, .webm, .jpg, or .png file".to_owned(),
                    );
                    cx.notify();
                    return;
                }
                let path = PathBuf::from(&raw_path);
                if !path.is_file() {
                    self.error_message = Some(format!("File not found: {raw_path}"));
                    cx.notify();
                    return;
                }
                let bg = build_local_media_spec(&path, scrim);
                let theme = build_custom_theme(name, font, bg);
                self.persist_and_close(theme, cx);
            }
            BackgroundTab::UnsplashGallery => {
                let url = self.url_input.read(cx).value().trim().to_owned();
                if url.is_empty() {
                    self.error_message =
                        Some("Choose an Unsplash scene or paste an image URL".to_owned());
                    cx.notify();
                    return;
                }
                // Fast offline path if a bundled preset matches the selected URL.
                if let Some(preset) = UNSPLASH_PRESETS.iter().find(|p| p.url == url)
                    && let Some(embedded) = preset.embedded_preview
                {
                    let bg = if preset.slug == "golden-highlands" {
                        BackgroundSpec::VideoLoop {
                            asset_id: MediaAssetId::new_v7(),
                            uri: "video/golden-light-loop.mp4".to_owned(),
                            poster_uri: embedded.to_owned(),
                            fps: 1.0,
                            scrim_opacity: scrim,
                        }
                    } else {
                        BackgroundSpec::Image {
                            asset_id: MediaAssetId::new_v7(),
                            uri: Some(embedded.to_owned()),
                            fit: ImageFit::Cover,
                            scrim_opacity: scrim,
                        }
                    };
                    let theme = build_custom_theme(name, font, bg);
                    self.persist_and_close(theme, cx);
                    return;
                }

                // Download Unsplash image off-thread into local offline media cache.
                self.busy = true;
                self.error_message = None;
                cx.notify();

                let (tx, rx) = async_channel::bounded::<Result<PathBuf, String>>(1);
                let slug = slugify(&name);
                std::thread::spawn(move || {
                    let res = download_unsplash_to_cache(&url, &slug);
                    let _ = tx.send_blocking(res);
                });

                cx.spawn(async move |this, cx| {
                    let res = rx.recv().await;
                    let _ = this.update(cx, |modal, cx| {
                        modal.busy = false;
                        match res {
                            Ok(Ok(local_path)) => {
                                let bg = BackgroundSpec::Image {
                                    asset_id: MediaAssetId::new_v7(),
                                    uri: Some(local_path.display().to_string()),
                                    fit: ImageFit::Cover,
                                    scrim_opacity: scrim,
                                };
                                let theme = build_custom_theme(name, font, bg);
                                modal.persist_and_close(theme, cx);
                            }
                            Ok(Err(err)) => {
                                modal.error_message = Some(err);
                                cx.notify();
                            }
                            Err(_) => {
                                modal.error_message = Some("Download worker closed".to_owned());
                                cx.notify();
                            }
                        }
                    });
                })
                .detach();
            }
        }
    }

    fn persist_and_close(&mut self, theme: Theme, cx: &mut Context<Self>) {
        let saved = self
            .store
            .update(cx, |s, cx| s.upsert_theme_local(theme, true, cx));
        if let Some(db) = self.db.clone() {
            cx.spawn(async move |_, _| {
                let _ = db.upsert_theme(saved).await;
            })
            .detach();
        }
        cx.emit(ModalClosed);
    }
}

impl Render for BackgroundPickerModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        render_background_picker_modal(self, cx)
    }
}

fn build_local_media_spec(path: &Path, scrim_opacity: f32) -> BackgroundSpec {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let uri = path.display().to_string();
    if matches!(ext.as_str(), "mp4" | "mov" | "webm" | "mkv" | "m4v") {
        BackgroundSpec::VideoLoop {
            asset_id: MediaAssetId::new_v7(),
            uri,
            poster_uri: "backgrounds/golden-light.jpg".to_owned(),
            fps: 1.0,
            scrim_opacity,
        }
    } else {
        BackgroundSpec::Image {
            asset_id: MediaAssetId::new_v7(),
            uri: Some(uri),
            fit: ImageFit::Cover,
            scrim_opacity,
        }
    }
}

fn build_custom_theme(name: String, font_family: String, background: BackgroundSpec) -> Theme {
    let is_serif = font_family == "Newsreader";
    Theme {
        id: ThemeId::new_v7(),
        name,
        target: if is_serif {
            ThemeTarget::Scripture
        } else {
            ThemeTarget::Song
        },
        is_default: false,
        background,
        font_family,
        font_size_pt: if is_serif { 64.0 } else { 76.0 },
        font_weight: if is_serif { 400 } else { 800 },
        line_height: if is_serif { 1.40 } else { 1.28 },
        text_color: Rgba8::from_rgb(245, 243, 239),
        h_align: if is_serif {
            HorizontalAlign::Left
        } else {
            HorizontalAlign::Center
        },
        v_align: VerticalAlign::Middle,
        stroke_width_px: if is_serif { 0.0 } else { 2.0 },
        stroke_color: Rgba8::from_rgba(0, 0, 0, 190),
        shadow: Some(TextShadow {
            offset_x_px: 0.0,
            offset_y_px: 6.0,
            blur_radius_px: 16.0,
            color: Rgba8::from_rgba(0, 0, 0, 200),
        }),
        margins: SafeMargins::DEFAULT,
        footer_font_size_pt: 22.0,
        transition_ms: 400,
    }
}

fn download_unsplash_to_cache(url: &str, slug: &str) -> Result<PathBuf, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
    let dir = PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("sundaydeck")
        .join("media");
    std::fs::create_dir_all(&dir).map_err(|e| format!("create media dir: {e}"))?;
    let dest = dir.join(format!("unsplash-{slug}.jpg"));

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into();
    let mut resp = agent
        .get(url)
        .call()
        .map_err(|e| format!("Unsplash download failed: {e}"))?;
    let bytes = resp
        .body_mut()
        .with_config()
        .limit(12 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("read image bytes: {e}"))?;
    if bytes.len() < 1024 {
        return Err("Downloaded image was empty or invalid".to_owned());
    }
    std::fs::write(&dest, &bytes).map_err(|e| format!("save image: {e}"))?;
    Ok(dest)
}

fn slugify(input: &str) -> String {
    let s: String = input
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = s.trim_matches('-');
    if trimmed.is_empty() {
        "custom".to_owned()
    } else {
        trimmed.to_owned()
    }
}
