# Train formation editor: implementation report (2026-10-15)

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0106](../roadmap/phase-10/M-0106.md) (first deliverable) |
| **Branch** | `feat/train-formation-editor` (13 commits on master `839651f`, 51 files, +5,662/-84) |
| **Design** | [train-formation-editor.md](../architecture/train-formation-editor.md), [ADR-0013](../adr/0013-train-formation-model.md) |
| **Research** | [train-formation-research-2026-10-15.md](train-formation-research-2026-10-15.md) |
| **Local CI** | passes (fmt, clippy `-D warnings`, tests, rustdoc, mdbook) |
| **Clean room** | no OMSI binary inspected, nothing of the installation modified, no OMSI asset committed (screenshots of vehicles were taken for checking and not added to the repository) |

## 1. What was implemented

| Part | Where | Notes |
| :--- | :--- | :--- |
| `.zug` parser and model | `opts-core/src/formats/trains/` | `TrainFormation`, `FormationSegment { vehicle, orientation, powered }`, `VehicleRef` (file or group), `Orientation`, `FormationSource`; CP1252/UTF-8/UTF-16, CRLF, missing final newline; script text refused by shape; spec page [`.zug`](../formats/text/zug.md) |
| Editing operations | `trains/edit.rs` | add, remove, move, turn, powered cycle, reverse, suggested name |
| Validation | `trains/validate.rs` | `VehicleLookup` trait, `FormationIssue` V1-V9, every problem reported, German/English keys reserved |
| Consist dynamics | `opts-sim/src/rail/dynamics.rs` | `ConsistLoad { total, powered, braked }`, `RailDynamics::for_consist`; the single-body tram is the special case and unchanged |
| Placement | `opts-game/.../omsi_world/consist_plan.rs` | pure planner: head chain plus each car with its own chain, turned units mirrored, ring-closing chains end their unit; `coupling_joint` shared with the AI loader |
| Spawner | `omsi_world/vehicle.rs`, `respawn.rs`, `rail.rs` | follows the plan; meshes of every unit are prewarmed; formations snap to the nearest track within 150 m; `set_consist` gives the whole train's masses to the head's `RailPlayer` |
| Protocol 1.6.0 | `opts-game/src/formation.rs`, `opts-cli` | `formation.validate`, `formation.spawn`, `formation.save`, `formation.list`, `formation.delete`; `saved`, `zug`, `cars`, `reverse` parameters; `menu.do formation <command>` |
| Editor | `opts-game/src/in_game/formation_menu/` | the *Formation* tab of Select Vehicle: Rail cars (877), Trains (223 `.zug`), Saved; train rows with Up/Down/Turn/Pulls/✕; summary with every problem; entry points; Reverse, Clear, Rename, Save, Spawn |
| Persistence | `opts-game/src/formation/store.rs` | TOML schema v1 in `<user data>/formations/`, atomic writes, unique ids, damaged files listed and kept; the session vehicle record carries `formation` so a save rebuilds the train |
| Catalogue | `opts-catalogue` | `rail_vehicles()`, `train_consists()` |

## 2. Verification (live, Grundorf and Tram maps)

| Scenario | Result |
| :--- | :--- |
| Single NF6D (`NF6D_B_User.bus`) | placed on the track 0.4 m from the entry point, accelerates at the consist model's 1.1 m/s² (40 t) |
| **Three NF6D units, the middle one turned** | validates, spawns coupled (35 files), cabs meet at the couplers (side view), drives at 1.08 m/s² |
| `Trains\regio_grundorf.zug` (BR218 + 3 coaches, two turned) | spawns on the track 82 m from the entry point, 0.42 m/s² (model: 0.41) — checked again after the planner was introduced |
| `Trains\TH_Gueter_Kohle.zug` (21 cars, about 880 t) | spawns, 0.12 m/s² |
| Editor (page) | preset loaded, 4 cars / 200 t / no problems; entry chosen; *Spawn* spawns; a turned first car shows its problem; search 877 → 78 |
| Save, restart, list, spawn from `saved` | file written in the documented schema, listed after a restart, spawned (3 cars) |
| Session save and restore | a save with a train restores both vehicles through the spawner, the train rebuilt (57 files in all), drives |
| Invalid formations | empty, one car, missing file, non-rail vehicle, group-only `.zug`, unknown saved id are all refused with the problem named by car |

Automated: about 80 new tests (parser 14, validator 9, editing 5, planner 8, store 9, protocol parameters 5, editor 12, dynamics 6, coupling maths 7, session save 1, corpus 1 (needs `OMSI_PATH`; 222 of 223 `Trains/` files parse, `RUHR.zug` refused, 0 `.bus` references), catalogue 2).

## 3. Deviations from the design

| Design | What happened | Why |
| :--- | :--- | :--- |
| Powered cars = those with a driver workstation | **Powered = the first car and its chain, every multi-file car, and any car the user marks**; single-file cars behind the head are hauled; per-car `powered` override | measured: every rail `.ovh` checked (loco, coach, wagon) has a driver camera, `[sound]` and `[script]`, and wagons list driven axles; the rule made every train accelerate like its locomotive alone |
| Chain units cannot be turned in v1 | **They can**; the planner mirrors the unit as a whole | the placement moved into a pure, tested planner, which made it a few lines; it was the headline use case |
| V1-V8 | V9 added: the first car must face forward (the player drives it); *Reverse train* turns a preset | a preset can start with a turned loco |
| Spawn at the nearest track as today | **within 150 m** for formations (12 m kept for single rail vehicles) | entry points are road-oriented; the Grundorf station track lies 82 m away |
| 40 parts, proposed 64 | 64 for a formation; the NF6D's 41 per unit was an artefact of following the ring | `JobLookup` now stops at the reversed link like the spawner does |
| German and English strings for the page | English only | the rest of the vehicle menu is not localised; the issue keys exist |
| *Load* combobox | a **Saved** list (searchable) | 223 presets need a search field; a combobox would not scale |
| Prewarm of the head only | prewarms every unit behind the head | without it cars came up without bodies (meshes=0); also benefits plain chain vehicles |
| `.zug` specified in `misc-types.md` | own spec page | the format is fully understood and corpus-checked |

## 4. Limits and known issues

- **Top speed 70 km/h** for every train (the existing rail constant); a 160 km/h set is capped. Not part of this milestone.
- **No coupler dynamics**: no slack, push-pull, per-car brake failure or consist collisions (ADR-0013 lists what would reopen the decision).
- **"Powered" is a heuristic** (Q-TF-01): the vehicle files do not tell a locomotive from a wagon; a three-unit EMU pulls with all units, a loco in the middle of a rake needs the *Pulls* toggle.
- **Group names** in `.zug` files (18 files) are parsed and kept but cannot be spawned outside a map's AI lists; the formation tab's *Trains* list shows them and validation names them.
- **Mass of a lone tram changed**: its mass now counts all cars (the NF6D: 40 t instead of the 20 t floor).
- **The rail network is a per-tile lane graph.** One early session on the Tram map (cold start, long tile streaming) showed an NF6D accelerating with road physics at 8 m/s² and exceeding 70 km/h; later sessions with the same code behaved correctly. The cause was not established (hypothesis: the train's lane indices went stale while tiles streamed in). Pre-existing code (`RailNetwork`), not changed here; worth its own investigation.
- **Restoring a save waits at the load screen** when the map has a load problem (`Continue` button, the Tram map lists a missing tile); the control protocol needs `ui.click loading.continue`.
- **No mouse-driven UI test**: the editor was exercised through `menu.do` and the protocol; the button actions have unit tests.
- **Windowed runs on the user's display can be closed by the user**; several verification runs were repeated headless for that reason.

## 5. Follow-up milestones

| Proposal | Content |
| :--- | :--- |
| **M-0107** (not written) | AI trains from `.zug`: the 53 AI lists that name a `.zug` are silently ignored by `load_rail_variants`; the parser, planner and resolver make it a small change; group-name resolution against `ailists.cfg` |
| Cab switching and passive cars' scripts | with M-0098 (consist script host and control cables) |
| Top speed and traction from the vehicle | read from the scripts/constants once the script drive is the rail default |
| Rail network stability | stale lane indices while tiles stream (see 4) |
| Track picker, 3D preview, `.zug` export | as in the design |
| Localisation of the vehicle menu | including this page |

## 6. Process notes

- One CI failure per commit on average came from generated files (protocol reference) and review lists (input gating audit, file size rule); each was fixed in a follow-up commit.
- The earlier rule against `pkill -f` matching its own command line cost one aborted command again; no state was lost.
