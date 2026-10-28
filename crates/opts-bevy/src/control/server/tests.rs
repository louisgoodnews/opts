//! Unit tests for `super` (`server.rs`).

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
