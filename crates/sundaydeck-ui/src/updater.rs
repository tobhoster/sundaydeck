use std::time::Duration;

use gpui::{App, Entity, WeakEntity};

use crate::store::PresentationStore;
use crate::views::OperatorWorkspace;

/// Current compiled SundayDeck release version from `Cargo.toml`.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Official GitHub Releases latest metadata endpoint (HTTPS only).
pub const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/tobhoster/sundaydeck/releases/latest";

/// Direct download landing page with automatic platform detection.
pub const DOWNLOAD_PAGE_URL: &str = "https://tobhoster.github.io/sundaydeck/download.html";

const ALLOWED_RELEASE_PREFIX: &str = "https://github.com/tobhoster/sundaydeck/";
const STARTUP_CHECK_DELAY_SECS: u64 = 4;
const PERIODIC_CHECK_INTERVAL_SECS: u64 = 12 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckOutcome {
    UpToDate {
        current: String,
    },
    UpdateAvailable {
        current: String,
        latest: String,
        download_url: String,
    },
    OfflineOrError(String),
}

/// Parses a semantic version tag like `"v0.0.2"`, `"0.1.4"`, or `"v1.2.3-rc1"`.
pub fn parse_semver(raw: &str) -> Option<(u64, u64, u64)> {
    let clean = raw.trim().trim_start_matches(['v', 'V']);
    let core = clean.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Returns `true` when `candidate` is strictly newer than `current`.
pub fn is_newer_version(current: &str, candidate: &str) -> bool {
    match (parse_semver(current), parse_semver(candidate)) {
        (Some(cur), Some(next)) => next > cur,
        _ => false,
    }
}

/// Blocking HTTPS check against GitHub Releases (`7s` timeout).
/// Always call from a background executor so the UI never stalls on offline church networks.
pub fn check_for_updates_blocking() -> UpdateCheckOutcome {
    let config = ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(7)))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let user_agent = format!("SundayDeck/{CURRENT_VERSION}");

    let response = match agent
        .get(GITHUB_LATEST_RELEASE_API)
        .header("User-Agent", &user_agent)
        .header("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(resp) => resp,
        Err(_) => {
            return UpdateCheckOutcome::OfflineOrError(
                "Could not reach GitHub Releases (offline). Your Sunday service is unaffected."
                    .to_owned(),
            );
        }
    };

    let body = match response.into_body().read_to_string() {
        Ok(text) => text,
        Err(_) => {
            return UpdateCheckOutcome::OfflineOrError(
                "Could not read update metadata from GitHub Releases.".to_owned(),
            );
        }
    };

    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            return UpdateCheckOutcome::OfflineOrError(
                "Unexpected response while checking for updates.".to_owned(),
            );
        }
    };

    let Some(tag) = parsed.get("tag_name").and_then(|v| v.as_str()) else {
        return UpdateCheckOutcome::OfflineOrError(
            "Latest release tag was missing in GitHub response.".to_owned(),
        );
    };

    let latest_clean = tag.trim().trim_start_matches(['v', 'V']).to_owned();
    if is_newer_version(CURRENT_VERSION, &latest_clean) {
        let download_url =
            select_download_url(&parsed, std::env::consts::OS, std::env::consts::ARCH);
        UpdateCheckOutcome::UpdateAvailable {
            current: CURRENT_VERSION.to_owned(),
            latest: latest_clean,
            download_url,
        }
    } else {
        UpdateCheckOutcome::UpToDate {
            current: CURRENT_VERSION.to_owned(),
        }
    }
}

/// Picks the matching direct installer asset from a GitHub Release JSON payload when present,
/// falling back to the release `html_url` or official download page. Only URLs under
/// `ALLOWED_RELEASE_PREFIX` are accepted.
pub fn select_download_url(release_json: &serde_json::Value, os: &str, arch: &str) -> String {
    let desired_suffix = match (os, arch) {
        ("macos", "aarch64") => Some("-macOS-arm64.dmg"),
        ("macos", "x86_64") => Some("-macOS-x86_64.dmg"),
        ("windows", _) => Some("-windows-x86_64-setup.exe"),
        ("linux", _) => Some("_amd64.deb"),
        _ => None,
    };
    if let (Some(suffix), Some(assets)) = (
        desired_suffix,
        release_json.get("assets").and_then(|v| v.as_array()),
    ) && let Some(asset_url) = assets.iter().find_map(|asset| {
        let name = asset.get("name").and_then(|v| v.as_str())?;
        if !name.ends_with(suffix) {
            return None;
        }
        asset
            .get("browser_download_url")
            .and_then(|v| v.as_str())
            .filter(|u| u.starts_with(ALLOWED_RELEASE_PREFIX))
    }) {
        return asset_url.to_owned();
    }

    release_json
        .get("html_url")
        .and_then(|v| v.as_str())
        .filter(|u| u.starts_with(ALLOWED_RELEASE_PREFIX))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| DOWNLOAD_PAGE_URL.to_owned())
}

/// Quietly checks GitHub Releases shortly after app launch and every 12 hours thereafter.
/// Completely silent when offline or already up to date so Sunday services are never disturbed;
/// surfaces the calm header update banner + `Download vX.Y.Z` button only when a newer version exists.
pub fn spawn_auto_update_check(
    workspace: WeakEntity<OperatorWorkspace>,
    store: Entity<PresentationStore>,
    cx: &mut App,
) {
    if std::env::var_os("SUNDAYDECK_DISABLE_AUTO_UPDATE").is_some() {
        return;
    }
    cx.spawn(async move |cx| {
        let mut delay = Duration::from_secs(STARTUP_CHECK_DELAY_SECS);
        loop {
            cx.background_executor().timer(delay).await;
            delay = Duration::from_secs(PERIODIC_CHECK_INTERVAL_SECS);

            let enabled = cx.update(|cx| store.read(cx).settings.auto_check_updates);
            if !enabled {
                continue;
            }
            if workspace.upgrade().is_none() {
                break;
            }

            let outcome = cx
                .background_executor()
                .spawn(async move { check_for_updates_blocking() })
                .await;

            if let UpdateCheckOutcome::UpdateAvailable {
                latest,
                download_url,
                ..
            } = outcome
            {
                let applied = workspace.update(cx, |ws, cx| {
                    let already_announced = ws
                        .available_update
                        .as_ref()
                        .is_some_and(|(ver, _)| ver == &latest);
                    ws.available_update = Some((latest.clone(), download_url));
                    if !already_announced {
                        ws.store.update(cx, |s, cx| {
                            s.set_status(
                                format!(
                                    "SundayDeck v{latest} is available — click Download to update."
                                ),
                                cx,
                            );
                        });
                    }
                    cx.notify();
                });
                if applied.is_err() {
                    break;
                }
            }
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::{DOWNLOAD_PAGE_URL, is_newer_version, parse_semver, select_download_url};

    #[test]
    fn parses_semver_tags_and_compares_versions() {
        assert_eq!(parse_semver("v0.0.1"), Some((0, 0, 1)));
        assert_eq!(parse_semver("0.2.10-beta.1"), Some((0, 2, 10)));
        assert_eq!(parse_semver("invalid"), None);
        assert!(is_newer_version("0.0.1", "v0.0.2"));
        assert!(is_newer_version("0.0.9", "0.1.0"));
        assert!(!is_newer_version("0.0.2", "v0.0.2"));
        assert!(!is_newer_version("0.1.0", "0.0.9"));
    }

    #[test]
    fn selects_platform_installer_asset_and_rejects_untrusted_urls() {
        let payload = serde_json::json!({
            "html_url": "https://github.com/tobhoster/sundaydeck/releases/tag/v0.0.5",
            "assets": [
                {
                    "name": "SundayDeck-0.0.5-macOS-arm64.dmg",
                    "browser_download_url": "https://github.com/tobhoster/sundaydeck/releases/download/v0.0.5/SundayDeck-0.0.5-macOS-arm64.dmg"
                },
                {
                    "name": "SundayDeck-0.0.5-windows-x86_64-setup.exe",
                    "browser_download_url": "https://github.com/tobhoster/sundaydeck/releases/download/v0.0.5/SundayDeck-0.0.5-windows-x86_64-setup.exe"
                },
                {
                    "name": "sundaydeck_0.0.5_amd64.deb",
                    "browser_download_url": "https://evil.example.com/spoofed.deb"
                }
            ]
        });

        assert_eq!(
            select_download_url(&payload, "macos", "aarch64"),
            "https://github.com/tobhoster/sundaydeck/releases/download/v0.0.5/SundayDeck-0.0.5-macOS-arm64.dmg"
        );
        assert_eq!(
            select_download_url(&payload, "windows", "x86_64"),
            "https://github.com/tobhoster/sundaydeck/releases/download/v0.0.5/SundayDeck-0.0.5-windows-x86_64-setup.exe"
        );
        // Untrusted asset URL falls back to verified GitHub release html_url.
        assert_eq!(
            select_download_url(&payload, "linux", "x86_64"),
            "https://github.com/tobhoster/sundaydeck/releases/tag/v0.0.5"
        );
        // Missing/untrusted html_url falls back to official download page.
        let untrusted = serde_json::json!({ "html_url": "https://evil.example.com/phish" });
        assert_eq!(
            select_download_url(&untrusted, "macos", "aarch64"),
            DOWNLOAD_PAGE_URL
        );
    }
}
