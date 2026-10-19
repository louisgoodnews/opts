# 🏗️ Phase 13: Crate Architecture: Legacy vs. Native Separation

*Goal: a strict boundary between **legacy OMSI data** and **native OPTS data**. Today `opts-core` holds both the OMSI parsers and the data structures the simulation and rendering crates use, so a native OPTS TOML format cannot be added without entangling it with OMSI's quirks. The phase splits parsing and data model into dedicated crates: `opts-omsi` (legacy `.bus`, `.sco`, `.cfg` readers, a leaf adapter), `opts-model` (native TOML read and write) and a shared base crate that both use, so there is no circular dependency.*

**Status:** 📝 Proposed 2026-10-19 by the project lead (task "Crate Architecture — Legacy vs. Native Separation"). Phase A (audit and ADR-0014) has not started. Three reporting points: after the audit (before any code moves), after the execution (before the documentation), and the final report with the merge.

The checklist is in [`ROADMAP.md`](../../../../ROADMAP.md) (Phase 13); this page holds the details.

## Why this is a phase of its own

It touches every crate: on 2026-10-19, 55 files of `opts-bevy`, 49 of `opts-game`, 20 of `opts-sim`, 7 of `opts-traffic` and 6 of `opts-benchmark` import `opts_core::formats::*`, and the game reads the OMSI installation at run time. The work needs an ADR, an importer, and a migration that keeps every step green, so it does not fit into a Phase 999 batch ([M-0129](../phase-999/M-0129.md) keeps the other crate-level decisions).

## Milestones (execution order)

1. [M-0131 — Audit, Options and ADR-0014](M-0131.md): the audit of `opts-core`, three options, a recommendation, the dependency lists, the importer location. **Checkpoint 1.**
2. [M-0132 — New Crates and Shared Types](M-0132.md): scaffolding and the shared domain types in the base crate.
3. [M-0133 — Move the OMSI Parsers into `opts-omsi`](M-0133.md): `git mv`, identical test results.
4. [M-0134 — Native Model, Importer, Workspace Wiring and Documentation](M-0134.md): `opts-model`, the importer, the rest of the workspace, ADR, crate graph, CONTRIBUTING, merge. **Checkpoint 2** before the documentation.

## Rules

No circular dependency; `opts-sim`, `opts-bevy` and `opts-game` do not depend on `opts-omsi`; `opts-omsi` is a leaf adapter; parser behaviour does not change; no OMSI parsing logic is deleted; `git mv` so history follows; no new workspace dependency without approval; nothing to `master` without the project lead's approval.

## Open points for the audit

1. The task says `opts-game` depends on `opts-omsi` "for the importer" and also that it must not. Proposed: a separate `opts-importer` crate that the CLI calls.
2. The game loads OMSI files at run time today; the ADR must say which crate does that without breaking the rule, or record a dated exception.
3. Overlap with M-0129: this phase decides `opts-core`; M-0129 keeps the other crates.
