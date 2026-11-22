//! Failure throttle for the `auth` call: per peer address and across all peers.
//! Spec: `docs/src/architecture/control-protocol.md` §7 (auth throttle).

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Failed `auth` calls in a row from one peer before that peer is locked out.
pub(super) const PEER_LIMIT: u32 = 5;
/// Failed `auth` calls from all peers within [`WINDOW`] before every peer is locked out.
pub(super) const GLOBAL_LIMIT: u32 = 50;
/// Length of the counting window of the global limit.
pub(super) const WINDOW: Duration = Duration::from_secs(60);
/// Length of a lockout.
pub(super) const LOCKOUT: Duration = Duration::from_secs(60);
/// Peer entries kept before expired ones are dropped.
const MAX_PEERS: usize = 4096;

/// Who is calling: the peer address and the shared throttle.
#[derive(Clone, Copy)]
pub(super) struct Caller<'a> {
    pub(super) addr: IpAddr,
    pub(super) throttle: &'a AuthThrottle,
}

#[derive(Default)]
struct PeerState {
    failures: u32,
    locked_until: Option<Instant>,
}

#[derive(Default)]
struct State {
    peers: HashMap<IpAddr, PeerState>,
    window_start: Option<Instant>,
    window_failures: u32,
    global_locked_until: Option<Instant>,
}

/// Counts failed `auth` calls and locks out peers and, globally, all callers.
#[derive(Default)]
pub(super) struct AuthThrottle {
    state: Mutex<State>,
}

impl AuthThrottle {
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `Ok` when `addr` may try `auth` now; otherwise the time left in the lockout.
    pub(super) fn check(&self, addr: IpAddr, now: Instant) -> Result<(), Duration> {
        let st = self.lock();
        let until = st
            .global_locked_until
            .max(st.peers.get(&addr).and_then(|p| p.locked_until));
        match until {
            Some(t) if t > now => Err(t - now),
            _ => Ok(()),
        }
    }

    /// Counts one failed `auth` from `addr`.
    pub(super) fn failed(&self, addr: IpAddr, now: Instant) {
        let mut st = self.lock();
        if st.peers.len() >= MAX_PEERS && !st.peers.contains_key(&addr) {
            st.peers
                .retain(|_, p| p.locked_until.is_some_and(|t| t > now));
        }
        let peer = st.peers.entry(addr).or_default();
        peer.failures += 1;
        if peer.failures >= PEER_LIMIT {
            peer.failures = 0;
            peer.locked_until = Some(now + LOCKOUT);
        }
        if st
            .window_start
            .is_none_or(|s| now.duration_since(s) > WINDOW)
        {
            st.window_start = Some(now);
            st.window_failures = 0;
        }
        st.window_failures += 1;
        if st.window_failures >= GLOBAL_LIMIT {
            st.global_locked_until = Some(now + LOCKOUT);
            st.window_failures = 0;
            st.window_start = None;
        }
    }

    /// A successful `auth` clears the count of `addr`.
    pub(super) fn succeeded(&self, addr: IpAddr) {
        self.lock().peers.remove(&addr);
    }
}
