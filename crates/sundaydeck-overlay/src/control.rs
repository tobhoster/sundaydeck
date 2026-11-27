use serde::Deserialize;

/// Maximum sanitized length for remote alert or parent-code text payloads.
const MAX_ALERT_CHARS: usize = 180;
const MAX_PARENT_CODE_CHARS: usize = 24;

/// Typed booth remote / Stream Deck / Bitfocus Companion control commands accepted over
/// localhost `POST /api/trigger/<action>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteControlCommand {
    GoLive,
    NextSlide,
    PrevSlide,
    ToggleClearText,
    ToggleBlack,
    ToggleLogo,
    ClearOverrides,
    ShowAlert(String),
    PageParent(String),
    ClearAlert,
}

#[derive(Deserialize)]
struct AlertBody {
    #[serde(default)]
    message: String,
    #[serde(default)]
    code: String,
}

/// Parses an incoming HTTP method, path, and optional body into a [`RemoteControlCommand`].
/// Returns `Err((http_status, message))` for invalid methods, unknown actions, or empty payloads.
pub fn parse_trigger_request(
    method: &str,
    path: &str,
    body: &str,
) -> Result<RemoteControlCommand, (&'static str, &'static str)> {
    if !method.eq_ignore_ascii_case("POST") {
        return Err((
            "405 Method Not Allowed",
            "Use POST for /api/trigger endpoints",
        ));
    }
    let action = path
        .split('?')
        .next()
        .unwrap_or(path)
        .strip_prefix("/api/trigger/")
        .unwrap_or("");

    match action {
        "go-live" => Ok(RemoteControlCommand::GoLive),
        "next" => Ok(RemoteControlCommand::NextSlide),
        "prev" => Ok(RemoteControlCommand::PrevSlide),
        "clear-text" => Ok(RemoteControlCommand::ToggleClearText),
        "black" => Ok(RemoteControlCommand::ToggleBlack),
        "logo" => Ok(RemoteControlCommand::ToggleLogo),
        "clear-overrides" => Ok(RemoteControlCommand::ClearOverrides),
        "clear-alert" => Ok(RemoteControlCommand::ClearAlert),
        "alert" => {
            let text = extract_text_field(body, false);
            let clean = sanitize_single_line(&text, MAX_ALERT_CHARS);
            if clean.is_empty() {
                return Err(("400 Bad Request", "Alert message cannot be empty"));
            }
            Ok(RemoteControlCommand::ShowAlert(clean))
        }
        "parent-page" => {
            let raw = extract_text_field(body, true);
            let clean = sanitize_single_line(&raw, MAX_PARENT_CODE_CHARS);
            if clean.is_empty() {
                return Err(("400 Bad Request", "Parent security code cannot be empty"));
            }
            Ok(RemoteControlCommand::PageParent(clean))
        }
        _ => Err(("404 Not Found", "Unknown trigger action")),
    }
}

/// Extracts a message or code from either JSON (`{"message":"..."}` / `{"code":"..."}`) or plain text.
fn extract_text_field(body: &str, prefer_code: bool) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('{')
        && let Ok(parsed) = serde_json::from_str::<AlertBody>(trimmed)
    {
        if prefer_code {
            if !parsed.code.trim().is_empty() {
                return parsed.code;
            }
            return parsed.message;
        }
        if !parsed.message.trim().is_empty() {
            return parsed.message;
        }
        return parsed.code;
    }
    trimmed.to_owned()
}

fn sanitize_single_line(raw: &str, max_chars: usize) -> String {
    raw.chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(max_chars)
        .collect()
}

/// Builds the `GET /api/status` JSON document for Bitfocus Companion / Stream Deck feedback.
pub fn build_api_status_json(latest_overlay_json: &str) -> String {
    format!(
        r#"{{"ok":true,"state":{latest_overlay_json},"triggers":["go-live","next","prev","clear-text","black","logo","clear-overrides","alert","parent-page","clear-alert"]}}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_companion_triggers_and_rejects_get_method() {
        assert_eq!(
            parse_trigger_request("POST", "/api/trigger/next", ""),
            Ok(RemoteControlCommand::NextSlide)
        );
        assert_eq!(
            parse_trigger_request("POST", "/api/trigger/black", ""),
            Ok(RemoteControlCommand::ToggleBlack)
        );
        assert_eq!(
            parse_trigger_request("POST", "/api/trigger/parent-page", r#"{"code":"142"}"#),
            Ok(RemoteControlCommand::PageParent("142".to_owned()))
        );
        assert_eq!(
            parse_trigger_request("POST", "/api/trigger/alert", "Nursery check-in"),
            Ok(RemoteControlCommand::ShowAlert(
                "Nursery check-in".to_owned()
            ))
        );
        assert!(parse_trigger_request("GET", "/api/trigger/next", "").is_err());
        assert!(parse_trigger_request("POST", "/api/trigger/unknown", "").is_err());
    }
}
