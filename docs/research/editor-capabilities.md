# Editor capabilities: maps, timetables and routes, scripts and vehicle behaviour

| Metadata | Value |
| :--- | :--- |
| **Status** | 🔬 Research, not decided |
| **Date** | 2026-11-22 |
| **Relates to** | [M-0051](../src/roadmap/phase-6/M-0051.md) editor mode; [ADR-0008](../src/adr/0008-editor-mode.md); [editor README](../src/editor/README.md) |

## 1. How to read this file

Each section lists capabilities. For every capability:

* **Priority.** *Must* means the editor is not useful for that content without it. *Should* means creators will notice its absence within a week of use. *Could* means a convenience or a later phase.
* **Exists.** What is in the repository today, with the file or milestone that shows it.
* **Gap.** What is missing.
* **Milestone.** The editor work package (E0 to E6 in the [editor roadmap](../src/editor/roadmap.md)) or the milestone that would own it.

The editor specification already fixes the principles (P1 to P8: non-destructive, lossless, what-you-see, one schema, early validation, protected content, unknown stays unknown, undoable). This file does not restate them. It asks what each content type needs on top of them.

**Existing OPTS tools that can be reused.** The control protocol ([control-protocol](../src/architecture/control-protocol.md)) gives input injection, `sim.step`, `reload.asset`, `drive.set`, `drive.trigger`, `vehicle.script_vars` and screenshots. Hot reload ([hot-reload](../src/architecture/hot-reload.md), [ADR-0007](../src/adr/0007-hot-reload.md)) covers meshes, textures, `.cfg`, `.bus`, `.sco`, `.sli`, `.hum` and map tiles. The `opts-tools` binary exists ([crates/opts-tools](../../crates/opts-tools)) but has no editor commands. The in-game mesh editor [M-0137](../src/roadmap/phase-14/M-0137.md) is proposed, not started, so it cannot be relied on yet. No tool named "content tool" exists in the repository; the nearest items are those listed here.

## 2. Maps

| Capability | Priority | Exists | Gap | Milestone |
| :--- | :--- | :--- | :--- | :--- |
| Edit the manifest (`global.cfg`): name, years, seasons, ground textures, entry points, tile list | **Must** | Writer for `global.cfg` ([`write/manifest.rs`](../../crates/opts-omsi-map/src/formats/map/write/manifest.rs)); parser ([global-cfg](../src/formats/map/global-cfg.md)) | No editor UI; the tile list is not yet checked against the folder | E2 |
| Place, move, copy and delete scenery objects in a tile | **Must** | Writer for tile `[object]` records ([`write/tile.rs`](../../crates/opts-omsi-map/src/formats/map/write/tile.rs)). The writer refuses splines, rules, attachments and flags | No placement UI. Object records beyond `[object]` have no writer | E2 (object tools) |
| Draw and edit splines (roads, rails, fences) with snapping, connections and gradients | **Must** | Spline rendering and terrain coverage exist as examples ([`spline_terrain_cover.rs`](../../crates/opts-bevy/examples/spline_terrain_cover.rs), [`spline_census.rs`](../../crates/opts-bevy/examples/spline_census.rs)). Terrain carving is implemented ([terrain carving](../src/architecture/terrain-carving.md)) | No spline writer; the chain-gradient rule is specified, not tested | E2 |
| Add and remove tiles; map overview with the tile grid | **Must** | The grid is read by the streaming code ([M-0024](../src/roadmap/phase-4/M-0024.md), partly delivered) | Overview and tile creation do not exist | E2 |
| Terrain height editing | **Should** | Terrain loading and carving ([terrain.rs](../../crates/opts-bevy/src/terrain.rs)) | Terrain writer. The `.map.terrain` file is fixed at 14,888 bytes and its field meanings are partly ❓ ([tiles](../src/formats/map/tiles.md)) | E2 |
| Ground painting and light-map bake | **Should** | Light maps are read ([tiles](../src/formats/map/tiles.md)) | No writer; the bake is not specified beyond E2 | E2; light-map work in [M-0120](../src/roadmap/phase-12/M-0120.md) |
| Bus stops and their placement on the road | **Should** | Stops are detected in the world ([M-0099](../src/roadmap/phase-4/M-0099.md), status sync 2026-10-25) | No stop tool; `Busstops.cfg` has no writer | E5 (stop tool) |
| Tunnels and spatial zones (where the ceiling and the tunnel rules apply) | **Should** | Zone model, tracker, sampler and `zones.query` are in progress ([M-0115](../src/roadmap/phase-11/M-0115.md), protocol 1.36.0); the ceiling rule exists ([spatial zones](../../crates/opts-spatial-zones/src/lib.rs)). Tunnel splines empty in some cases ([ISS-012](../dev/issues/ISS-012-tunnel-zones-empty-splines.md)) | No authoring of zones. The zone definition in the map format is not in the editor spec | Proposed: E2, after M-0115 |
| Scenery lighting: light rooms, cones, sun and moon | **Could** | Lighting is specified and partly rendered ([lighting system](../src/architecture/lighting-system.md), [lighting cones](../src/architecture/lighting-cones.md)) | No authoring; the editor spec does not list lights as a tool | Proposed: E3 or later |
| Traffic rules and signals (`[rule]`, `[traffic_light]`) | **Should** | Signals in the traffic crate ([`signals.rs`](../../crates/opts-traffic/src/signals.rs)) | Rule tool and signal panel; validation rules MAP-* and TRF-* are specified only | E2 (rule tool), E5 (signal programs) |
| Undo, redo and lossless save of the tile | **Must** | Principle P2 and P8 in the [editor README](../src/editor/README.md) | No command bus exists; no `opts-edit` crate | E1 |
| Validation while editing (references, overlaps, missing tiles) | **Should** | Rule catalogue in [validation](../src/editor/validation.md) | No engine; the catalogue is a specification | E1 (first rules), E2 (map rules) |

**Assessment.** The map gap is writers, not readers. Readers and renderers are mature enough that the editor can reuse them (P3). The first writer milestone is the tile `[object]` record, which exists, followed by splines, which do not. The spline writer is the most valuable single item.

## 3. Timetables and routes

| Capability | Priority | Exists | Gap | Milestone |
| :--- | :--- | :--- | :--- | :--- |
| Read the line, trip and track files (`.ttl`, `.ttp`, `.ttr`) | **Must** | Route layer and timetable callbacks ([`route.rs`](../../crates/opts-bevy/src/route.rs), [`timetable.rs`](../../crates/opts-bevy/src/host_callbacks/route_callbacks/timetable.rs)); [M-0099](../src/roadmap/phase-4/M-0099.md) done 2026-10-19 | Field meanings are mostly unverified ([timetables](../src/formats/map/timetables.md), status draft) | done for read |
| Write the line, trip and track files | **Must** | None. No timetable writer exists in `crates/` | Writers start as preserve-only for unknown fields (E0 spec gap). The field meanings must be measured first | E5 |
| Edit lines and trips: stop lists, timing profiles, station links (`StnLinks.cfg`) | **Must** | Parsed as keyword files ([timetables](../src/formats/map/timetables.md)) | Editor forms, no writer | E5 |
| Day types and chrono variants (dates, deactivated lines, ticket pack) | **Must** | Chrono reading ([chrono](../src/formats/map/chrono.md)); day-type circuits are planned in [M-0155](../src/roadmap/phase-16/M-0155.md) | Chrono editing exists only in the E5 timeline plan | E5; M-0155 for circuits |
| Validate against the stops and the map: each trip stop exists, each track follows the road or rail, times are ordered | **Must** | Rule IDs TT-* are named in [validation](../src/editor/validation.md) | No rules implemented; the track check needs the spline data of section 2 | E5 (after E2 splines) |
| Export to the existing OMSI timetable layout, so OMSI 2 and OPTS both load it | **Must** for the OMSI path | None | Depends on the writer above and on OMSI acceptance checks, which cannot be automated ([editor roadmap risks](../src/editor/roadmap.md)) | E5, manual acceptance checklist |
| Export to an OPTS-native timetable format, with GTFS input | **Should** | GTFS format page exists ([gtfs](../src/formats/map/gtfs.md)); the vision lists GTFS ([vision](../src/vision.md)) | No importer or exporter in `crates/`; no OPTS-native timetable type in [`opts-model`](../../crates/opts-model/src) (its files are `lib.rs`, `source.rs`, `vehicle.rs`) | Proposed: a new milestone after M-0099 |
| Generate runs from headways (a frequency rule that writes many trips) | **Could** | None | Not specified. Writes many trips from one rule, so it depends on the validator | Proposed, E5 or later |
| Check a timetable against the scheduled AI traffic and the passenger model | **Could** | Scheduled AI traffic is planned ([M-0112](../src/roadmap/phase-11/M-0112.md)); passengers in [M-0101](../src/roadmap/phase-4/M-0101.md) (partial) | Cross-check tool not specified | after M-0112 |

**Assessment.** The timetable gap is the write path. Reading is done. The owner decision on OMSI-loadable output (decision 4 in the [README](README.md)) decides the size of this work: an OMSI-compatible writer needs the field meanings measured first, and an OPTS-native format needs no measurement but gives up OMSI compatibility.

## 4. Scripts and vehicle behaviour

| Capability | Priority | Exists | Gap | Milestone |
| :--- | :--- | :--- | :--- | :--- |
| Author and edit `.osc` text with highlighting and completion | **Must** | Language reference ([osc](../src/formats/text/vehicles/osc.md)); the lexer and compiler ([script-vm](../src/architecture/script-vm.md)) | No editor. Completion and symbol lookup are specified in [content-assets §5](../src/editor/content-assets.md) | E3 (script editor) |
| Compiler diagnostics while editing | **Must** | `ScriptEngine::from_sources` returns compile diagnostics ([script-vm](../src/architecture/script-vm.md)); every corpus script compiles ([content-assets](../src/editor/content-assets.md)) | Whether each diagnostic carries a file and line in the public API was not verified in this pass | E3 |
| Run-time diagnostics, each reported once | **Should** | `drain_diagnostics()` in the host interface ([script-vm](../src/architecture/script-vm.md)) | Not shown in an editor panel | E3 |
| Test a script in a rig with a scripted input profile | **Must** | Rig examples: [`script_rig`](../../crates/opts-bevy/examples/script_rig) and [`script_diag.rs`](../../crates/opts-sim/examples/script_diag.rs); the rig is described in [osc-runtime](../src/architecture/osc-runtime.md) | The headless rig as a library crate, `opts-script_engine`, does not exist ([M-0092](../src/roadmap/phase-6/M-0092.md), not started) | M-0092 |
| Inspect variables and triggers of the running vehicle | **Should** | `vehicle.script_vars`, `drive.controls` and `drive.trigger` in the [control protocol](../src/architecture/control-protocol.md) | No editor panel; the control server has open blockers ([ISS-001](../audit/issues-2026-11-18.md)) | E4, after ISS-001 |
| Hot reload of an edited `.osc` | **Must** for the dev loop | The watcher, debouncer and validate-then-apply pipeline exist ([`hot_reload`](../../crates/opts-bevy/src/hot_reload)) | `.osc` is not in the scope of [ADR-0007](../src/adr/0007-hot-reload.md); the module has no `.osc` handling. Script state must be migrated on reload, which ADR-0007 section 9 leaves to a separate design | Proposed: E4, own decision (owner decision 6) |
| Error reporting with line numbers in the editor | **Must** | As for compile diagnostics above | Panel and source mapping not built | E3 |
| Vehicle script generator from a specification (`VehicleSpec`) | **Could** | Specified ([script-generator](../src/architecture/script-generator.md)) | No `opts-script_editor` crate. [M-0055](../src/roadmap/phase-6/M-0055.md) not started | M-0055, M-0056 |
| Visual subsystem graph | **Could** | Specified in [script-generator §8](../src/architecture/script-generator.md) | No code; the owner must confirm it (decision 8) | M-0056 |

### 4.1 Rules that protect the simulation

These rules apply to every script the editor writes or runs. Most are already in the specifications; the table records where each is stated and whether the code enforces it.

| Rule | Stated in | Enforced in code |
| :--- | :--- | :--- |
| The OMSI installation is never written except by an explicit, previewed install | [editor README P1](../src/editor/README.md) | Not yet; no editor exists |
| Predefined variables are read-only for generated scripts; no undeclared variable | [script-generator §7](../src/architecture/script-generator.md) (lint) | Not yet; lint is specified |
| Determinism: the same seed gives the same run | [script-vm](../src/architecture/script-vm.md) (`seed_random`, fixed-index names) | Yes for the VM; the per-consist budget described in [osc-runtime](../src/architecture/osc-runtime.md) was not found by `grep` in `crates/opts-sim/src/script` |
| Bounded execution: the VM has no loops, and macro nesting is limited to 64 | [script-vm](../src/architecture/script-vm.md) | Stated; nesting limit not re-checked in code in this pass |
| A failing script does not stop the simulation; its last outputs are kept | [osc-runtime](../src/architecture/osc-runtime.md) (isolation) | Documented; not checked in code |
| A script from an untrusted author cannot read files outside its project | Not stated in any document | No. This is a gap for community content and is taken up in [scripting-dsl.md](scripting-dsl.md) |

## 5. Protected content

The editor treats scrambled meshes as read-only, as [editor README §6](../src/editor/README.md) states. Community scripts, sounds and meshes may be copied into a project only as file references by default. These rules are not new and this file does not change them.

## 6. Summary of the top gaps by priority

1. **Writers for splines and for timetables.** Both are *must* and both are missing. Splines are the most valuable map writer; the timetable writer is blocked by unverified field meanings and by the OMSI-or-OPTS decision.
2. **A headless script rig as a crate.** *Must* for testing; blocked by [M-0092](../src/roadmap/phase-6/M-0092.md), which is not started.
3. **Hot reload for `.osc`.** *Must* for the dev loop, and a change to ADR-0007 scope that needs its own decision.

Lower on the list, but worth tracking: the control server blockers (ISS-001 to ISS-003) must close before any editor action goes through the control protocol.
