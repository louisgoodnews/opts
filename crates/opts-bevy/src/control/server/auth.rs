//! Authentication and admission gate for incoming control requests.

use std::time::Instant;

use opts_cli::protocol::{code, Request, Response, RpcError};
use serde_json::{json, Value};

use super::limiter::Limiter;
use super::throttle::Caller;

pub(super) fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

pub(super) fn gate(
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

pub(super) fn auth_reply(
    req: &Request,
    token: Option<&str>,
    authed: &mut bool,
    caller: Caller<'_>,
) -> Response {
    let id = req.id.clone().unwrap_or(Value::Null);
    let Some(t) = token else {
        return Response::ok(id, json!({ "ok": true }));
    };
    let now = Instant::now();
    if let Err(wait) = caller.throttle.check(caller.addr, now) {
        let msg = format!(
            "too many failed attempts; retry in {} s",
            wait.as_secs() + 1
        );
        return Response::err(id, RpcError::new(code::BUSY, msg));
    }
    match req.params.get("token").and_then(Value::as_str) {
        Some(given) if constant_time_eq(given, t) => {
            *authed = true;
            caller.throttle.succeeded(caller.addr);
            Response::ok(id, json!({ "ok": true }))
        }
        _ => {
            caller.throttle.failed(caller.addr, now);
            Response::err(id, RpcError::new(code::UNAUTHORIZED, "wrong token"))
        }
    }
}
