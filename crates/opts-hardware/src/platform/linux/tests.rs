//! Unit tests for `super` (`linux.rs`).

use super::*;

#[test]
fn session_type_wins_then_sockets() {
    assert_eq!(
        display_server_from(Some("wayland"), false, true),
        DisplayServer::Wayland
    );
    assert_eq!(
        display_server_from(Some("tty"), false, true),
        DisplayServer::X11
    );
    assert_eq!(
        display_server_from(None, true, true),
        DisplayServer::Wayland
    );
    assert_eq!(display_server_from(None, false, false), DisplayServer::None);
}

#[test]
fn probes_never_panic_here() {
    let n = Native;
    let _ = (n.display_server(), n.environment(), n.cpu_caches_kib());
    let _ = (
        n.gpu_fallback(),
        n.display_fallback(),
        n.audio_fallback(),
        n.input_fallback(),
    );
}
