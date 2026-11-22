# ISS-003: No read timeout or connection cap on the control server

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-003 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (safety) |
| **Milestone** | [M-0043](../../src/roadmap/phase-5/M-0043.md) CLI Control Protocol & Hot Reload (owns [ADR-0006](../../src/adr/0006-control-protocol.md) and the [control protocol](../../src/architecture/control-protocol.md)) |
| **Audit** | PRJ-07, safety 3 in [project-audit-2026-11-18](../../audit/project-audit-2026-11-18.md) |
| **Status** | in progress (started 2026-11-22 on `agent/control-server-2`, from master `ecefa84b`) |

## Problem

`serve_connection` in `crates/opts-bevy/src/control/server/connection.rs` loops until EOF. The server never calls `set_read_timeout` (only `opts-cli/src/transport.rs` does for the client), so a silent client keeps a thread for ever. The listener spawns one thread per accepted socket with no upper bound, so many silent clients exhaust threads and file descriptors. The stop flag cannot wake a thread blocked in `read_line`.

## Work

1. Spec first: control protocol section 1 gets the values: idle read timeout of 30 s per connection, and at most 8 simultaneous connections; a ninth receives one `1006 Busy` line and is closed.
2. `serve_connection` takes the idle timeout as a parameter; the listener passes the 30 s constant.
3. The listener counts live connections with a guard that releases the slot when the thread ends (or fails to start).
4. Tests: a silent client is dropped after the timeout (short timeout in the test); with 8 silent clients open, a ninth is refused with `1006` and the first one still works.
5. Removal check per test; restore with `git checkout`.

## Outcome

Spec first in `5a087e03` (section 1): a connection silent for 30 s is closed; at most 8 connections are served at once, a ninth gets one `1006 Busy` line and is closed. Code: `admission.rs` (cap, `Slots` guard, `refuse`); `connection.rs` takes the idle timeout as a parameter; `server.rs` listener uses both. The stop flag still waits for a blocked read to time out (at most 30 s); socket shutdown on stop is not done.

Removal checks (each file restored byte-identical):
- read timeout (`set_read_timeout` removed): `control::server::tests_admission::a_silent_client_is_dropped_after_the_read_timeout ... FAILED`; `test result: FAILED. 14 passed; 1 failed`
- connection cap (cap check removed): `control::server::tests_admission::the_ninth_connection_is_refused_with_busy ... FAILED`; `test result: FAILED. 14 passed; 1 failed`

Passing with the change: `cargo test -p opts-bevy --lib control::server` 15 passed, 0 failed (exit 0); `cargo test -p opts-cli --lib` 37 passed, 0 failed (exit 0). Logs: `opts-scratch/work/iss002-004/`.
