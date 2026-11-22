//! Unit tests of the `auth` throttle and of the locked-peer refusal (`super`, `server.rs`).

use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};

use opts_cli::protocol::{code, parse_line, Incoming, Request, Response};

use super::auth::auth_reply;
use super::throttle::{AuthThrottle, Caller, GLOBAL_LIMIT, LOCKOUT, PEER_LIMIT, WINDOW};

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

fn auth_request(token: &str) -> Request {
    let line =
        format!(r#"{{"jsonrpc":"2.0","id":1,"method":"auth","params":{{"token":"{token}"}}}}"#);
    let Ok(Incoming::Single(req)) = parse_line(&line) else {
        panic!("auth line does not parse");
    };
    req
}

fn error_code(r: &Response) -> Option<i64> {
    r.error.as_ref().map(|e| e.code)
}

fn peer(n: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, n))
}

#[test]
fn a_locked_peer_is_refused_even_with_the_right_token() {
    let throttle = AuthThrottle::default();
    let caller = Caller {
        addr: peer(1),
        throttle: &throttle,
    };
    let mut authed = false;
    for _ in 0..PEER_LIMIT {
        let r = auth_reply(&auth_request("wrong"), Some(TOKEN), &mut authed, caller);
        assert_eq!(error_code(&r), Some(code::UNAUTHORIZED));
    }
    let r = auth_reply(&auth_request(TOKEN), Some(TOKEN), &mut authed, caller);
    assert_eq!(
        error_code(&r),
        Some(code::BUSY),
        "locked peer must not be checked"
    );
    assert!(!authed);
    let other = Caller {
        addr: peer(2),
        throttle: &throttle,
    };
    let r = auth_reply(&auth_request(TOKEN), Some(TOKEN), &mut authed, other);
    assert_eq!(error_code(&r), None, "another peer is not locked");
    assert!(authed);
}

#[test]
fn a_peer_lockout_ends_after_its_time() {
    let t = AuthThrottle::default();
    let now = Instant::now();
    for _ in 0..PEER_LIMIT {
        t.failed(peer(1), now);
    }
    assert!(t.check(peer(1), now).is_err());
    assert!(t
        .check(peer(1), now + LOCKOUT + Duration::from_secs(1))
        .is_ok());
}

#[test]
fn a_success_clears_the_failure_count() {
    let t = AuthThrottle::default();
    let now = Instant::now();
    for _ in 0..PEER_LIMIT - 1 {
        t.failed(peer(1), now);
    }
    t.succeeded(peer(1));
    for _ in 0..PEER_LIMIT - 1 {
        t.failed(peer(1), now);
    }
    assert!(t.check(peer(1), now).is_ok());
}

#[test]
fn the_global_limit_locks_every_peer() {
    let t = AuthThrottle::default();
    let now = Instant::now();
    for i in 0..GLOBAL_LIMIT {
        t.failed(peer(i as u8), now);
    }
    assert!(t.check(peer(200), now).is_err());
    assert!(t
        .check(peer(200), now + LOCKOUT + Duration::from_secs(1))
        .is_ok());
}

#[test]
fn failures_spread_over_windows_do_not_trip_the_global_limit() {
    let t = AuthThrottle::default();
    let now = Instant::now();
    for i in 0..GLOBAL_LIMIT {
        let at = now + (WINDOW + Duration::from_secs(1)) * i;
        t.failed(peer(i as u8), at);
    }
    let end = now + (WINDOW + Duration::from_secs(1)) * GLOBAL_LIMIT;
    assert!(t.check(peer(200), end).is_ok());
}
