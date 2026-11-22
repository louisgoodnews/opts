//! Unit tests for `super` (`server.rs`): token rules and the bind checks.

use super::*;

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

fn config(bind: &str, token: Option<&str>) -> ServerConfig {
    ServerConfig {
        bind: bind.into(),
        port: 0,
        token: token.map(String::from),
    }
}

fn start_kind(cfg: &ServerConfig) -> Option<std::io::ErrorKind> {
    let (tx, _rx) = mpsc::sync_channel(1);
    start(cfg, tx).err().map(|e| e.kind())
}

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
    assert_eq!(
        start_kind(&config("0.0.0.0", None)),
        Some(std::io::ErrorKind::PermissionDenied)
    );
}

#[test]
fn an_empty_token_is_refused_on_loopback() {
    assert_eq!(
        start_kind(&config("127.0.0.1", Some(""))),
        Some(std::io::ErrorKind::InvalidInput)
    );
}

#[test]
fn a_short_token_is_refused_on_loopback() {
    let short = "x".repeat(MIN_TOKEN_CHARS - 1);
    assert_eq!(
        start_kind(&config("127.0.0.1", Some(&short))),
        Some(std::io::ErrorKind::InvalidInput)
    );
}

#[test]
fn a_token_of_the_minimum_length_is_accepted_on_loopback() {
    assert_eq!(TOKEN.len(), MIN_TOKEN_CHARS);
    assert_eq!(start_kind(&config("127.0.0.1", Some(TOKEN))), None);
}

#[test]
fn loopback_without_a_token_is_refused() {
    assert_eq!(
        start_kind(&config("127.0.0.1", None)),
        Some(std::io::ErrorKind::PermissionDenied)
    );
}

#[test]
fn a_non_loopback_bind_with_a_valid_token_passes_the_checks() {
    // TEST-NET-1 is not a local address, so the bind itself fails; the checks before it pass.
    let kind = start_kind(&config("192.0.2.1", Some(TOKEN)));
    assert_ne!(kind, Some(std::io::ErrorKind::PermissionDenied));
    assert_ne!(kind, Some(std::io::ErrorKind::InvalidInput));
}
