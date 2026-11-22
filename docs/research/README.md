# Research: editor capabilities, scripting and OPTS as a system

| Metadata | Value |
| :--- | :--- |
| **Status** | 🔬 Research, not decided. Nothing here is a commitment or a milestone. |
| **Date** | 2026-11-22 |
| **Scope** | Docs only. No code, no builds, no game runs. |
| **Governing rule** | [ADR-0002](../src/adr/0002-clean-room-policy.md) applies to every file in this folder. |
| **Location** | Plain folder, not part of the mdBook build (`docs/src` is the book). |

## Purpose

The vision describes OPTS as "a modern, extensible, community-driven simulator" with an editor, scenarios and a scripting layer ([vision](../src/vision.md)). The roadmap already specifies much of this in pieces: the editor mode ([M-0051](../src/roadmap/phase-6/M-0051.md)), the script runtime and visual script editor ([M-0055](../src/roadmap/phase-6/M-0055.md), [M-0056](../src/roadmap/phase-6/M-0056.md), [M-0092](../src/roadmap/phase-6/M-0092.md)), the scenario editor ([M-0172](../src/roadmap/phase-18/M-0172.md)) and roaming services ([M-0173](../src/roadmap/phase-18/M-0173.md)). Nothing yet answers three questions together:

1. What must an editor do for maps, timetables and vehicle logic, and what is missing today?
2. Which scripting approach should OPTS use for content it does not inherit from OMSI?
3. What does "OPTS as a system, not only an OMSI replica" mean as a test a player can see?

These files answer those questions as proposals for the owner.

## Files

| File | Answers | Roadmap link |
| :--- | :--- | :--- |
| [editor-capabilities.md](editor-capabilities.md) | Capability lists for maps, timetables and routes, and scripts and vehicle behaviour. Each capability has a priority, what exists in the repository, the gap and the milestone it belongs to. | [M-0051](../src/roadmap/phase-6/M-0051.md) (E1 to E5), [M-0055](../src/roadmap/phase-6/M-0055.md), [M-0056](../src/roadmap/phase-6/M-0056.md), [M-0137](../src/roadmap/phase-14/M-0137.md) |
| [scripting-dsl.md](scripting-dsl.md) | Four options for scripting: OMSI compatibility only, an OPTS-native DSL, an embedded language, or nothing beyond OSC. A recommendation, with the conditions that would change it. | [M-0092](../src/roadmap/phase-6/M-0092.md), [M-0026](../src/roadmap/phase-4/M-0026.md), [M-0172](../src/roadmap/phase-18/M-0172.md), [M-0173](../src/roadmap/phase-18/M-0173.md), [ADR-0007](../src/adr/0007-hot-reload.md) section 9 |
| [vision-and-system.md](vision-and-system.md) | Which parts of OPTS replicate OMSI, which are OPTS's own, and the player-visible test for each. | [vision](../src/vision.md), [Phase 18](../src/roadmap/phase-18/README.md) |
| [sources.md](sources.md) | Every external source with its URL, the date accessed and a confidence tag. Also the project documents read for this set. | none |

## How this relates to the roadmap

* **No milestone is changed by this set.** Each proposal names the milestone it would fall under. A new milestone is proposed only where none fits (the scripting decision and the scenario format are the two cases).
* **Existing documents already answer parts of the question.** The editor specification ([editor README](../src/editor/README.md), [architecture](../src/editor/architecture.md), [validation](../src/editor/validation.md)) fixes the principles and the rule catalogue. The script generator specification ([script-generator](../src/architecture/script-generator.md)) fixes the vehicle-script tooling. This set does not repeat them. It records where they stop, and which decision they leave open.
* **Status checks.** Statements about what exists were checked against the code with `grep` and against the crate list on 2026-11-22. Where a claim could not be verified in code, the file says so.

## Status of the main facts (checked 2026-11-22)

| Fact | Evidence |
| :--- | :--- |
| No editor crate exists (`opts-edit`, `opts-editor` are specified only) | `crates/` listing; [architecture](../src/editor/architecture.md) |
| Map writers exist for `global.cfg` and tile `[object]` records only | `crates/opts-omsi/src/formats/map/write/manifest.rs`, `write/tile.rs` |
| No timetable writer exists; timetables are read for the route layer | `crates/opts-bevy/src/route.rs`, `host_callbacks/route_callbacks/timetable.rs`; [M-0099](../src/roadmap/phase-4/M-0099.md) done |
| No `opts-script_engine` crate; the VM lives in `opts-sim` | `crates/` listing; [M-0092](../src/roadmap/phase-6/M-0092.md) not started |
| Hot reload does not cover `.osc` files | [ADR-0007](../src/adr/0007-hot-reload.md) scope; no `osc` hit in `crates/opts-bevy/src/hot_reload` |
| No scenario, challenge, career or multiplayer code | grep of `crates/` for "challenge", "career", "multiplayer" (the only "scenario" hits are benchmark and probe names) |
| The control protocol has no `editor.*` group | [control protocol](../src/architecture/control-protocol.md); grep of `crates/` |
| The control server has an open blocker | ISS-001 in [issues register](../audit/issues-2026-11-18.md); [BOARD](../dev/BOARD.md) |

## Open questions

Open questions are marked in each file (Q-SD, Q-VS, or a table row). The owner decisions are listed at the end of this README and refer back to them.

## Owner decisions needed

1. **Scripting option for new content.** Choose (a), (b), (c) or (d) from [scripting-dsl.md](scripting-dsl.md). The recommendation is (b), limited to declarative scenario rules, with (c) deferred.
2. **Scenario format and scoring output.** Decide whether a scenario is a declarative file, a section of a save ([ADR-0011](../src/adr/0011-session-save-format.md)), or both. Decide whether a run ends with a session summary or a stored result (the open part of [ISS-048](../dev/issues/ISS-048-challenges-and-scoring.md) and [M-0172](../src/roadmap/phase-18/M-0172.md)).
3. **Timetable writer scope.** Decide whether timetable writing waits until the field meanings of `.ttl`, `.ttp` and `.ttr` are verified, with preserve-only writers in the meantime (the E0 spec gap in the [editor roadmap](../src/editor/roadmap.md)).
4. **Export target for timetables.** Decide whether exports must load in OMSI 2 or whether an OPTS-native format is acceptable for new content.
5. **OMSI compatibility for new scripts.** Decide whether generated and hand-written new vehicle scripts must also load in OMSI (open question Q-SG-1 in [script-generator](../src/architecture/script-generator.md)).
6. **Hot reload for `.osc`.** Decide whether `.osc` files join the hot-reload scope, which [ADR-0007](../src/adr/0007-hot-reload.md) does not list.
7. **Control server before editor control.** Confirm that the `editor.*` control methods wait until ISS-001 to ISS-003 are closed, since an editor that writes files through the control protocol widens the attack surface.
8. **Visual script editor.** Decide whether the subsystem graph in [M-0056](../src/roadmap/phase-6/M-0056.md) stays in scope, or whether forms and generated text are enough.
9. **Editor order.** Decide whether maps (E2) or timetables (E5) come first when the editor starts, since scenarios depend on both.
10. **Scenario editor timing.** Decide whether [M-0172](../src/roadmap/phase-18/M-0172.md) waits for E5, or starts with a read-only scenario format.
11. **Career and scenarios.** Decide whether career progression (RG-06 in the [feature research](../audit/feature-research-omsi-lotus-2026-11-22.md)) uses scenario rules, or is a separate system.
12. **Multiplayer.** Decide whether multiplayer stays a vision item without milestone ([ISS-046](../dev/issues/ISS-046-multiplayer-co-op-dispatch.md)), since the answer changes whether the scripting layer needs a network-safe API.
13. **Community OMSI SDK material.** Decide whether community translations and summaries of the OMSI map editor manual may be used as reference under [ADR-0002](../src/adr/0002-clean-room-policy.md), or only the project's own measurements.

**Total: 13 owner decisions.**
