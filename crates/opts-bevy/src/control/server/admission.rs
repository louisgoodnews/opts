//! Admission of control connections: the cap on simultaneous connections and the refusal of the
//! extra one (ISS-003; spec `docs/src/architecture/control-protocol.md` §1).

use std::io::Write;
use std::net::TcpStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use opts_cli::protocol::{code, Response, RpcError};
use serde_json::Value;

/// A connection that sends no complete line for this long is closed.
pub(super) const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// Connections served at once; the next one is refused with `1006 Busy`.
pub(super) const MAX_CONNECTIONS: usize = 8;

/// Counts the live connections. The listener thread takes a [`Slot`] per accepted socket; the slot
/// is released when the connection's thread ends, or when its thread cannot start.
#[derive(Default)]
pub(super) struct Slots(Arc<AtomicUsize>);

/// One live connection; dropping it frees the slot.
pub(super) struct Slot(Arc<AtomicUsize>);

impl Slots {
    /// A slot for a new connection, or `None` when [`MAX_CONNECTIONS`] are live.
    pub(super) fn take(&self) -> Option<Slot> {
        if self.0.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
            return None;
        }
        self.0.fetch_add(1, Ordering::SeqCst);
        Some(Slot(self.0.clone()))
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Tells a refused client why, then drops the socket.
pub(super) fn refuse(mut stream: TcpStream) {
    let reply = Response::err(
        Value::Null,
        RpcError::new(code::BUSY, "too many control connections"),
    );
    if let Ok(text) = serde_json::to_string(&reply) {
        let _ = writeln!(stream, "{text}");
    }
}
