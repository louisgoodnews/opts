# ISS-095: The opts-bevy dependency cycle blocks Bevy routing for crate splits

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-095 (owner request, 2026-11-26: investigate the opts-bevy cycle, create issues, fix) |
| **Severity** | major (architecture; blocks the epics ISS-090 and ISS-092) |
| **Milestone** | none; owned by ISS-093 (code audit, restructuring and monitoring) |
| **Status** | open, reported 2026-11-26; the rule is decided, the spatial-zones part is in progress |

## Problem

The owner decided that a new crate that needs Bevy imports Bevy through `opts-bevy` (no new
exemption in the Bevy rule, ISS-063). That only works for a crate that `opts-bevy` does not depend
on, directly or through other crates. Cargo rejects a cycle, so a Bevy part that depends on
`opts-bevy` cannot be a dependency of anything `opts-bevy` reaches.

Measured on integration (`crates/*/Cargo.toml`, a read-only check): `opts-bevy` reaches these
epic crates: `opts-audio`, `opts-light-quality`, `opts-omsi-text`, `opts-sim-passengers`,
`opts-spatial-zones`, `opts-traffic`. It does not reach `opts-game-state` or `opts-challenge`.

## Findings per epic

- **opts-light-quality (ISS-090):** the Bevy-free `levels` module moved to
  `opts-light-quality-core`. The rest derives Bevy's `Resource` on `LightQuality`, which the
  modules that build it need, so it stays in the facade. Resolved for the split; the rest is a
  redesign, not a split.
- **opts-audio (ISS-088):** the pure gain curves moved to `opts-audio-curves`. The sound engine
  stays in the facade (Bevy). No cycle, because it does not route through `opts-bevy`.
- **opts-spatial-zones (ISS-092):** uses Bevy directly (13 files). `opts-bevy` depends on it.
  A Bevy split part that depends on `opts-bevy` would close the loop. Only a Bevy-free part can
  move out, as in ISS-090.
- **opts-game-state (ISS-084):** uses Bevy directly, but `opts-bevy` does not reach it, so its
  Bevy parts may depend on `opts-bevy`. No cycle.
- **opts-traffic (ISS-085), opts-sim-passengers (ISS-086), opts-omsi-text (ISS-087):** no direct
  Bevy dependency. Their planned parts are pure. No cycle expected.
- **opts-shell-keys (ISS-091), opts-challenge (ISS-089):** no Bevy; split done.

## Rule to apply to every remaining split

Before a split, check the graph: if the crate being split is reached by `opts-bevy`, a part that
needs Bevy stays in the facade or in `opts-bevy`; only the Bevy-free part moves out, and the
moved part must not depend on `opts-bevy`.

## Done when

- The spatial-zones split is done (its Bevy-free part) or recorded as not possible with the reason.
- ISS-090 and ISS-092 say which part moved and which stays, with the cycle check result.
- The rule above is in the epic method for every remaining epic.
