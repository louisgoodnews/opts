//! The TCP side of the control server. Runs entirely on `std` threads and talks to the main
//! thread through a channel (see `docs/src/architecture/control-protocol.md` §1).

use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::mpsc;
use std::sync::mpsc::{Sender, SyncSender};
use std::sync::Arc;
use std::time::Duration;

use super::state::Pending;

mod admission;
mod auth;
mod connection;
mod dispatch;
mod limiter;
mod throttle;

use self::admission::{refuse, Slots, READ_TIMEOUT};
#[cfg(test)]
use self::auth::constant_time_eq;
use self::connection::serve_connection;
#[cfg(test)]
use self::limiter::Limiter;
use self::throttle::AuthThrottle;

/// Capacity of the command queue.
pub const QUEUE_CAPACITY: usize = 1024;

/// Shortest token the server accepts (audit PRJ-02: at least 32 random characters).
pub const MIN_TOKEN_CHARS: usize = 32;

/// Server settings.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind (default loopback).
    pub bind: String,
    /// Port (`0` = any free port).
    pub port: u16,
    /// Token clients must present with `auth`.
    pub token: Option<String>,
}

/// A running server; dropping it stops accepting connections.
pub struct ServerHandle {
    /// The bound address (useful with port 0).
    pub addr: SocketAddr,
    stop: Arc<AtomicBool>,
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_millis(200));
    }
}

/// Starts the listener thread.
///
/// # Errors
/// An `io::Error` when the address is invalid, no token is set (on every bind, loopback included), the
/// token is shorter than [`MIN_TOKEN_CHARS`] (an empty token included), or the address cannot be bound.
pub fn start(config: &ServerConfig, tx: SyncSender<Pending>) -> std::io::Result<ServerHandle> {
    if let Some(t) = &config.token {
        if t.chars().count() < MIN_TOKEN_CHARS {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("the token must be at least {MIN_TOKEN_CHARS} characters"),
            ));
        }
    }
    let ip: IpAddr = config.bind.parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("bad --cli-bind: {e}"),
        )
    })?;
    if config.token.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "the control server requires a token: set OPTS_CLI_TOKEN or --cli-token-file",
        ));
    }
    let listener = TcpListener::bind((ip, config.port))?;
    let addr = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let token = config.token.clone();
    let throttle = Arc::new(AuthThrottle::default());
    std::thread::Builder::new()
        .name("control-listener".into())
        .spawn(move || {
            let slots = Slots::default();
            for stream in listener.incoming() {
                if flag.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let Some(slot) = slots.take() else {
                    refuse(stream);
                    continue;
                };
                let (tx, token, flag) = (tx.clone(), token.clone(), flag.clone());
                let throttle = throttle.clone();
                let spawned = std::thread::Builder::new()
                    .name("control-conn".into())
                    .spawn(move || {
                        let _slot = slot;
                        serve_connection(
                            stream,
                            &tx,
                            token.as_deref(),
                            &flag,
                            &throttle,
                            READ_TIMEOUT,
                        );
                    });
                if let Err(e) = spawned {
                    tracing::warn!(target: "opts_bevy::control", "cannot start connection thread: {e}");
                }
            }
        })?;
    Ok(ServerHandle { addr, stop })
}

/// A sender-side handle kept for tests that need to inject without a socket.
pub type CommandSender = Sender<Pending>;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_admission;
#[cfg(test)]
mod tests_throttle;
