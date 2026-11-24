use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use sundaydeck_overlay::{OverlayPayload, OverlayServerHandle};

#[test]
fn obs_overlay_serves_transparent_html_state_json_and_pushes_websocket_frames() {
    let server = OverlayServerHandle::start("127.0.0.1:0").expect("start overlay server");
    let addr = server.local_addr();
    assert!(server.overlay_url().ends_with("/overlay"));

    // 1. Verify GET /overlay returns transparent lower-third HTML
    let mut http = TcpStream::connect(addr).expect("connect http");
    http.write_all(b"GET /overlay HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("send GET /overlay");
    let mut html_resp = String::new();
    http.read_to_string(&mut html_resp).expect("read html");
    assert!(html_resp.contains("200 OK"));
    assert!(html_resp.contains("background: transparent"));
    assert!(html_resp.contains("SundayDeck — OBS Lower-Third Overlay"));

    // 2. Verify real RFC 6455 WebSocket upgrade receives initial + pushed live slide frames
    let mut ws = TcpStream::connect(addr).expect("connect ws");
    ws.set_read_timeout(Some(Duration::from_secs(2)))
        .expect("timeout");
    ws.write_all(
        b"GET /ws HTTP/1.1\r\n\
          Host: localhost\r\n\
          Upgrade: websocket\r\n\
          Connection: Upgrade\r\n\
          Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
          Sec-WebSocket-Version: 13\r\n\r\n",
    )
    .expect("send ws upgrade");

    let mut initial_buf = [0u8; 2048];
    let n = ws.read(&mut initial_buf).expect("read ws handshake");
    let head = String::from_utf8_lossy(&initial_buf[..n]);
    assert!(head.contains("101 Switching Protocols"));
    assert!(head.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));

    // Push a live worship slide update and assert the WS client receives it immediately
    server.publish(&OverlayPayload {
        version: 42,
        mode: "slide".to_owned(),
        section_label: Some("Chorus".to_owned()),
        body: "It is well with my soul".to_owned(),
        footer: Some("Horatio G. Spafford".to_owned()),
        transition: "soft_rise".to_owned(),
        direction: "forward".to_owned(),
        duration_ms: 260,
        alert: None,
    });

    let mut push_buf = [0u8; 2048];
    let m = ws.read(&mut push_buf).expect("read pushed ws frame");
    let pushed = String::from_utf8_lossy(&push_buf[..m]);
    assert!(pushed.contains("It is well with my soul"));
    assert!(pushed.contains("\"transition\":\"soft_rise\""));

    // 3. Verify GET /state.json reflects the pushed slide
    let mut state_conn = TcpStream::connect(addr).expect("connect state");
    state_conn
        .write_all(b"GET /state.json HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("send GET /state.json");
    let mut state_resp = String::new();
    state_conn
        .read_to_string(&mut state_resp)
        .expect("read state.json");
    assert!(state_resp.contains("It is well with my soul"));

    server.shutdown();
}
