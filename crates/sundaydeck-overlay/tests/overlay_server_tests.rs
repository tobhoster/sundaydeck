use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

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

    let mut received = String::new();
    let mut chunk = [0u8; 2048];
    // Wait until both the HTTP 101 handshake and the initial clear state frame arrive.
    while !received.contains("\"mode\":\"clear\"") {
        let n = ws
            .read(&mut chunk)
            .expect("read ws handshake and initial frame");
        assert!(n > 0, "unexpected EOF waiting for ws handshake");
        received.push_str(&String::from_utf8_lossy(&chunk[..n]));
    }
    assert!(received.contains("101 Switching Protocols"));
    assert!(received.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));

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

    let mut pushed = String::new();
    while !pushed.contains("It is well with my soul") {
        let m = ws.read(&mut chunk).expect("read pushed ws frame");
        assert!(m > 0, "unexpected EOF waiting for pushed ws frame");
        pushed.push_str(&String::from_utf8_lossy(&chunk[..m]));
    }
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

const WS_UPGRADE_REQUEST: &[u8] = b"GET /ws HTTP/1.1\r\n\
    Host: localhost\r\n\
    Upgrade: websocket\r\n\
    Connection: Upgrade\r\n\
    Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
    Sec-WebSocket-Version: 13\r\n\r\n";

/// Upgrades a fresh connection to `/ws`, consumes the 101 headers and the initial state frame,
/// and returns the reader positioned at the next server frame.
fn open_ws_client(addr: SocketAddr) -> BufReader<TcpStream> {
    let mut stream = TcpStream::connect(addr).expect("connect ws");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("ws read timeout");
    stream
        .write_all(WS_UPGRADE_REQUEST)
        .expect("send ws upgrade");
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read ws status line");
    assert!(line.contains("101 Switching Protocols"), "got {line:?}");
    while line != "\r\n" {
        line.clear();
        let n = reader.read_line(&mut line).expect("read ws header line");
        assert!(n > 0, "unexpected EOF inside ws handshake headers");
    }
    let mut initial = Vec::new();
    read_ws_text_frame(&mut reader, &mut initial).expect("read initial ws frame");
    reader
}

/// Reads one unmasked server-to-client RFC 6455 frame payload into `payload`.
fn read_ws_text_frame(reader: &mut impl Read, payload: &mut Vec<u8>) -> io::Result<()> {
    let mut head = [0u8; 2];
    reader.read_exact(&mut head)?;
    let len = match head[1] & 0x7f {
        126 => {
            let mut ext = [0u8; 2];
            reader.read_exact(&mut ext)?;
            u64::from(u16::from_be_bytes(ext))
        }
        127 => {
            let mut ext = [0u8; 8];
            reader.read_exact(&mut ext)?;
            u64::from_be_bytes(ext)
        }
        short => u64::from(short),
    };
    let len = usize::try_from(len).map_err(|_| io::Error::other("ws frame too large"))?;
    payload.resize(len, 0);
    reader.read_exact(payload)
}

/// AC1.7 reading: `publish` never waits on a client. The typical (median) call stays under
/// 50 ms, and no single call reaches 250 ms, the length of one client write timeout: base
/// blocked ≥ 2 s per call once the stalled client's buffers filled, and a regression back to
/// writing synchronously (several 250 ms timeouts per frame) trips it too. A lone call slowed
/// by CPU scheduling under heavy load stays far below 250 ms; the median guards typical cost.
#[test]
fn publish_never_blocks_on_a_stalled_ws_client_and_healthy_client_gets_final_frame() {
    const PUBLISH_COUNT: u64 = 2_000;
    const MAX_MEDIAN_PUBLISH: Duration = Duration::from_millis(50);
    const MAX_SINGLE_PUBLISH: Duration = Duration::from_millis(250);
    const FINAL_DELIVERY: Duration = Duration::from_secs(1);

    let server = OverlayServerHandle::start("127.0.0.1:0").expect("start overlay server");
    let addr = server.local_addr();

    // Completes the upgrade, then never reads again so its socket buffers fill up.
    let stalled = open_ws_client(addr);

    let final_prefix = format!("{{\"version\":{},", PUBLISH_COUNT - 1).into_bytes();
    let mut healthy = open_ws_client(addr);
    let (final_tx, final_rx) = mpsc::channel::<()>();
    let healthy_reader = thread::spawn(move || {
        let mut payload = Vec::new();
        while read_ws_text_frame(&mut healthy, &mut payload).is_ok() {
            if payload.starts_with(&final_prefix) {
                // The receiver is gone only if the test already failed on its 1 s deadline.
                let _ = final_tx.send(());
                return;
            }
        }
    });

    let mut payload = OverlayPayload {
        mode: "slide".to_owned(),
        body: "x".repeat(64 * 1024),
        ..OverlayPayload::default()
    };
    let mut publish_times = Vec::with_capacity(PUBLISH_COUNT as usize);
    for seq in 0..PUBLISH_COUNT {
        payload.version = seq;
        let started = Instant::now();
        server.publish(&payload);
        let took = started.elapsed();
        assert!(
            took < MAX_SINGLE_PUBLISH,
            "publish #{seq} blocked for {took:?} behind a stalled ws client \
             (limit {MAX_SINGLE_PUBLISH:?})"
        );
        publish_times.push(took);
    }
    publish_times.sort_unstable();
    let median = publish_times[publish_times.len() / 2];
    assert!(
        median < MAX_MEDIAN_PUBLISH,
        "median publish took {median:?} with a stalled ws client (limit {MAX_MEDIAN_PUBLISH:?})"
    );

    final_rx.recv_timeout(FINAL_DELIVERY).expect(
        "healthy ws client did not receive the final payload within 1 s of the last publish",
    );

    server.shutdown();
    drop(stalled);
    healthy_reader.join().expect("healthy reader thread");
}
