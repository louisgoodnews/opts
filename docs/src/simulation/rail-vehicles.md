# Rail Vehicles in OPTS — Research and Design

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Research and proposal (2026-10-11); nothing implemented |
| **Premise** | OMSI treats trams and trains as buses and cars: `.bus`/`.ovh` files with a type flag, road-style axle physics, rail only as a path type. OPTS does **not** have to: rail vehicles get their own domain model, with the same assets. |

## 1. What the data says

| Fact | Count / source |
| :--- | :--- |
| Playable vehicles with `[type] 2` (rail) | **52** `.bus` (e.g. `Tram_NF6D/NF6D_A.bus`) |
| AI/other vehicles with `[type] 2` | **841** `.ovh` (e.g. `A3/A3L_67_K.ovh`) |
| Aircraft (`[type] 3`) | 7 `.ovh` |
| Road vehicles | 1,435 `.bus`, 397 `.ovh` |
| Spline profiles with `[rail_enh]` / `[third_rail]` | 419 / 348 (`[rail_enh]`: rail length, joints, wave length/amplitude/power in x and z: track irregularities) |
| `.sco` objects with rail `[path]` (type 2) | ≈ 1,275 (junctions, crossings, points) |
| `.sco` objects with `[switch]` | 282 (points: number of positions; `[switchdir]` selects a position for the following `[path]`) |
| Timetable tracks | `.ttr` files (3,588; `[track_entry]` lists the rail path of a trip, 713 k entries) |

Structure of a rail vehicle: `NF6D` is **many small `.bus` units**: cars (`NF6D_A/B/C`), bogies as separate vehicles (`NF6D_DrehgestellV/H`), articulation units (`NF6D_Gelenk_*`), couplers (`NF6D_Kupplung_DT`), with `[coupling_front]/[coupling_back]` and ~30 `[control_cable_front]` records (multiple-unit control lines). Wheelsets are `[newachse]` records with track width 0.984–1.016 m (gauge play) and a wheel diameter of 0.58 m. `[paths]` names the passenger paths, `.hof` depots, `Chrono` timetables hold `.ttr` tracks. Rail-specific physics (adhesion, traction curves, brakes) is in scripts (`.osc`) and constants, not in `.bus`.

## 2. How OMSI models it (to deviate from)

* A rail vehicle is a road vehicle whose axles are constrained by the **rail path** (type 2) instead of steering input; the simulation keeps the "bus" structure (steering, pedals mapped to traction/brake).
* The path of a track is geometric: lane ends within tolerance connect; points are `.sco` objects with `[switch]` positions chosen at run time or by timetable (`.ttr`).
* AI rail traffic follows `.ttr` track entries; signals are `.sco` traffic lights bound to paths (`[use_traffic_light]`) plus `signalroutes.cfg`.

## 3. OPTS design

### 3.1 Domain model (`opts-sim::rail`, no Bevy)
* **Track graph** from type-2 paths of splines and objects: nodes = lane ends joined geometrically (the rule of the road/pedestrian graphs), edges = lanes with a clothoid-free arc/line geometry (the same arc maths as the ribbons), gradient and **cant** from `halfcantwidth`/profile height; **points** as nodes with `switch_positions` and a *state* (position index) — the graph exposes only the active edge for movement.
* **Train**: ordered **consist** of units (the coupling chain already supported for articulated buses, `[couple_back]`), each unit with 1..n **bogies**; a bogie runs on the track as a point pair (two wheelsets) at arc length `s`; the car body pose derives from its two bogies (position midpoint, heading from the chord, pitch from heights; roll from cant). This is what a road-physics model cannot do and what makes curves and points look right.
* **Longitudinal dynamics** along the path: tractive effort from the vehicle's constants/scripts, davis-type running resistance (`A + B·v + C·v²`), gradient force `m·g·slope`, curve resistance, adhesion limit `μ·N` (wet/dry from weather), brake force per unit (service, emergency, electro-dynamic), slack/forces at couplings. One coordinate `s` per bogie, no wheel steering, no lateral tyre model.
* **Track-quality** `[rail_enh]` as small lateral/vertical displacements of the bogie (noise with the given wavelength, amplitude, power) for ride feel and wheel/rail sound.
* **Power supply**: overhead line/third rail as a boolean availability per section later; not in the first version.

### 3.2 Player-driven rail vehicles
* Driver controls: master controller (traction/brake notches), brake handle, direction, doors, pantograph, horn/bell; keyboard mapping extends `keyboard.cfg` actions (`[vehicles]` trigger names: the script contract stays, so existing `.osc` cockpit logic runs — see the script VM). The **physics outputs** the scripts expect (speed, acceleration, pressures) come from the new model through the same named variables, so unmodified OMSI scripts keep working.
* Camera/cab: unchanged cameras; the vehicle origin follows the leading bogie.
* Point control: automatic route setting from the timetable track (`.ttr`) or manual lever (`input`: `switch.toggle id`); the graph rejects a move over a point set against the vehicle (or flags a derailment risk: speed ≤ limit rule).
* Signals: a signalled block model (`stop-and-proceed`), driver must obey; later ATP-lite. Start with "advisory" only.

### 3.3 AI rail traffic
* Trip = `.ttp` stops + `.ttr` track entries → **route over the track graph** (A*/Dijkstra with turn restrictions from points); departure from the `.ttl` tour; dwell from `Busstops.cfg`/profile times (`[profile_man_dep_time]`…).
* Movement: the same longitudinal model with a **speed profile** (line speed limits from `[rule] speedlimit`, curve limits `v ≤ sqrt(a_lat·R)`, station stops with braking curves, signal stops); no collision physics, only **headway control** (block occupancy; minimum distance to the vehicle ahead).
* Spawning/despawning at depots/stubs like road AI; LOD: far trains reduce to a moving point (no bogie meshes).
* Shared with the road AI (parallel session): timetable scheduler, signal states, density curves — the rail agent plugs into the same scheduler with a different mover.

### 3.4 Crossings and mixed traffic
Level crossings are `.sco` with both road and rail paths and a signal group; road AI treats the crossing as a conflict zone whose state comes from the rail signal; trams on street track share the road lanes: **embedded track** profiles carry road (`0`) and rail (`2`) paths side by side (the composition rule, [lane kinds](lane-kinds.md)); road vehicles may use the rail lane in the same direction only where the profile allows (open question ❓), trams yield to none.

## 4. Rendering and assets
No change in loaders: meshes, `[newanim]` for bogie swing/wheels, pantograph by variables. New: bogie transform hierarchy driven by track geometry (each bogie entity placed at its track pose, the body between them), cant roll, and gauge-correct wheelset placement. Overhead wires and rails already render as spline ribbons (rails need the profile's cant).

## 5. Phases
| Phase | Deliverable | Acceptance |
| :--- | :--- | :--- |
| R0 | Corpus audit: all type-2 vehicles' axle layouts, couplings, constants; rail paths of maps; points | report in `docs/src/meta/`; list of unsupported features |
| R1 | Track graph from splines/objects incl. points; viewer overlay | every rail path of 3 reference maps connected; point states switchable |
| R2 | Bogie/consist kinematics on the graph (no dynamics): a static train pulled along a route | wheels stay on rails around curves and points; no visible slip |
| R3 | Longitudinal dynamics, brakes, adhesion, `[rail_enh]` noise | stopping distances match published values within 10 % for 2 reference vehicles |
| R4 | Player cab controls through existing `.osc` scripts | an unmodified tram cockpit works (doors, controller, display) |
| R5 | AI timetable rail traffic with headway control | a line runs a day without deadlock on a reference map |
| R6 | Signals, level crossings, mixed road/rail | road AI waits at closed crossings; tram obeys signals |

## 6. Risks and open questions
* OMSI's rail physics lives in scripts and unspecified engine behaviour ❓; reproducing *behaviour* (not code) must be validated by measurement in OMSI with test vehicles; no decompilation (ADR-0002).
* `[third_rail]` identifiers (`9030`), `.ttr` fields, `[switchdir]` semantics, and how scripts read speed/axle variables for rail units are open in the specifications.
* Composite-vehicle scripts assume OMSI's road-style variable names (`Wheel_Rotation*`, steering); the compatibility layer must map them.
* Performance: long consists (6–8 units × 2 bogies) at 60 fps with AI — budget per train ≈ 0.1 ms.
