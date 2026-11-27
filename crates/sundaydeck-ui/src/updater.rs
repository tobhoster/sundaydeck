use std::time::Duration;

/// Current compiled SundayDeck release version from `Cargo.toml`.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Official GitHub Releases latest metadata endpoint (HTTPS only).
pub const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/tobhoster/sundaydeck/releases/latest";

/// Direct download landing page with automatic platform detection.
pub const DOWNLOAD_PAGE_URL: &str = "https://tobhoster.github.io/sundaydeck/download.html";

const ALLOWED_RELEASE_PREFIX: &str = "https://github.com/tobhoster/sundaydeck/";

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
        let release_url = parsed
            .get("html_url")
            .and_then(|v| v.as_str())
            .filter(|u| u.starts_with(ALLOWED_RELEASE_PREFIX))
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| DOWNLOAD_PAGE_URL.to_owned());
        UpdateCheckOutcome::UpdateAvailable {
            current: CURRENT_VERSION.to_owned(),
            latest: latest_clean,
            download_url: release_url,
        }
    } else {
        UpdateCheckOutcome::UpToDate {
            current: CURRENT_VERSION.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_newer_version, parse_semver};

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
}
