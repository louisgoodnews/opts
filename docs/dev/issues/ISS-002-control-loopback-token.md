# ISS-002: Control server on loopback needs no token

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-002 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (safety) |
| **Milestone** | [M-0043](../../src/roadmap/phase-5/M-0043.md) CLI Control Protocol & Hot Reload (owns [ADR-0006](../../src/adr/0006-control-protocol.md) and the [control protocol](../../src/architecture/control-protocol.md)) |
| **Audit** | PRJ-06, safety 2 in [project-audit-2026-11-18](../../audit/project-audit-2026-11-18.md) |
| **Status** | in progress (started 2026-11-22 on `agent/control-server-2`, from master `ecefa84b`) |

## Problem

The control server starts on loopback without a token. `ControlConfig::default()` (`crates/opts-bevy/src/control/mod.rs`) has `token: None`, and `start` in `crates/opts-bevy/src/control/server.rs` refuses a missing token only for a non-loopback bind. Any local account that can reach `127.0.0.1` can send every command, including `shutdown`, input injection and file-writing commands. A browser page can reach the port too: the line-based reader in `connection.rs` has no HTTP parsing, so the drive-by case is suspected, not run.

## Work

1. Spec first: the control protocol (sections 1 and 7) and ADR-0006 say that a token is required on loopback too. State the old and new wording in the issue outcome.
2. `start` refuses a server without a token on every bind (`PermissionDenied`, message names `OPTS_CLI_TOKEN` and `--cli-token-file`).
3. Tests: loopback without a token is refused; the removal of that check makes the test fail.
4. Regenerate nothing else; update the CLI README and manual lines that start the game without a token.

## Outcome

Spec first in `5a087e03` (section 1, section 7, ADR-0006): a token is required on every bind, loopback included. Code: `start` in `crates/opts-bevy/src/control/server.rs` refuses a server without a token (`PermissionDenied`, names `OPTS_CLI_TOKEN` and `--cli-token-file`). `auth` is unchanged.

Removal check (the token-required check in `start` removed; file restored byte-identical):
- `control::server::tests::loopback_without_a_token_is_refused ... FAILED`
- `control::server::tests::non_loopback_binds_need_a_token ... FAILED`
- `test result: FAILED. 13 passed; 2 failed`

Passing with the change: `cargo test -p opts-bevy --lib control::server` 15 passed, 0 failed (exit 0); `cargo test -p opts-cli --lib` 37 passed, 0 failed (exit 0). Logs: `opts-scratch/work/iss002-004/`.
