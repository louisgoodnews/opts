# ISS-001: Control server accepts empty or weak tokens; `auth` is not throttled

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-001 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | blocker (safety) |
| **Milestone** | [M-0043](../../src/roadmap/phase-5/M-0043.md) CLI Control Protocol & Hot Reload (owns [ADR-0006](../../src/adr/0006-control-protocol.md) and the [control protocol](../../src/architecture/control-protocol.md)) |
| **Audit** | PRJ-02 in [project-audit-2026-11-18](../../audit/project-audit-2026-11-18.md), safety note 1 |
| **Status** | done (commit `0c6f1fbb` on agent/control-server-auth, from master `b8a4b706`; started and finished 2026-11-22) |

## Problem

The control server decides whether a token is set and nothing more. Its `auth` call is not rate limited.

- `crates/opts-bevy/src/control/server.rs` (`start`, the non-loopback check): only `token.is_none()` is refused, so `""` passes, and a short token passes.
- `crates/opts-bevy/src/control/server/auth.rs` (`auth_reply`) compares the token in constant time but keeps no count of failures.
- `crates/opts-bevy/src/control/server/dispatch.rs` (`handle_single`, `handle_batch`) return from `auth` before `gate()` and the per-connection limiter, so failed `auth` attempts are never limited, and a new connection resets the limiter.
- The token travels in plain TCP (`server.rs`, `connection.rs`). ISS-004 (argv) and ISS-002 (loopback without a token) are separate.

With a LAN bind this allows a brute-force guess of the token, then `shutdown`, input injection, `capture.prune` with `dry_run:false`, save and profile deletes.

## Next (from the audit)

Refuse empty and short tokens (at least 32 random characters). Throttle failed `auth` per peer and globally. Until throttled, refuse any non-loopback bind.

## Work

1. Spec first: state the token rules in the [control protocol](../../src/architecture/control-protocol.md) (sections 1 and 7) before code.
2. `start` refuses an empty token and a token shorter than the minimum, on every bind.
3. A throttle for failed `auth` per peer address and globally, with a lockout; `auth` is covered by it.
4. Non-loopback bind is allowed only with a valid token and the throttle in place.
5. One test per refusal (empty, short, non-loopback without a token, valid token accepted, loopback behaviour) and throttle tests. The token check's test must fail when the check is removed.
6. Update the control-protocol docs, the issue outcome, and the board line.

## Decisions

- Minimum token length: **32 characters**. Source: the audit's recommendation (PRJ-02: "at least 32 random characters"). The protocol docs and ADR-0006 give no minimum before this change.

## Outcome

Fixed in `0c6f1fbb` (`fix(control): refuse short tokens and throttle failed auth calls`, M-0043). The commit does what the Work list asks:

- `start` in `crates/opts-bevy/src/control/server.rs` refuses a `--cli-token` shorter than `MIN_TOKEN_CHARS` (32), an empty one included, before the bind check, so every bind is covered.
- `crates/opts-bevy/src/control/server/throttle.rs` counts failed `auth` calls per peer (5 in a row: 60 s lockout) and across peers (50 in 60 s: 60 s lockout for everyone). `auth` is no longer exempt in `dispatch.rs`; `auth.rs` and `connection.rs` are updated to match.
- Tests: `control/server/tests.rs` and `control/server/tests_throttle.rs`.

Test runs (`cargo test -p opts-bevy --lib control::server:: --no-fail-fast`, shared target `opts-scratch/target-featuregap`):

- Earlier runs (before this session, filters not recorded in the logs): `test1.log` 13 passed, `test2.log` 3 passed, `test3.log` 13 passed, all `ok`.
- Removal check: the token-length check in `start` was removed (`git` diff of `server.rs`, 8 lines). Result, log `opts-scratch/work/iss001/removal.log`:
  - `control::server::tests::an_empty_token_is_refused_on_loopback ... FAILED` (`tests.rs:45`, `assertion \`left == right\` failed`, left `None`, right `Some(InvalidInput)`)
  - `control::server::tests::a_short_token_is_refused_on_loopback ... FAILED` (`tests.rs:54`, same assertion)
  - `test result: FAILED. 11 passed; 2 failed; 0 ignored; 0 measured; 630 filtered out; finished in 0.00s`
- Restored with `git checkout -- crates/opts-bevy/src/control/server.rs` (tree equal to HEAD), log `opts-scratch/work/iss001/restored.log`:
  - `test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 630 filtered out; finished in 0.00s`

A first removal run (`removal-stale-run.log`) reported 3 tests and passed. It used a stale test binary in the shared target directory, so it was not evidence and is kept only for the record. The rerun above recompiled `opts-bevy` after touching the sources.

Still open: ISS-002 (loopback without a token), ISS-003, ISS-004 (token in argv).
