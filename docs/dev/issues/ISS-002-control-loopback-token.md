# ISS-002: Control server on loopback needs no token

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-002 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | major (safety) |
| **Milestone** | [M-0043](../../src/roadmap/phase-5/M-0043.md) CLI Control Protocol & Hot Reload (owns [ADR-0006](../../src/adr/0006-control-protocol.md) and the [control protocol](../../src/architecture/control-protocol.md)) |
| **Audit** | PRJ-06, safety 2 in [project-audit-2026-11-18](../../audit/project-audit-2026-11-18.md) |
| **Status** | in progress: token on every bind done; the browser drive-by check is open (manual, see Outcome) |

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

**Start failure with `--cli-port` (decision, `f513166d`, 2026-11-22).** The spec makes `--cli-port` the only way to start the server (control protocol §1 and §7: default off, and the args layer already refuses `--cli-port` without a token). The spec did not say what a failed start does; it is now fatal. `ControlPlugin::build` prints `opts: the control server could not start: …` and exits with status 2 before the event loop starts. Before: it logged the error and the game ran without a server. Spec §1 has the row "Start failure". Not run end to end (no game runs on this branch); the start path is covered by the `start` tests above.

**Open: browser drive-by check (item 3, manual, not claimed).** Not verified by any test. A manual check: start the game with `OPTS_CLI_TOKEN=<32+ characters> opts-game --cli-port 44201` (scratch folder, not `/tmp`). Serve a page from a different origin, for example `python3 -m http.server 8000` in a scratch folder, and open `http://localhost:8000/` in a browser. In the page's console, send `fetch("http://127.0.0.1:44201/", {method: "POST", mode: "no-cors", body: '{"jsonrpc":"2.0","id":1,"method":"ping"}\n'})`, and a second request with a plain HTTP line (`GET / HTTP/1.1`). Check the game's `DEBUG` log under `opts_bevy::control`: the request should arrive and be refused with error `1003` (UNAUTHORIZED) before `auth`, and no command should run; the HTTP line should get an error line, not a page. Record the result here with the date; until then this item stays open.
