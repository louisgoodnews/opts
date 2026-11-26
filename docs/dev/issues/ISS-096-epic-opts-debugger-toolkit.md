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

1. **Research (this stage):** inventory the existing development tools and their overlap; decide
   the scope of the toolkit; decide the placement (the crate graph rule, the Bevy routing rule of
   ISS-095, and whether it is a binary, a library, or both); write the research document
   `docs/dev/research/opts-debugger-2026-11-26.md`. No code.
2. **Decisions:** the owner decides the scope and the placement from the research.
3. **Skeleton:** the crate and its first tool, with its budget at its measured size and a crate
   graph row, in the style of the other split crates.
4. **Tools:** the remaining tools, one per issue, as the research orders them.

## Decisions needed (after the research)

- The first set of tools (the research proposes a short list, ordered by value).
- A binary, a library, or both.
- Whether the toolkit may depend on Bevy (routed through `opts-bevy`, per ISS-095) or stays free of it.
- Whether tools that touch the control server reuse the existing client code or own one.

## Done when

- The research document exists, and the owner has decided its scope and placement.
- The crate exists with a budget entry at its measured size and a crate-graph row.
- No game crate depends on `opts-debugger`; the cycle and Bevy rules hold (grep).
