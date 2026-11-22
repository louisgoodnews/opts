# ISS-004: Control token on command line and in shell history

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-004 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | minor (safety) |
| **Milestone** | [M-0043](../../src/roadmap/phase-5/M-0043.md) CLI Control Protocol & Hot Reload (owns [ADR-0006](../../src/adr/0006-control-protocol.md) and the [control protocol](../../src/architecture/control-protocol.md)) |
| **Audit** | PRJ-15, safety 4 in [project-audit-2026-11-18](../../audit/project-audit-2026-11-18.md) |
| **Status** | in progress (started 2026-11-22 on `agent/control-server-2`, from master `ecefa84b`) |

## Problem

The token is a plain argument: `--cli-token` in the game (`crates/opts-cli/src/args.rs`) and the client (`crates/opts-cli/src/run.rs`, `--token`), and the `auth` command takes it as a positional (`crates/opts-cli/src/commands/table/basics.rs`). It is visible in `/proc` to other local users and is saved in the shell history.

## Work

1. Spec first: control protocol section 7 names the sources: environment variable `OPTS_CLI_TOKEN` or a file given by `--cli-token-file <PATH>` (or `--token-file` for the client); the old argv flags are refused with a clear error.
2. A shared token-source helper in `opts-cli`; the game and the client use it.
3. `auth` without a positional uses the configured token; with a positional it is refused.
4. Tests: the old flag is refused (game and client); the env and file sources work; both sources at once are refused.
5. Removal check per test; restore with `git checkout`.

## Outcome

Open.
