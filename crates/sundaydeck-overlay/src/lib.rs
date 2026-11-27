mod broadcast;
pub mod control;
mod html;
mod remote_html;
mod ws;

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use async_channel::{Receiver, Sender, bounded};
use serde::{Deserialize, Serialize};
use sundaydeck_core::{
    AudienceRenderState, CustomSlideLayout, PresentationSnapshot, SlideContent, TransitionDirection,
};

use crate::broadcast::{CLIENT_WRITE_TIMEOUT, FrameBroadcaster};
pub use crate::control::RemoteControlCommand;
use crate::control::{build_api_status_json, parse_trigger_request};
use crate::html::OVERLAY_HTML;
use crate::remote_html::REMOTE_HTML;
use crate::ws::{encode_ws_text_frame, websocket_accept_key};

pub const DEFAULT_OVERLAY_BIND_ADDR: &str = "127.0.0.1:8787";
const CONTROL_QUEUE_CAPACITY: usize = 64;

/// Read budget for a client's HTTP request line and headers.
const REQUEST_READ_TIMEOUT: Duration = Duration::from_secs(2);
/// Fixed request buffer; no client-controlled length is ever read.
const REQUEST_BUFFER_BYTES: usize = 4096;

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
    broadcaster: Arc<FrameBroadcaster>,
    control_tx: Sender<RemoteControlCommand>,
    running: AtomicBool,
}

/// Non-blocking handle to the local OBS Browser Source HTTP + WebSocket overlay & Companion API.
#[derive(Clone)]
pub struct OverlayServerHandle {
    addr: SocketAddr,
    shared: Arc<SharedOverlayState>,
    control_rx: Receiver<RemoteControlCommand>,
}

impl OverlayServerHandle {
    /// Starts the local OBS overlay + Booth Remote / Companion API server on `bind_addr`.
    pub fn start(bind_addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(bind_addr)?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;

        let initial_json = serde_json::to_string(&OverlayPayload::default())
            .unwrap_or_else(|_| r#"{"mode":"clear","body":""}"#.to_owned());
        let (control_tx, control_rx) = bounded(CONTROL_QUEUE_CAPACITY);
        let shared = Arc::new(SharedOverlayState {
            latest_json: Mutex::new(initial_json),
            broadcaster: FrameBroadcaster::start()?,
            control_tx,
            running: AtomicBool::new(true),
        });

        let accept_shared = Arc::clone(&shared);
        let _ = thread::Builder::new()
            .name("sundaydeck-overlay".to_owned())
            .spawn(move || accept_loop(listener, accept_shared));

        Ok(Self {
            addr,
            shared,
            control_rx,
        })
    }

    /// URL to paste directly into OBS Studio -> Sources -> Browser Source (`http://127.0.0.1:8787/overlay`).
    pub fn overlay_url(&self) -> String {
        format!("http://{}/overlay", self.addr)
    }

    /// URL for the built-in touch Booth Remote & Bitfocus Companion / Stream Deck API info.
    pub fn remote_url(&self) -> String {
        format!("http://{}/remote", self.addr)
    }

    /// Async receiver for incoming Stream Deck / Companion / Booth Remote commands.
    pub fn control_receiver(&self) -> Receiver<RemoteControlCommand> {
        self.control_rx.clone()
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Publishes the latest [`OverlayPayload`] to all connected OBS WebSocket clients without
    /// waiting on any of them: the frame is handed to the broadcaster's writer thread.
    pub fn publish(&self, payload: &OverlayPayload) {
        // `OverlayPayload` is plain strings and numbers; a failed serialize has nothing to send.
        let Ok(json) = serde_json::to_string(payload) else {
            return;
        };
        let frame = encode_ws_text_frame(&json);
        // Offered under the `latest_json` lock so the pending frame always matches the newest state.
        if let Ok(mut latest) = self.shared.latest_json.lock() {
            *latest = json;
            self.shared.broadcaster.offer_frame(frame);
        }
    }

    /// Stops accepting new connections, stops the frame writer and drops connected overlay clients.
    pub fn shutdown(&self) {
        self.shared.running.store(false, Ordering::SeqCst);
        self.shared.control_tx.close();
        self.shared.broadcaster.shutdown();
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
    // Best-effort socket options: on failure the OS defaults apply and the request is still served.
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(REQUEST_READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(CLIENT_WRITE_TIMEOUT));

    let mut buf = [0u8; REQUEST_BUFFER_BYTES];
    let Ok(n) = stream.read(&mut buf) else {
        return;
    };
    let req = String::from_utf8_lossy(&buf[..n]);
    let mut parts = req.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let body = req.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or("");

    if path == "/ws"
        && let Some(key) = find_header(&req, "sec-websocket-key")
    {
        accept_websocket_client(stream, key, &shared);
        return;
    }
    serve_http(&mut stream, method, path, body, &shared);
}

/// Completes the RFC 6455 upgrade and sends the current state *before* registering the client,
/// so a slow handshake never holds the client list the writer thread fans out over.
fn accept_websocket_client(mut stream: TcpStream, key: &str, shared: &SharedOverlayState) {
    let accept = websocket_accept_key(key);
    let handshake = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    // A client that cannot take the handshake or first frame is dropped: OBS reconnects on its own.
    if stream.write_all(handshake.as_bytes()).is_err() {
        return;
    }
    let sent_json = shared
        .latest_json
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();
    if stream.write_all(&encode_ws_text_frame(&sent_json)).is_err() {
        return;
    }
    shared.broadcaster.add_client(stream);
    offer_latest_if_published_since(shared, &sent_json);
}

/// A `publish` that landed between this client's first frame and its registration fanned out
/// without it; re-offer the newest state so the writer delivers it (others get an idempotent
/// repeat). Offered under the `latest_json` lock, like `publish`, so the pending frame never
/// regresses to an older state.
fn offer_latest_if_published_since(shared: &SharedOverlayState, sent_json: &str) {
    if let Ok(latest) = shared.latest_json.lock()
        && latest.as_str() != sent_json
    {
        shared
            .broadcaster
            .offer_frame(encode_ws_text_frame(&latest));
    }
}

fn serve_http(
    stream: &mut TcpStream,
    method: &str,
    path: &str,
    body: &str,
    shared: &SharedOverlayState,
) {
    if path.starts_with("/api/trigger/") {
        match parse_trigger_request(method, path, body) {
            Ok(cmd) => match shared.control_tx.try_send(cmd) {
                Ok(()) => write_http_response(
                    stream,
                    "200 OK",
                    "application/json; charset=utf-8",
                    r#"{"ok":true}"#,
                ),
                Err(_) => write_http_response(
                    stream,
                    "503 Service Unavailable",
                    "application/json; charset=utf-8",
                    r#"{"ok":false,"error":"control queue full or closed"}"#,
                ),
            },
            Err((status, msg)) => {
                let err_json = format!(r#"{{"ok":false,"error":"{msg}"}}"#);
                write_http_response(stream, status, "application/json; charset=utf-8", &err_json);
            }
        }
        return;
    }

    match path {
        "/" | "/overlay" => {
            write_http_response(stream, "200 OK", "text/html; charset=utf-8", OVERLAY_HTML)
        }
        "/remote" => write_http_response(stream, "200 OK", "text/html; charset=utf-8", REMOTE_HTML),
        "/state.json" => {
            let body = shared
                .latest_json
                .lock()
                .map(|g| g.clone())
                .unwrap_or_else(|_| "{}".to_owned());
            write_http_response(stream, "200 OK", "application/json; charset=utf-8", &body);
        }
        "/api/status" => {
            let latest = shared
                .latest_json
                .lock()
                .map(|g| g.clone())
                .unwrap_or_else(|_| "{}".to_owned());
            let payload = build_api_status_json(&latest);
            write_http_response(
                stream,
                "200 OK",
                "application/json; charset=utf-8",
                &payload,
            );
        }
        "/health" => write_http_response(stream, "200 OK", "text/plain; charset=utf-8", "ok"),
        _ => {
            write_http_response(
                stream,
                "404 Not Found",
                "text/plain; charset=utf-8",
                "Not Found",
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

fn write_http_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let header = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}
