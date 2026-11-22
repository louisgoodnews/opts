# ISS-001: Control server accepts empty or weak tokens; `auth` is not throttled

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-001 (local register: [issues-2026-11-18](../../audit/issues-2026-11-18.md)) |
| **Severity** | blocker (safety) |
| **Milestone** | [M-0043](../../src/roadmap/phase-5/M-0043.md) CLI Control Protocol & Hot Reload (owns [ADR-0006](../../src/adr/0006-control-protocol.md) and the [control protocol](../../src/architecture/control-protocol.md)) |
| **Audit** | PRJ-02 in [project-audit-2026-11-18](../../audit/project-audit-2026-11-18.md), safety note 1 |
| **Status** | in progress (agent/control-server-auth, started 2026-11-22, from master `b8a4b706`) |

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

Open. See the Work list; filled in when the fix lands.
