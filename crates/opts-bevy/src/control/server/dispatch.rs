//! Queueing of control requests to the main thread and the reply wait.

use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use opts_cli::protocol::{code, Request, Response, RpcError, MAX_BATCH};
use serde_json::Value;

use super::auth::{auth_reply, gate};
use super::limiter::Limiter;
use crate::control::state::{Pending, PendingFlags};

/// Seconds the app has to pick a request up before it fails with `1005`.
const START_TIMEOUT: Duration = Duration::from_secs(5);
/// Seconds a started (possibly multi-frame) command may take.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(125);

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

pub(super) fn handle_single(
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

pub(super) fn handle_batch(
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
