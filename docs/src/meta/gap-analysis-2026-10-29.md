# Gap Analysis: What Is Missing for a Complete Public-Transport Simulator (2026-10-29)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Written for the project lead's prioritisation; nothing below is approved or scheduled |
| **Milestone** | [M-0074](../roadmap/phase-5/M-0074.md) (expansion pass) |
| **Compared** | The state of [`ROADMAP.md`](../../../ROADMAP.md) and the milestone files on 2026-10-29 against the pillars of the [vision](../vision.md) and the area list of M-0074 |
| **Author** | Claude Sonnet 5.5, from the roadmap and the code; the project lead decides what is wanted |

## Method and limits

For each pillar of the vision (vehicle, world, transit, passenger, game, modding) and each area M-0074 lists (passengers and ticketing, timetables and routes, stops and displays, audio, weather, traffic rules, trams and rail, save games, mods, editor, release) the table says what exists, which milestone carries it, and what is missing. A gap is called **"no milestone"** only after a search of `ROADMAP.md`, the milestone files and the vision for the topic found nothing that plans it (searches: career, scenario, OpenStreetMap, GTFS, depot, transfer, demand, scoring, rail signalling, multiplayer, trolleybus). A topic that is *out of scope* of a milestone is listed as such, with the milestone that says so.

Nothing here was tested in the running game. "Done" is what the roadmap and the code say; whether it feels right is the project lead's judgement, as for the items in the Review column of the [board](../../dev/BOARD.md).

## The areas

| Area | What exists | Open inside existing milestones | Not planned by any milestone |
| :--- | :--- | :--- | :--- |
| **Vehicle simulation** | Physics and dynamics ([M-0022](../roadmap/phase-4/M-0022.md), [M-0073](../roadmap/phase-10/M-0073.md)), the script VM, articulated consists, passenger mass ([M-0124](../roadmap/phase-12/M-0124.md)), cold starts of 948 buses | The driver's workplace: dashboards and printers ([M-0141](../roadmap/phase-4/M-0141.md), [M-0100](../roadmap/phase-4/M-0100.md)); trams beyond the NF6D; `[couple_front]`; per-unit passenger load | |
| **World** | Tile streaming, terrain, water, sky, sun, shadows, mirrors, session switching of maps ([M-0108](../roadmap/phase-11/M-0108.md)) | Water textures ([M-0067](../roadmap/phase-10/M-0067.md)); LOD and instancing ([M-0029](../roadmap/phase-5/M-0029.md)); content defects ([M-0090](../roadmap/phase-10/M-0090.md)) | City generation from OpenStreetMap, named in the vision ("Cities built from OpenStreetMap data", "Import pipelines for … OSM data") |
| **Transit** | Routes, timetables and IBIS data ([M-0099](../roadmap/phase-4/M-0099.md)); scheduled buses, trams and trains that follow the timetable ([M-0112](../roadmap/phase-11/M-0112.md)) | Delay semantics `GetTTDelay` (Q-TT-05) | Depot dispatch and rosters beyond the first and last trip, inter-line transfers and timetable editing (all stated **out of scope** by M-0112); GTFS and GTFS-RT (only "compatibility notes", [M-0004](../roadmap/phase-1/M-0004.md), untouched since 2026-10-04) |
| **Passengers and ticketing** | Stops, boarding and alighting through the scripts' doors, tickets from the map's pack, seating, standing room, mass ([M-0101](../roadmap/phase-4/M-0101.md)) | Cash desk, `GivenTicket` and change giving; pedestrians at crossings and signals ([M-0050](../roadmap/phase-4/M-0050.md)) | **Passengers with destinations, waiting behaviour, demand and transfers** (vision: "Passenger agents with destinations", "Transfers between routes"). The ROADMAP carries "passenger journey routing" as an open sub-item with no milestone |
| **Stops and displays** | Script textures, destination boards, text textures ([M-0094](../roadmap/phase-4/M-0094.md), [M-0039](../roadmap/phase-4/M-0039.md)) | Dynamic passenger information on scripted stops (`GetTT*`, [M-0099](../roadmap/phase-4/M-0099.md)/[M-0100](../roadmap/phase-4/M-0100.md)); some printers and TFTs | |
| **Audio** | Sound engine with buses, voices and distance fading ([M-0027](../roadmap/phase-4/M-0027.md)) | The player's vehicle ([M-0102](../roadmap/phase-4/M-0102.md): stop announcements, force feedback), scenery ([M-0139](../roadmap/phase-4/M-0139.md)) and AI sounds ([M-0140](../roadmap/phase-4/M-0140.md)); `[volcurve]` script variables and `[3d]` fall-off (spec); tunnel reverb ([M-0115](../roadmap/phase-11/M-0115.md)) | |
| **Weather** | Presets, sky and fog read from the map | Real-weather research ([M-0116](../roadmap/phase-11/M-0116.md)); wet and snow grip, exhaust ([M-0117](../roadmap/phase-11/M-0117.md)); dirt, rain, wet and snowy surfaces ([M-0120](../roadmap/phase-12/M-0120.md), [M-0121](../roadmap/phase-12/M-0121.md)) | |
| **Traffic rules and signals** | AI traffic on lanes, speed limits, density groups, traffic lights, yellow by rowdiness, parked cars as obstacles ([M-0049](../roadmap/phase-4/M-0049.md), [M-0095](../roadmap/phase-4/M-0095.md), [M-0111](../roadmap/phase-11/M-0111.md)) | Overtaking of a stopped vehicle, level-crossing barriers (no data hook), red running (needs a product decision) | |
| **Trams and rail** | Train formations ([M-0106](../roadmap/phase-10/M-0106.md)), rail controls, points as shared state, scheduled trains, scenario tests ([M-0113](../roadmap/phase-11/M-0113.md)) | The player's click on the points tongue (needs picking on scenery meshes) | Railway signalling and interlocking (stated **out of scope** by M-0113 and M-0049) |
| **Save games** | Versioned sections ([M-0069](../roadmap/phase-10/M-0069.md)): vehicles with their script variables, camera, clock, weather, counters, the passenger-count override | | **The live world**: AI traffic, pedestrians, passengers on board, timetable progress and announcements are **not saved** ([session save](../architecture/session-save.md), "Not restored yet"); they are re-seeded on load |
| **Game layer** | Free play in a loaded map; end-of-session summary ([M-0062](../roadmap/phase-9/M-0062.md)) | | **Career with progression, scenarios to play, scoring and feedback** (vision: "Single-player career mode with progression", "Scenario editor"); multiplayer is "long-term" in the vision only. [M-0010](../roadmap/phase-1/M-0010.md) documents the OMSI *scenario file* format, not a scenario system. A headless punctuality statistic exists ([M-0112](../roadmap/phase-11/M-0112.md)) but nothing shows it to the player |
| **Mods** | Repaint discovery and load-time liveries ([M-0032](../roadmap/phase-6/M-0032.md)); vehicle override design ([M-0135](../roadmap/phase-14/M-0135.md)) | Mod directory layout and load priority ([M-0031](../roadmap/phase-6/README.md)); runtime livery switching; creator guides and the package linter ([M-0033](../roadmap/phase-6/README.md)) | |
| **Editors and tools** | `opts-tools inspect`; the editor mode is specified ([M-0051](../roadmap/phase-6/M-0051.md)) | Editor phases E1–E6; script generator and visual script editor ([M-0092](../roadmap/phase-6/M-0092.md), [M-0055](../roadmap/phase-6/M-0055.md), [M-0056](../roadmap/phase-6/M-0056.md)); mesh editor ([M-0137](../roadmap/phase-14/M-0137.md)) | |
| **Release** | CI on Linux, Windows and macOS is green; the settings and wizard are in | Release matrix and packages ([M-0034](../roadmap/phase-7/README.md)); end-user manuals (`docs/src/user/` does not exist yet, [M-0035](../roadmap/phase-7/README.md)); rebinding for wheels and gamepads; `v0.1.0-alpha` ([M-0036](../roadmap/phase-7/README.md)); there is no Credits screen | |

## What is not planned (the finding)

These gaps have no milestone. They are proposals, in the order in which they build on each other; each needs the project lead's approval and then the intake steps of [CONTRIBUTING](../../../CONTRIBUTING.md) (an item, a milestone file, a board entry) before work starts. No milestone file has been created for them.

1. **Player feedback of the service** (small). The punctuality, missed stops, tickets sold and revenue already exist as counters; show them in the end-of-session summary. It makes a drive mean something and is the first step of any career layer.
2. **A passenger journey model** (large, core). Passengers with a destination, a waiting time and a transfer, instead of people who board at a stop and ride for a fixed number of stops. It builds on [M-0101](../roadmap/phase-4/M-0101.md) and [M-0112](../roadmap/phase-11/M-0112.md). Design first, like [M-0135](../roadmap/phase-14/M-0135.md): the data (demand per stop and time) has no source in the OMSI files yet ❓.
3. **Saving the live world** (medium). The save system takes new sections without change; the work is to make traffic, passengers and timetable progress capturable and restorable. It is worth doing after 2, because the passenger state would change.
4. **Depot dispatch, rosters and timetable editing** (medium). The step from "a bus spawns at the route start" to "a vehicle is sent out and comes back"; M-0112 names it as its boundary.
5. **A scenario and career layer** (large, long-term). Design first. It needs 1 and 2.
6. **OpenStreetMap and GTFS import** (large, long-term). Research first: licences (ODbL, feed terms), what map format OPTS would write, and how it relates to the OMSI map format. [M-0004](../roadmap/phase-1/M-0004.md) holds only compatibility notes.

Railway signalling, multiplayer and a damage model are deliberately not proposed here: the milestones that touch them put them out of scope, and the vision calls multiplayer long-term.

## Gaps that are not features

* **Verification in the running game.** Seven items sit in the Review column of the board, some since 2026-10-11, waiting for the project lead's look. Most work in this analysis was checked headlessly (tests, protocol, corpus), which finds defects but not whether a bus feels right. A short regular session of looking, or a recorded checklist per milestone, would clear the column; this is a process question, not a milestone.
* **End-user documentation.** There is no `docs/src/user/`, and the Controls page still shows one "Feature not available yet!" notice. Neither blocks development; both block a release.
* **Content fidelity.** [M-0090](../roadmap/phase-10/M-0090.md) and [M-0066](../roadmap/phase-10/M-0066.md) carry the defects found on real maps. They shrink with each report but cannot be closed by inspection of the code.

## How to use this

The project lead can mark each proposal *wanted*, *later* or *no*; an approved one becomes an item and a milestone by the intake rule, and enters the [priority list](../../../priority-list.md) at the tier the project lead chooses. The M-0074 task "expansion pass" is done when this has been reviewed.
