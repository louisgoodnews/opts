# ISS-096: Epic: opts-debugger, a development toolkit crate

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-096 (owner request, 2026-11-26) |
| **Severity** | minor (development tooling; no player-visible change) |
| **Milestone** | none yet; to be assigned by the owner |
| **Status** | open, reported 2026-11-26; research first |

## Problem

The project has development and debugging tools spread over several places: the logging crate
`opts-logging`, the control protocol and its client, `opts-cli`, `opts-tools`, the scripts under
`scripts/`, and the examples and probes in several crates. There is no single crate that collects
the tools a developer uses to inspect and debug a running session. The owner asked for one:
`opts-debugger`, a toolkit crate for development.

## Goal

A development toolkit crate, `opts-debugger`, that developers use while working on the game. It
is not part of the game's runtime: no game crate depends on it, and it is not shipped as part of
the game.

## Stages

Research: `docs/dev/research/opts-debugger-2026-11-26.md` (merged). The stages below follow it.

1. **Research (this stage):** inventory the existing development tools and their overlap; decide
   the scope of the toolkit; decide the placement (the crate graph rule, the Bevy routing rule of
   ISS-095, and whether it is a binary, a library, or both); write the research document
   `docs/dev/research/opts-debugger-2026-11-26.md`. No code.
2. **Decisions:** the owner decides the scope and the placement from the research.
3. **Skeleton:** the crate and its first tool, with its budget at its measured size and a crate
   graph row, in the style of the other split crates.
4. **Tools:** the remaining tools, one per issue, as the research orders them.

## Decisions (2026-11-26, owner)

1. **Placement:** a new crate, `opts-debugger` (not subcommands of `opts-tools`).
2. **Trace tool and Bevy:** the trace tool routes through `opts-bevy` for the trace types, as the Bevy rule allows.
3. **Client:** an own client inside `opts-debugger`; `opts-cli` is not reused. The protocol code is duplicated and must be kept in step with the server.
4. **Server work:** the entity, resource and asset dump commands are part of this epic, as a later stage. They change the game's control server, so each such change needs a review before it merges.
5. **Bundle contents:** logs, crash reports, timing and frame files, and screenshots. The control token is never included. Screenshots can show personal data, so the bundle must say so in its manifest.

## Done when

- The research document exists, and the owner has decided its scope and placement.
- The crate exists with a budget entry at its measured size and a crate-graph row.
- No game crate depends on `opts-debugger`; the cycle and Bevy rules hold (grep).
