# 🔭 Phase 11: Advanced Systems, Realism & Continuous Improvement

*Goal: Build on the session-based simulator of Phases 9 and 10 with the systems that make it feel finished: a usable in-game map, comfort and accessibility settings, deeper AI traffic and scheduled public transport, rail switches, adaptive streaming, spatial zones such as tunnels, weather that affects driving, a consolidated event system, and a recurring review of the roadmap itself.*

**Status:** ⏳ Planned 2026-10-18; approved by the project lead on 2026-10-20: the milestones are open for implementation (the page lists each one's own status). The execution order below is a recommendation; the project lead may reorder it.

The checklist is in [`ROADMAP.md`](../../../../ROADMAP.md) (Phase 11); this page holds the details.

## Milestones (recommended execution order)

1. [M-0118 — OMSI Event System Research and Consolidation Strategy](M-0118.md) — 📝 Proposed; documents only (taxonomy, dispatch, consolidation design); feeds M-0102, M-0098 and M-0115.
2. [M-0119 — Dev Agent Roadmap Review Process](M-0119.md) — 📝 Proposed; delivers prompt P-1010, a CONTRIBUTING note and the workflow reference.
3. [M-0109 — Quality-of-Life Suite](M-0109.md) — 📝 Proposed; one checklist: door camera, smooth driver view (after the jitter fix), UI scale presets, UI font settings.
4. [M-0108 — Full-Size In-Game Map and In-Game Map Switching](M-0108.md) — ✅ Done 2026-10-19; F6 map (the whole map), Alt menu *Change map*.
5. [M-0115 — Tunnel Detection and Spatial Zones](M-0115.md) — 📝 Proposed; zone abstraction for reverb and light-room ambience.
6. [M-0110 — Setvar Runtime Editor and CTI Value Types](M-0110.md) — 🟡 In progress since 2026-10-20: Alt menu *Setvars* page done; extends M-0078 and M-0077.
7. [M-0111 — AI Traffic Depth](M-0111.md) — 📝 Proposed; audit first, then obstacles, conservative lane changes, traffic-light compliance with `[rowdy_factor]`.
8. [M-0114 — Hybrid Tile and Object Streaming](M-0114.md) — 📝 Proposed; speed-adaptive streaming policy.
9. [M-0116 — Real Weather Research](M-0116.md) — 📝 Proposed; research only, a document and a decision.
10. [M-0117 — Weather Effects on Driving and Exhaust Visibility](M-0117.md) — 📝 Proposed; grip, fog, exhaust.
11. [M-0112 — Scheduled AI Traffic](M-0112.md) — ✅ Done 2026-10-23; buses, trains and trams by timetable (limits in the milestone).
12. [M-0113 — Track Switch Interaction](M-0113.md) — 📝 Proposed; points for AI and player on rails.

## Dependencies on prior phases

| Milestone | Needs |
| :--- | :--- |
| M-0108 | [M-0071](../phase-9/M-0071.md) mini map, [M-0068](../phase-10/M-0068.md) sessions (merged); [M-0069](../phase-10/M-0069.md) session save for carry-over |
| M-0109 | [M-0063](../phase-9/M-0063.md)/[M-0065](../phase-9/M-0065.md) settings and a11y, [M-0084](../phase-4/M-0084.md) camera; the merge of the parallel branch `agent/spawn-drift-jitter` for the smoothing item |
| M-0110 | [M-0078](../phase-10/M-0078.md) and [M-0077](../phase-10/M-0077.md) (both done 2026-10-18 and 2026-10-20) |
| M-0111 | [M-0049](../phase-4/M-0049.md), [M-0095](../phase-4/M-0095.md) and the 2026-10 kinematics/lane-change work (merged) |
| M-0112 | [M-0099](../phase-4/M-0099.md) and [M-0098](../phase-4/M-0098.md) (both not started: the longest dependency chain of the phase) |
| M-0113 | [M-0106](../phase-10/M-0106.md) (implemented) and M-0112 |
| M-0114 | [streaming coordinates](../../architecture/streaming-coordinates.md), [M-0088](../phase-9/M-0088.md) |
| M-0115 | lighting rooms ([lighting system](../../architecture/lighting-system.md)), M-0118 for the event shape |
| M-0116 | [weather](../../architecture/weather.md) (implemented) |
| M-0117 | [M-0073](../phase-10/M-0073.md) (planned); M-0116 optional |
| M-0118 | M-0096 script host; consumers [M-0102](../phase-4/M-0102.md), M-0098 |
| M-0119 | the prompt library |

## Ordering notes

* Research and process first (M-0118, M-0119): they cost no code and shape M-0102, M-0098 and M-0115.
* M-0109 and M-0108 are the visible player-facing wins and have no hard unfinished dependencies; the smoothing item of M-0109 waits for the jitter-fix branch.
* M-0115 follows M-0118 so that zone enter/leave events use the agreed event shape.
* M-0110 is only worthwhile once M-0078 exists; if M-0078 is scheduled earlier, fold the value types into it and keep only the Alt menu page here.
* M-0117 should follow M-0073; without it only an interim grip multiplier is possible.
* M-0112 is last-but-one because it waits for M-0099 and M-0098 from Phase 4; M-0113 needs M-0112 for the AI half but its player half can start after M-0106.
* This is a recommendation; the user may reorder.

## Feature-to-milestone map (27 items)

The numbering is the project lead's inventory of 2026-10-18. Items 22 to 26 are the Group G repeats of items 4, 3, 8, 5 and 6 and are merged into those milestones; no item is dropped or deferred.

| # | Feature (project lead's inventory) | Captured in |
| :--- | :--- | :--- |
| 1 | Full-size in-game map (F6, drag pan, wheel zoom, Esc, position and heading) | [M-0108](M-0108.md) |
| 2 | In-game map switching via the Alt menu | [M-0108](M-0108.md) |
| 3 | Door camera | [M-0109](M-0109.md) |
| 4 | Smooth camera in driver view | [M-0109](M-0109.md) (after the `agent/spawn-drift-jitter` fix) |
| 5 | UI scaling setting (100/125/150/200 %) | [M-0109](M-0109.md) |
| 6 | UI font and base font size | [M-0109](M-0109.md) |
| 7 | In-simulation setvar editor | [M-0110](M-0110.md) |
| 8 | CTI interpreter: non-binary value types | [M-0110](M-0110.md) |
| 9 | Force feedback | [M-0102](../phase-4/M-0102.md), existing; gaps listed below |
| 10 | AI obstacle recognition and braking | [M-0111](M-0111.md) |
| 11 | AI lane change | [M-0111](M-0111.md) |
| 12 | AI traffic-light compliance with rowdyness | [M-0111](M-0111.md) |
| 13 | Scheduled AI traffic (buses, trams, trains) | [M-0112](M-0112.md) |
| 14 | Track switch interaction | [M-0113](M-0113.md) |
| 15 | Hybrid tile and object streaming | [M-0114](M-0114.md) |
| 16 | Tunnel detection rule | [M-0115](M-0115.md) |
| 17 | Real weather (research) | [M-0116](M-0116.md) |
| 18 | Weather effects on driving physics | [M-0117](M-0117.md) |
| 19 | Exhaust gas visibility | [M-0117](M-0117.md) |
| 20 | OMSI event system documentation | [M-0118](M-0118.md) |
| 21 | Event consolidation strategy | [M-0118](M-0118.md) |
| 22 | Smooth camera (duplicate of 4) | [M-0109](M-0109.md) (as item 4) |
| 23 | Door camera (duplicate of 3) | [M-0109](M-0109.md) (as item 3) |
| 24 | CTI improvements (duplicate of 8) | [M-0110](M-0110.md) (as item 8) |
| 25 | UI scaling (duplicate of 5) | [M-0109](M-0109.md) (as item 5) |
| 26 | UI font and size (duplicate of 6) | [M-0109](M-0109.md) (as item 6) |
| 27 | Dev agent participation (process) | [M-0119](M-0119.md) |

## Already covered elsewhere

**Force feedback: [M-0102](../phase-4/M-0102.md) (decision: it stays there).** Phase 11 creates no milestone for it. Gaps in M-0102's text that its owner should close when it starts:

* The scope names `FF_*` but the only deliverable is one line (`FF_Vib_*` to a controller abstraction, no-op without a device); the other `FF_*` variables and their meaning are not listed.
* No acceptance criterion covers force feedback at all (criteria 1-3 are about sound triggers and events).
* No device layer is chosen (platform API, crate), no setting to enable it, no strength/gain control and no test double for a fake device.
* The event/variable path for FF is not tied to the event design of [M-0118](M-0118.md); once that exists, M-0102 should reference it.

**AI baseline: [M-0049](../phase-4/M-0049.md) and [M-0095](../phase-4/M-0095.md).** Car following, profiles, traffic lights, right of way, lane changes and kinematics exist; [M-0111](M-0111.md) starts with an audit of them and closes only the remaining gaps. Scheduled traffic is deliberately out of M-0049/M-0095; [M-0112](M-0112.md) takes it, on top of [M-0099](../phase-4/M-0099.md) (whose timetable model serves scripts and the player; a reader API for non-player vehicles is not specified there) and [M-0098](../phase-4/M-0098.md).

**Setvars: [M-0078](../phase-10/M-0078.md) and [M-0077](../phase-10/M-0077.md).** The catalogue, the editor in the wizard and Pause page, and configurations belong there. [M-0110](M-0110.md) adds the Alt-menu runtime editor and the value types, which M-0078 names (switch, choice, number) but does not specify against the `.cti` syntax.

**Recently merged work this phase builds on** ([Berlin-Spandau report](../../meta/berlin-spandau-2026-10-17.md), [systems report](../../meta/berlin-spandau-systems-2026-10-17.md)): weather rendering, presets and wetness; AI lane changes and kinematics; mirror cameras; light rooms and cones. None of the Phase 11 milestones repeats them.

---

*Proposed phase page. It is mirrored in `ROADMAP.md`, the book summary and the board (Backlog); execution waits for the project lead's approval.*
