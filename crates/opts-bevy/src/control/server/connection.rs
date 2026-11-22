//! Socket read loop of one control connection.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use opts_cli::protocol::{code, parse_line, Incoming, Response, RpcError, MAX_LINE_BYTES};
use serde_json::Value;

use super::dispatch::{handle_batch, handle_single};
use super::limiter::Limiter;
use super::throttle::{AuthThrottle, Caller};
use crate::control::state::Pending;

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

pub(super) fn serve_connection(
    stream: TcpStream,
    tx: &SyncSender<Pending>,
    token: Option<&str>,
    stop: &AtomicBool,
    throttle: &AuthThrottle,
    idle: Duration,
) {
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(idle));
    let addr = stream
        .peer_addr()
        .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |a| a.ip());
    let caller = Caller { addr, throttle };
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
            Ok(Incoming::Single(req)) => {
                handle_single(req, tx, token, &mut authed, &mut limiter, caller)
                    .map(|r| serde_json::to_string(&r))
            }
            Ok(Incoming::Batch(items)) => Some(serde_json::to_string(&handle_batch(
                items,
                tx,
                token,
                &mut authed,
                &mut limiter,
                caller,
            ))),
        };
        if let Some(Ok(text)) = reply {
            if writeln!(writer, "{text}").is_err() || writer.flush().is_err() {
                break;
            }
        }
    }
}
