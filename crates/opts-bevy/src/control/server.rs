//! The TCP side of the control server. Runs entirely on `std` threads and talks to the main
//! thread through a channel (see `docs/src/architecture/control-protocol.md` §1).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use opts_cli::protocol::{
    code, parse_line, Incoming, Request, Response, RpcError, MAX_BATCH, MAX_LINE_BYTES,
};
use serde_json::{json, Value};

use super::state::{Pending, PendingFlags};

/// Seconds the app has to pick a request up before it fails with `1005`.
const START_TIMEOUT: Duration = Duration::from_secs(5);
/// Seconds a started (possibly multi-frame) command may take.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(125);
/// Requests per second and burst of the per-connection limiter.
const RATE_PER_SEC: f64 = 200.0;
const RATE_BURST: f64 = 400.0;
/// Capacity of the command queue.
pub const QUEUE_CAPACITY: usize = 1024;

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

fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

/// Starts the listener thread.
///
/// # Errors
/// An `io::Error` when the address is invalid, not loopback without a token, or cannot be bound.
pub fn start(config: &ServerConfig, tx: SyncSender<Pending>) -> std::io::Result<ServerHandle> {
    let ip: IpAddr = config.bind.parse().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("bad --cli-bind: {e}"),
        )
    })?;
    if !ip.is_loopback() && config.token.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "binding a non-loopback address requires --cli-token",
        ));
    }
    let listener = TcpListener::bind((ip, config.port))?;
    let addr = listener.local_addr()?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let token = config.token.clone();
    std::thread::Builder::new()
        .name("control-listener".into())
        .spawn(move || {
            for stream in listener.incoming() {
                if flag.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let (tx, token, flag) = (tx.clone(), token.clone(), flag.clone());
                let spawned = std::thread::Builder::new()
                    .name("control-conn".into())
                    .spawn(move || serve_connection(stream, &tx, token.as_deref(), &flag));
                if let Err(e) = spawned {
                    tracing::warn!(target: "opts_bevy::control", "cannot start connection thread: {e}");
                }
            }
        })?;
    Ok(ServerHandle { addr, stop })
}

struct Limiter {
    tokens: f64,
    last: Instant,
}

impl Limiter {
    fn new() -> Self {
        Self {
            tokens: RATE_BURST,
            last: Instant::now(),
        }
    }

    fn allow(&mut self) -> bool {
        let now = Instant::now();
        self.tokens = (self.tokens + now.duration_since(self.last).as_secs_f64() * RATE_PER_SEC)
            .min(RATE_BURST);
        self.last = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

fn read_line(reader: &mut BufReader<TcpStream>) -> Option<Result<String, RpcError>> {
    let mut buf = String::new();
    match reader
        .by_ref()
        .take(MAX_LINE_BYTES as u64 + 1)
        .read_line(&mut buf)
    {
        Ok(0) | Err(_) => None,
        Ok(n) if n > MAX_LINE_BYTES => {
            Some(Err(RpcError::new(code::INVALID_REQUEST, "line too long")))
        }
        Ok(_) => Some(Ok(buf.trim_end_matches(['\n', '\r']).to_string())),
    }
}

fn serve_connection(
    stream: TcpStream,
    tx: &SyncSender<Pending>,
    token: Option<&str>,
    stop: &AtomicBool,
) {
    let _ = stream.set_nodelay(true);
    let Ok(read_half) = stream.try_clone() else {
        return;
    };
    let mut writer = stream;
    let mut reader = BufReader::new(read_half);
    let mut authed = token.is_none();
    let mut limiter = Limiter::new();
    while !stop.load(Ordering::SeqCst) {
        let Some(line) = read_line(&mut reader) else {
            break;
        };
        let reply = match line.and_then(|l| parse_line(&l)) {
            Err(e) => Some(serde_json::to_string(&Response::err(Value::Null, e))),
            Ok(Incoming::Single(req)) => handle_single(req, tx, token, &mut authed, &mut limiter)
                .map(|r| serde_json::to_string(&r)),
            Ok(Incoming::Batch(items)) => Some(serde_json::to_string(&handle_batch(
                items,
                tx,
                token,
                &mut authed,
                &mut limiter,
            ))),
        };
        if let Some(Ok(text)) = reply {
            if writeln!(writer, "{text}").is_err() || writer.flush().is_err() {
                break;
            }
        }
    }
}

fn gate(
    req: &Request,
    token: Option<&str>,
    authed: &mut bool,
    limiter: &mut Limiter,
) -> Option<RpcError> {
    if req.method == "auth" {
        return None;
    }
    if !*authed {
        return Some(RpcError::new(
            code::UNAUTHORIZED,
            "send `auth` with the token first",
        ));
    }
    let _ = token;
    if limiter.allow() {
        None
    } else {
        Some(RpcError::new(code::BUSY, "rate limit exceeded"))
    }
}

fn auth_reply(req: &Request, token: Option<&str>, authed: &mut bool) -> Response {
    let id = req.id.clone().unwrap_or(Value::Null);
    match token {
        None => Response::ok(id, json!({ "ok": true })),
        Some(t) => match req.params.get("token").and_then(Value::as_str) {
            Some(given) if constant_time_eq(given, t) => {
                *authed = true;
                Response::ok(id, json!({ "ok": true }))
            }
            _ => Response::err(id, RpcError::new(code::UNAUTHORIZED, "wrong token")),
        },
    }
}

/// Queues one request; the returned receiver yields its answer.
fn submit(
    req: Request,
    tx: &SyncSender<Pending>,
) -> Result<(Receiver<Response>, Arc<PendingFlags>), RpcError> {
    let (reply, rx) = mpsc::channel();
    let flags = Arc::new(PendingFlags::default());
    tx.try_send(Pending {
        request: req,
        reply,
        flags: flags.clone(),
    })
    .map_err(|e| match e {
        mpsc::TrySendError::Full(_) => RpcError::new(code::BUSY, "command queue is full"),
        mpsc::TrySendError::Disconnected(_) => {
            RpcError::new(code::INTERNAL_ERROR, "the application is shutting down")
        }
    })?;
    Ok((rx, flags))
}

fn await_reply(rx: &Receiver<Response>, flags: &PendingFlags, id: &Value) -> Response {
    let began = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(r) => return r,
            Err(RecvTimeoutError::Timeout) => {
                let waited = began.elapsed();
                let stalled = !flags.started.load(Ordering::SeqCst) && waited > START_TIMEOUT;
                if stalled || waited > TOTAL_TIMEOUT {
                    flags.cancelled.store(true, Ordering::SeqCst);
                    return Response::err(
                        id.clone(),
                        RpcError::new(code::TIMEOUT, "the application did not answer in time"),
                    );
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Response::err(
                    id.clone(),
                    RpcError::new(code::INTERNAL_ERROR, "the application dropped the request"),
                );
            }
        }
    }
}

fn handle_single(
    req: Request,
    tx: &SyncSender<Pending>,
    token: Option<&str>,
    authed: &mut bool,
    limiter: &mut Limiter,
) -> Option<Response> {
    let id = req.id.clone();
    if req.method == "auth" {
        let r = auth_reply(&req, token, authed);
        return id.map(|_| r);
    }
    let reply_id = id.clone().unwrap_or(Value::Null);
    if let Some(e) = gate(&req, token, authed, limiter) {
        return id.map(|_| Response::err(reply_id, e));
    }
    match submit(req, tx) {
        Err(e) => id.map(|_| Response::err(reply_id, e)),
        Ok((rx, flags)) => id.map(|_| await_reply(&rx, &flags, &reply_id)),
    }
}

fn handle_batch(
    items: Vec<Result<Request, RpcError>>,
    tx: &SyncSender<Pending>,
    token: Option<&str>,
    authed: &mut bool,
    limiter: &mut Limiter,
) -> Vec<Response> {
    if items.len() > MAX_BATCH {
        let e = RpcError::new(
            code::INVALID_REQUEST,
            format!("at most {MAX_BATCH} requests per batch"),
        );
        return vec![Response::err(Value::Null, e)];
    }
    enum Slot {
        Done(Response),
        Waiting(Receiver<Response>, Arc<PendingFlags>, Value),
        Silent,
    }
    let mut slots = Vec::new();
    for item in items {
        let req = match item {
            Ok(r) => r,
            Err(e) => {
                slots.push(Slot::Done(Response::err(Value::Null, e)));
                continue;
            }
        };
        let id = req.id.clone();
        let reply_id = id.clone().unwrap_or(Value::Null);
        if req.method == "auth" {
            let r = auth_reply(&req, token, authed);
            slots.push(if id.is_some() {
                Slot::Done(r)
            } else {
                Slot::Silent
            });
        } else if let Some(e) = gate(&req, token, authed, limiter) {
            slots.push(if id.is_some() {
                Slot::Done(Response::err(reply_id, e))
            } else {
                Slot::Silent
            });
        } else {
            match submit(req, tx) {
                Ok((rx, flags)) if id.is_some() => slots.push(Slot::Waiting(rx, flags, reply_id)),
                Ok(_) => slots.push(Slot::Silent),
                Err(e) => slots.push(if id.is_some() {
                    Slot::Done(Response::err(reply_id, e))
                } else {
                    Slot::Silent
                }),
            }
        }
    }
    slots
        .into_iter()
        .filter_map(|s| match s {
            Slot::Done(r) => Some(r),
            Slot::Waiting(rx, flags, id) => Some(await_reply(&rx, &flags, &id)),
            Slot::Silent => None,
        })
        .collect()
}

/// A sender-side handle kept for tests that need to inject without a socket.
pub type CommandSender = Sender<Pending>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_comparison_matches_equality() {
        assert!(constant_time_eq("secret", "secret"));
        assert!(!constant_time_eq("secret", "secreT"));
        assert!(!constant_time_eq("secret", "secrets"));
        assert!(!constant_time_eq("", "x"));
    }

    #[test]
    fn limiter_allows_a_burst_then_refuses() {
        let mut l = Limiter::new();
        let allowed = (0..1000).filter(|_| l.allow()).count();
        assert!((400..=420).contains(&allowed), "{allowed}");
    }

    #[test]
    fn non_loopback_binds_need_a_token() {
        let (tx, _rx) = mpsc::sync_channel(1);
        let cfg = ServerConfig {
            bind: "0.0.0.0".into(),
            port: 0,
            token: None,
        };
        assert_eq!(
            start(&cfg, tx).err().map(|e| e.kind()),
            Some(std::io::ErrorKind::PermissionDenied)
        );
    }
}
