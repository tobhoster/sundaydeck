//! Non-blocking fan-out of overlay frames to WebSocket clients.
//!
//! One writer thread owns every client socket write. Publishers only drop the newest encoded
//! frame into a latest-frame slot and wake the writer, so a stalled OBS source can never block
//! the caller. Frames offered while the writer is busy coalesce: only the newest is sent, which
//! is correct for a "current slide" overlay.

use std::io::{self, Write};
use std::net::TcpStream;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Per-write budget for an overlay client socket; a client that cannot take a frame within it
/// is dropped, so one frozen OBS source delays the others by at most this much, once.
pub(crate) const CLIENT_WRITE_TIMEOUT: Duration = Duration::from_millis(250);

const WRITER_THREAD_NAME: &str = "sundaydeck-overlay-writer";

/// Writer wake-up state. The stop flag shares the slot's mutex so a stop can never slip in
/// between the writer's check and its wait (no lost wake-up).
#[derive(Default)]
struct FrameSlot {
    latest_frame: Option<Vec<u8>>,
    stopping: bool,
}

/// Owns the connected WebSocket clients and the writer thread that fans frames out to them.
pub(crate) struct FrameBroadcaster {
    slot: Mutex<FrameSlot>,
    wake: Condvar,
    clients: Mutex<Vec<TcpStream>>,
    writer: Mutex<Option<JoinHandle<()>>>,
}

impl FrameBroadcaster {
    /// Creates the broadcaster and spawns its writer thread.
    pub(crate) fn start() -> io::Result<Arc<Self>> {
        let broadcaster = Arc::new(Self {
            slot: Mutex::new(FrameSlot::default()),
            wake: Condvar::new(),
            clients: Mutex::new(Vec::new()),
            writer: Mutex::new(None),
        });
        let worker = Arc::clone(&broadcaster);
        let writer = thread::Builder::new()
            .name(WRITER_THREAD_NAME.to_owned())
            .spawn(move || worker.run_writer())?;
        *lock_tolerating_poison(&broadcaster.writer) = Some(writer);
        Ok(broadcaster)
    }

    /// Replaces any frame the writer has not sent yet and wakes it. Never touches a socket.
    pub(crate) fn offer_frame(&self, frame: Vec<u8>) {
        lock_tolerating_poison(&self.slot).latest_frame = Some(frame);
        self.wake.notify_one();
    }

    /// Registers a client that already received the handshake and the current frame.
    pub(crate) fn add_client(&self, stream: TcpStream) {
        lock_tolerating_poison(&self.clients).push(stream);
    }

    /// Stops and joins the writer thread, then drops every client. Waits for at most the
    /// fan-out in flight (≤ [`CLIENT_WRITE_TIMEOUT`] per stalled client).
    pub(crate) fn shutdown(&self) {
        lock_tolerating_poison(&self.slot).stopping = true;
        self.wake.notify_all();
        let writer = lock_tolerating_poison(&self.writer).take();
        if let Some(writer) = writer {
            // A panicked writer has nothing left to flush; the clients are dropped below either way.
            let _ = writer.join();
        }
        lock_tolerating_poison(&self.clients).clear();
    }

    fn run_writer(&self) {
        while let Some(frame) = self.next_frame() {
            self.fan_out(&frame);
        }
    }

    /// Blocks until a frame is pending; `None` once shutdown was requested.
    fn next_frame(&self) -> Option<Vec<u8>> {
        let mut slot = self
            .wake
            .wait_while(lock_tolerating_poison(&self.slot), |slot| {
                !slot.stopping && slot.latest_frame.is_none()
            })
            .unwrap_or_else(PoisonError::into_inner);
        if slot.stopping {
            return None;
        }
        slot.latest_frame.take()
    }

    /// Writes `frame` to every client, dropping those whose write fails or times out.
    fn fan_out(&self, frame: &[u8]) {
        lock_tolerating_poison(&self.clients).retain_mut(|client| client.write_all(frame).is_ok());
    }
}

fn lock_tolerating_poison<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::time::Instant;

    use super::*;

    /// Bound for one fan-out past a stalled peer. A 32 MiB frame needs a few partial writes,
    /// each allowed the 250 ms write timeout (≈ 0.8 s in total), so 1.9 s leaves room for
    /// scheduling under load while a 2 s write timeout (≈ 6 s for the same frame) still fails.
    const STALLED_FAN_OUT_BUDGET: Duration = Duration::from_millis(1_900);
    /// Larger than any loopback socket buffer, so a peer that never reads stalls the write.
    const FRAME_BYTES: usize = 32 * 1024 * 1024;

    #[test]
    fn fan_out_drops_a_peer_that_never_reads_within_the_write_timeout() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let peer = TcpStream::connect(listener.local_addr().expect("local addr")).expect("connect");
        let (client, _) = listener.accept().expect("accept");
        // The same socket option the server applies to every overlay client.
        client
            .set_write_timeout(Some(CLIENT_WRITE_TIMEOUT))
            .expect("write timeout");
        let broadcaster = FrameBroadcaster {
            slot: Mutex::new(FrameSlot::default()),
            wake: Condvar::new(),
            clients: Mutex::new(vec![client]),
            writer: Mutex::new(None),
        };

        let frame = vec![0u8; FRAME_BYTES];
        let started = Instant::now();
        broadcaster.fan_out(&frame);
        let elapsed = started.elapsed();

        assert!(
            elapsed < STALLED_FAN_OUT_BUDGET,
            "fan-out stalled for {elapsed:?} behind a peer that never reads"
        );
        assert!(
            lock_tolerating_poison(&broadcaster.clients).is_empty(),
            "the stalled peer is dropped"
        );
        drop(peer);
    }
}
