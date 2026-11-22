# ISS-004: Control token on command line and in shell history

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-004 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (safety) |
| **Milestone** | [M-0043](../../src/roadmap/phase-5/M-0043.md) CLI Control Protocol & Hot Reload (owns [ADR-0006](../../src/adr/0006-control-protocol.md) and the [control protocol](../../src/architecture/control-protocol.md)) |
| **Audit** | PRJ-15, safety 4 in [project-audit-2026-11-18](../../audit/project-audit-2026-11-18.md) |
| **Status** | done (2026-11-22; started on `agent/control-server-2` from `ecefa84b`, closed on `agent/control-server-3`) |

## Problem

The token is a plain argument: `--cli-token` in the game (`crates/opts-cli/src/args.rs`) and the client (`crates/opts-cli/src/run.rs`, `--token`), and the `auth` command takes it as a positional (`crates/opts-cli/src/commands/table/basics.rs`). It is visible in `/proc` to other local users and is saved in the shell history.

## Work

1. Spec first: control protocol section 7 names the sources: environment variable `OPTS_CLI_TOKEN` or a file given by `--cli-token-file <PATH>` (or `--token-file` for the client); the old argv flags are refused with a clear error.
2. A shared token-source helper in `opts-cli`; the game and the client use it.
3. `auth` without a positional uses the configured token; with a positional it is refused.
4. Tests: the old flag is refused (game and client); the env and file sources work; both sources at once are refused.
5. Removal check per test; restore with `git checkout`.

## Outcome

Spec first in `5a087e03` (section 7). The token comes from `OPTS_CLI_TOKEN` or a file (`--cli-token-file` for the game, `--token-file` for `opts-cli`); setting both is refused. `--cli-token` (game) and `--token` (`opts-cli`) are refused with a clear error. `opts-cli auth` takes no argument and sends the configured token. The shared helper is `crates/opts-cli/src/token.rs`.

Closed 2026-11-22 on `agent/control-server-3`:
- Token file permissions (`e1aa232f`): on Unix a file with `mode & 0o077 != 0` is refused with an error that names the mode and asks for 0600 (spec §7; before: "keep the file readable only by you", not checked). Applies to the game's `--cli-token-file` and the client's `--token-file` through `from_sources`.
- Launchers (audit, no change needed): every script under `scripts/` that passes `--cli-port` sets the token source before it starts the game: `instancing-bench.sh`, `shadow-cull-bench.sh`, `visual-ab.sh` export `OPTS_CLI_TOKEN` (32 hex characters from `od`); `setvars-check.py` and `streaming-perf.py` set `OPTS_CLI_TOKEN` in the environment they pass to `Popen`. `capture-agent-demo.sh` only uses the client. `bash -n` on the three shell launchers and `python3 -m py_compile` on the two Python ones pass; none was run.

Removal check (the `--cli-token` refusal in `args/defaults.rs` removed; file restored byte-identical):
- `args::tests::the_old_cli_token_flag_is_refused_with_a_clear_error ... FAILED`
- `test result: FAILED. 6 passed; 1 failed`

Removal check for the file permission check (the `refuse_shared_file` call in `read_token_file` removed; `git checkout` restored it):
- `token::tests::a_0644_token_file_is_refused ... FAILED`; `test result: FAILED. 6 passed; 1 failed`

Passing with the change: `cargo test -p opts-bevy --lib control::server` 16 passed, 0 failed (exit 0); `cargo test -p opts-cli --lib` 39 passed, 0 failed (exit 0); after the restore `token::tests` 7 passed (exit 0). Logs: `opts-scratch/work/iss002-004/`.
