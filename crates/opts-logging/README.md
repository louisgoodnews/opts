# 🪵 `opts-logging`

Foundational Layer 0 logging crate for the **Open Public Transport Simulator (OPTS)**.

Provides console formatting, non-blocking asynchronous writers, per-subsystem file routing (`opts.log`, `opts-core.log`, `opts-sim.log`, `opts-bevy.log`, `opts-gui.log`, `opts-game.log`, `opts-tools.log`, `panics.log`), log rotation, retention, and crash panic hooks.

## Architectural Boundary

`opts-logging` has zero dependencies on any internal OPTS crate. It is positioned at Layer 0 in the workspace architecture.
