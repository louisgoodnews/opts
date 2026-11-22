//! Admission of control connections: the cap on simultaneous connections, the refusal of the
//! extra one (ISS-003) and the close of every open connection when the server stops (ISS-003;
//! spec `docs/src/architecture/control-protocol.md` §1).

use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use opts_cli::protocol::{code, Response, RpcError};
use serde_json::Value;

/// A connection that sends no complete line for this long is closed.
pub(super) const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// Connections served at once; the next one is refused with `1006 Busy`.
pub(super) const MAX_CONNECTIONS: usize = 8;
/// Longest wait for the connection threads to end once the server has stopped.
pub(super) const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);

/// The live connections: a clone of each socket, so that stopping can close it.
#[derive(Default)]
struct Live {
    closed: bool,
    next: u64,
    sockets: Vec<(u64, TcpStream)>,
}

/// Counts the live connections. The listener thread takes a [`Slot`] per accepted socket; the slot
/// is released when the connection's thread ends, or when its thread cannot start.
#[derive(Clone, Default)]
pub(super) struct Slots(Arc<Mutex<Live>>);

/// One live connection; dropping it frees the slot.
pub(super) struct Slot {
    id: u64,
    live: Arc<Mutex<Live>>,
}

impl Slots {
    fn lock(&self) -> MutexGuard<'_, Live> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A slot for a connection, or `None` when [`MAX_CONNECTIONS`] are live or the server is closing.
    pub(super) fn take(&self, socket: &TcpStream) -> Option<Slot> {
        let mut live = self.lock();
        if live.closed || live.sockets.len() >= MAX_CONNECTIONS {
            return None;
        }
        let clone = socket.try_clone().ok()?;
        live.next += 1;
        let id = live.next;
        live.sockets.push((id, clone));
        Some(Slot {
            id,
            live: self.0.clone(),
        })
    }

    /// Number of live connections.
    #[cfg(test)]
    pub(super) fn count(&self) -> usize {
        self.lock().sockets.len()
    }

    /// Closes every open connection (the clients read end-of-file), refuses new ones, and waits up
    /// to [`SHUTDOWN_WAIT`] for the connection threads to end.
    pub(super) fn close(&self) {
        {
            let mut live = self.lock();
            live.closed = true;
            for (_, socket) in &live.sockets {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
        let deadline = Instant::now() + SHUTDOWN_WAIT;
        loop {
            if self.lock().sockets.is_empty() || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        let id = self.id;
        live.sockets.retain(|(i, _)| *i != id);
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
