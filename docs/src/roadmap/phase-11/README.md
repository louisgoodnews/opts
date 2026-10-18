# 🔭 Phase 11: Advanced Systems, Realism & Continuous Improvement

*Goal: Build on the session-based simulator of Phases 9 and 10 with the systems that make it feel finished: a usable in-game map, comfort and accessibility settings, deeper AI traffic and scheduled public transport, rail switches, adaptive streaming, spatial zones such as tunnels, weather that affects driving, a consolidated event system, and a recurring review of the roadmap itself.*

**Status:** ⏳ Planned 2026-10-18; every milestone is 📝 Proposed and awaits approval by the project lead. Nothing is started. The execution order below is a recommendation; the project lead may reorder it.

The checklist for this phase will live in the [roadmap](../README.md) once the phase is approved; until then this page is the only index (the roadmap, the book summary and the board are not updated yet).

## Milestones (recommended execution order)

1. [M-0118 — OMSI Event System Research and Consolidation Strategy](M-0118.md) — 📝 Proposed; documents only (taxonomy, dispatch, consolidation design); feeds M-0102, M-0098 and M-0115.
2. [M-0119 — Dev Agent Roadmap Review Process](M-0119.md) — 📝 Proposed; delivers prompt P-1010, a CONTRIBUTING note and the workflow reference.
3. [M-0109 — Quality-of-Life Suite](M-0109.md) — 📝 Proposed; one checklist: door camera, smooth driver view (after the jitter fix), UI scale presets, UI font settings.
4. [M-0108 — Full-Size In-Game Map and In-Game Map Switching](M-0108.md) — 📝 Proposed; M key map, Alt menu *Change map*.
5. [M-0115 — Tunnel Detection and Spatial Zones](M-0115.md) — 📝 Proposed; zone abstraction for reverb and light-room ambience.
6. [M-0110 — Setvar Runtime Editor and CTI Value Types](M-0110.md) — 📝 Proposed; extends M-0078 and M-0077.
7. [M-0111 — AI Traffic Depth](M-0111.md) — 📝 Proposed; audit first, then obstacles, conservative lane changes, traffic-light compliance with `[rowdy_factor]`.
8. [M-0114 — Hybrid Tile and Object Streaming](M-0114.md) — 📝 Proposed; speed-adaptive streaming policy.
9. [M-0116 — Real Weather Research](M-0116.md) — 📝 Proposed; research only, a document and a decision.
10. [M-0117 — Weather Effects on Driving and Exhaust Visibility](M-0117.md) — 📝 Proposed; grip, fog, exhaust.
11. [M-0112 — Scheduled AI Traffic](M-0112.md) — 📝 Proposed; buses, trams, trains by timetable.
12. [M-0113 — Track Switch Interaction](M-0113.md) — 📝 Proposed; points for AI and player on rails.

## Dependencies on prior phases

| Milestone | Needs |
| :--- | :--- |
| M-0108 | [M-0071](../phase-9/M-0071.md) mini map, [M-0068](../phase-10/M-0068.md) sessions (merged); [M-0069](../phase-10/M-0069.md) session save for carry-over |
| M-0109 | [M-0063](../phase-9/M-0063.md)/[M-0065](../phase-9/M-0065.md) settings and a11y, [M-0084](../phase-4/M-0084.md) camera; the merge of the parallel branch `agent/spawn-drift-jitter` for the smoothing item |
| M-0110 | [M-0078](../phase-10/M-0078.md) and [M-0077](../phase-10/M-0077.md) (both still planned) |
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

The 27 items below are derived from the approved milestone list of 2026-10-18; the original inventory text was not part of the brief and should be checked against this table by the project lead.

| # | Feature | Milestone |
| :--- | :--- | :--- |
| 1 | Full-size in-game map (M key, pan, zoom, position and heading) | [M-0108](M-0108.md) |
| 2 | In-game map switching via the Alt menu | [M-0108](M-0108.md) |
| 3 | Door camera overlay | [M-0109](M-0109.md) |
| 4 | Smooth driver-view camera | [M-0109](M-0109.md) |
| 5 | UI scale presets 100/125/150/200 % | [M-0109](M-0109.md) |
| 6 | UI font setting | [M-0109](M-0109.md) |
| 7 | Base font size setting | [M-0109](M-0109.md) |
| 8 | Setvar runtime editor (Alt menu) | [M-0110](M-0110.md) |
| 9 | CTI value types (ints, ranges, enums) | [M-0110](M-0110.md) |
| 10 | AI obstacle recognition and braking | [M-0111](M-0111.md) |
| 11 | AI conservative lane change | [M-0111](M-0111.md) |
| 12 | AI traffic-light compliance with `[rowdy_factor]` | [M-0111](M-0111.md) |
| 13 | Scheduled AI buses | [M-0112](M-0112.md) |
| 14 | Scheduled AI trams | [M-0112](M-0112.md) |
| 15 | Scheduled AI trains | [M-0112](M-0112.md) |
| 16 | Track switch reading from tiles | [M-0113](M-0113.md) |
| 17 | Vehicle route announcement and switch response (AI and player) | [M-0113](M-0113.md) |
| 18 | Hybrid tile and object streaming by speed | [M-0114](M-0114.md) |
| 19 | Tunnel detection | [M-0115](M-0115.md) |
| 20 | Spatial zones (reverb, light-room ambience precursor) | [M-0115](M-0115.md) |
| 21 | Real weather research | [M-0116](M-0116.md) |
| 22 | Weather effects on driving physics | [M-0117](M-0117.md) |
| 23 | Exhaust visibility | [M-0117](M-0117.md) |
| 24 | OMSI event system research | [M-0118](M-0118.md) |
| 25 | Event consolidation strategy | [M-0118](M-0118.md) |
| 26 | Dev agent roadmap review process | [M-0119](M-0119.md) |
| 27 | Force feedback | already covered by [M-0102](../phase-4/M-0102.md) (see below) |

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

*Proposed phase page. Not yet mirrored in the [roadmap](../README.md), the book summary or the board; those follow after the project lead's approval.*
