mod html;
mod ws;

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sundaydeck_core::{
    AudienceRenderState, CustomSlideLayout, PresentationSnapshot, SlideContent, TransitionDirection,
};

use crate::html::OVERLAY_HTML;
use crate::ws::{encode_ws_text_frame, websocket_accept_key};

pub const DEFAULT_OVERLAY_BIND_ADDR: &str = "127.0.0.1:8787";

/// Real-time lower-third state synced to OBS Browser Source WebSocket clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayPayload {
    pub version: u64,
    pub mode: String,
    pub section_label: Option<String>,
    pub body: String,
    pub footer: Option<String>,
    pub transition: String,
    pub direction: String,
    pub duration_ms: u16,
    pub alert: Option<String>,
}

impl Default for OverlayPayload {
    fn default() -> Self {
        Self {
            version: 0,
            mode: "clear".to_owned(),
            section_label: None,
            body: String::new(),
            footer: None,
            transition: "crossfade".to_owned(),
            direction: "forward".to_owned(),
            duration_ms: 280,
            alert: None,
        }
    }
}

impl OverlayPayload {
    pub fn from_snapshot(snapshot: &PresentationSnapshot) -> Self {
        match &snapshot.audience {
            AudienceRenderState::Black { transition_ms } => Self {
                version: snapshot.version,
                mode: "black".to_owned(),
                duration_ms: *transition_ms,
                ..Self::default()
            },
            AudienceRenderState::Logo {
                fallback_title,
                transition_ms,
                ..
            } => Self {
                version: snapshot.version,
                mode: "logo".to_owned(),
                body: fallback_title.clone(),
                duration_ms: *transition_ms,
                ..Self::default()
            },
            AudienceRenderState::Slide {
                slide,
                show_text,
                alert,
                active_transition,
                transition_direction,
                ..
            } => {
                if !*show_text {
                    return Self {
                        version: snapshot.version,
                        mode: "clear".to_owned(),
                        duration_ms: active_transition.duration_ms(),
                        ..Self::default()
                    };
                }
                let (section_label, body, footer) =
                    extract_slide_copy(&slide.label, &slide.content);
                Self {
                    version: snapshot.version,
                    mode: "slide".to_owned(),
                    section_label,
                    body,
                    footer,
                    transition: active_transition.css_kind().to_owned(),
                    direction: match transition_direction {
                        TransitionDirection::Forward => "forward".to_owned(),
                        TransitionDirection::Backward => "backward".to_owned(),
                    },
                    duration_ms: active_transition.duration_ms(),
                    alert: alert.as_ref().map(|a| a.message.clone()),
                }
            }
        }
    }
}

fn extract_slide_copy(
    slide_label: &str,
    content: &SlideContent,
) -> (Option<String>, String, Option<String>) {
    match content {
        SlideContent::Text {
            body,
            section_label,
            footer,
            ..
        } => (
            section_label
                .clone()
                .or_else(|| Some(slide_label.to_owned())),
            body.clone(),
            footer.clone(),
        ),
        SlideContent::Custom {
            layout,
            revealed_steps,
        } => match layout {
            CustomSlideLayout::TitleSubtitle {
                title, subtitle, ..
            } => (
                Some(slide_label.to_owned()),
                title.clone(),
                Some(subtitle.clone()),
            ),
            CustomSlideLayout::BigStatement {
                statement,
                attribution,
                ..
            } => (
                Some(slide_label.to_owned()),
                statement.clone(),
                attribution.clone(),
            ),
            CustomSlideLayout::BulletPoints {
                heading,
                bullets,
                progressive_reveal,
            } => {
                let take_n = if *progressive_reveal {
                    revealed_steps.unwrap_or(1).clamp(1, bullets.len().max(1))
                } else {
                    bullets.len()
                };
                let joined = bullets
                    .iter()
                    .take(take_n)
                    .map(|b| format!("• {b}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                (Some(heading.clone()), joined, None)
            }
            CustomSlideLayout::ImageTextSplit { heading, body, .. } => {
                (Some(heading.clone()), body.clone(), None)
            }
            CustomSlideLayout::FullBleedImage { caption, .. } => {
                (None, caption.clone().unwrap_or_default(), None)
            }
            CustomSlideLayout::CountdownSlide {
                heading, subtitle, ..
            } => (
                Some(heading.clone()),
                subtitle.clone().unwrap_or_default(),
                None,
            ),
            CustomSlideLayout::LiturgyResponse { title, lines } => {
                let body = lines
                    .iter()
                    .map(|l| format!("{}: {}", l.speaker.label(), l.text))
                    .collect::<Vec<_>>()
                    .join("\n");
                (Some(title.clone()), body, None)
            }
        },
        SlideContent::Image { caption, .. } => (None, caption.clone().unwrap_or_default(), None),
        SlideContent::Blank => (None, String::new(), None),
    }
}

struct SharedOverlayState {
    latest_json: Mutex<String>,
    clients: Mutex<Vec<TcpStream>>,
    running: AtomicBool,
}

/// Non-blocking handle to the local OBS Browser Source HTTP + WebSocket overlay server.
#[derive(Clone)]
pub struct OverlayServerHandle {
    addr: SocketAddr,
    shared: Arc<SharedOverlayState>,
}

impl OverlayServerHandle {
    /// Starts the local OBS overlay server on `bind_addr` (e.g. `"127.0.0.1:8787"`).
    pub fn start(bind_addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr)?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;

        let initial_json = serde_json::to_string(&OverlayPayload::default())
            .unwrap_or_else(|_| r#"{"mode":"clear","body":""}"#.to_owned());
        let shared = Arc::new(SharedOverlayState {
            latest_json: Mutex::new(initial_json),
            clients: Mutex::new(Vec::new()),
            running: AtomicBool::new(true),
        });

        let accept_shared = Arc::clone(&shared);
        let _ = thread::Builder::new()
            .name("sundaydeck-overlay".to_owned())
            .spawn(move || accept_loop(listener, accept_shared));

        Ok(Self { addr, shared })
    }

    /// URL to paste directly into OBS Studio -> Sources -> Browser Source (`http://127.0.0.1:8787/overlay`).
    pub fn overlay_url(&self) -> String {
        format!("http://{}/overlay", self.addr)
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Publishes the latest [`OverlayPayload`] immediately to all connected OBS WebSocket clients.
    pub fn publish(&self, payload: &OverlayPayload) {
        let Ok(json) = serde_json::to_string(payload) else {
            return;
        };
        if let Ok(mut guard) = self.shared.latest_json.lock() {
            *guard = json.clone();
        }
        let frame = encode_ws_text_frame(&json);
        if let Ok(mut clients) = self.shared.clients.lock() {
            clients.retain_mut(|stream| stream.write_all(&frame).is_ok());
        }
    }

    /// Stops accepting new connections and drops connected overlay clients cleanly.
    pub fn shutdown(&self) {
        self.shared.running.store(false, Ordering::SeqCst);
        if let Ok(mut clients) = self.shared.clients.lock() {
            clients.clear();
        }
    }
}

fn accept_loop(listener: TcpListener, shared: Arc<SharedOverlayState>) {
    while shared.running.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                let s = Arc::clone(&shared);
                let _ = thread::Builder::new()
                    .name("sundaydeck-overlay-conn".to_owned())
                    .spawn(move || handle_connection(stream, s));
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) => break,
        }
    }
}

fn handle_connection(mut stream: TcpStream, shared: Arc<SharedOverlayState>) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let mut buf = [0u8; 4096];
    let Ok(n) = stream.read(&mut buf) else {
        return;
    };
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");

    if path == "/ws"
        && let Some(key) = find_header(&req, "sec-websocket-key")
    {
        let accept = websocket_accept_key(key);
        let handshake = format!(
            "HTTP/1.1 101 Switching Protocols\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        if stream.write_all(handshake.as_bytes()).is_ok()
            && let Ok(mut clients) = shared.clients.lock()
        {
            let latest = shared
                .latest_json
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default();
            if stream.write_all(&encode_ws_text_frame(&latest)).is_ok() {
                clients.push(stream);
            }
        }
        return;
    }

    match path {
        "/" | "/overlay" => {
            write_http_response(&mut stream, "text/html; charset=utf-8", OVERLAY_HTML)
        }
        "/state.json" => {
            let body = shared
                .latest_json
                .lock()
                .map(|g| g.clone())
                .unwrap_or_else(|_| "{}".to_owned());
            write_http_response(&mut stream, "application/json; charset=utf-8", &body);
        }
        "/health" => write_http_response(&mut stream, "text/plain; charset=utf-8", "ok"),
        _ => {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot Found",
            );
        }
    }
}

fn find_header<'a>(req: &'a str, needle_lower: &str) -> Option<&'a str> {
    for line in req.lines().skip(1) {
        if let Some((name, value)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case(needle_lower)
        {
            return Some(value.trim());
        }
    }
    None
}

fn write_http_response(stream: &mut TcpStream, content_type: &str, body: &str) {
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}
