# ISS-002 removal check: refused control requests are logged

Feature: a control request refused for missing authentication (error 1003) is logged at DEBUG under
target `opts_bevy::control` with the method, the peer address and the error code. The token and
the request body are never logged. Spec: [control protocol](../../src/architecture/control-protocol.md)
section 7 ("All requests and errors are logged at DEBUG/WARN under target `opts_bevy::control`").
Code: the `gate` function in `crates/opts-bevy/src/control/server/auth.rs`. Test:
`crates/opts-bevy/tests/control_refusal_log.rs`, test `an_unauthenticated_ping_is_logged_without_the_token`
(its own test binary, so its global subscriber does not clash with `tests/control/protocol.rs`).

The log line (two physical lines: the `#[rustfmt::skip]` attribute and the statement):

```rust
        #[rustfmt::skip]
        tracing::debug!(target: "opts_bevy::control", method = %req.method, peer = %caller.addr, code = code::UNAUTHORIZED, "request refused: not authenticated");
```

## Check: remove the log statement (both lines) from `gate`

Removed the `#[rustfmt::skip]` line and the `tracing::debug!` line; nothing else changed.

| Test | Result | Message |
| :--- | :--- | :--- |
| `an_unauthenticated_ping_is_logged_without_the_token` | FAILED | `no refusal line in the log` (the test's `unwrap_or_else` panic at `control_refusal_log.rs:86`) |

`cargo test -p opts-bevy --test control_refusal_log`: `1 failed` with the line removed; `1 passed` with it.

Restore: a copy taken before the check (`opts-scratch/work/iss002-log/auth.rs.final-good`) was
copied back. `cmp` against it: identical. `sha256sum` of `crates/opts-bevy/src/control/server/auth.rs`
before the check and after the restore: `ac04ea5d208b950721e1e2c31119d53a662b6179ca19adcb99fecb3a53ce81f4`.

An earlier run of the same check, on a version of the file whose `return` was on one line, also failed
the test and restored byte-identical (`63bc9e19...`); that version was replaced by the final formatting above.

## Budget

`opts-bevy` stays within its budget: the gate change nets one code line fewer than before
(`let _ = token;` removed, the one-line log statement added, the `return` rewritten as `let e` and
`return Some(e)`; the measured total went from 45831 to 45830). `docs/dev/crate-budget.txt` lowered from 45831 to 45830 (the measured total).
