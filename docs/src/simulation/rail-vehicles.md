# Rail Vehicles in OPTS — Research and Design

| Metadata | Value |
| :--- | :--- |
| **Status** | 🚧 R1–R3 first cut implemented (2026-10-11): track network, player tram, AI trains; see §7 |
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

## 7. Implementation status (first cut)

| Part | Where | Notes |
| :--- | :--- | :--- |
| Track graph | `opts-sim::pedestrian::{FootLane, PedestrianNet}` with `lanes_of_kind(.., Rail)` and `lanes_from_object_of_type(.., 2)` | The lane graph is reused; rail `[path]`s of splines and placed objects are loaded per tile (`omsi_world/rail.rs`, `RailNetwork`). Ends within 0.6 m / 0.8 m connect. |
| Train on the graph | `opts-sim::rail::Train` | Position as (lane, s, direction) plus a trail for poses behind the head; at a point, `Branch` (Straight/Left/Right) picks the exit. |
| Dynamics | `opts-sim::rail::RailDynamics::tram` | Tractive force with a power limit, Davis resistance, service/emergency/hold brake, adhesion limit, 70 km/h. Constants are **assumed** (typical tram values), not measured in OMSI. |
| Player tram | `RailPlayer` (inserted when the `.bus` has `[type] 2`) | **Controls (2026-10-22, project lead's scheme):** a power lever and a direction selector instead of pedals: **Q** moves the lever towards traction and **Y** (the key at the Z position of a QWERTY keyboard) towards braking while held, the lever stays where it is left (above 0 it is the throttle, below 0 the service brake); **W** selects forward and **S** backwards (a tram's own switches are pressed: the drive position and the reverse-drive switch `RFahrt_A`; the first move of the lever does the same when no direction key was pressed; a standing vehicle only changes its direction of travel, a backing one keeps facing the way it faced and its head moves back along the track it ran over); **`.` does nothing** (a rail vehicle has no parking brake of its own: its rail brake is a hot-spot of the cab, and the brake starts free); the **arrows** choose left, straight or right at the next point (they were Z/X/C). All of them are rebindable actions (`controls.drive.rail_*`). The chassis is snapped to the nearest track within 12 m (else ERROR, stays a road vehicle); `follow_ground` skips it; the coupled units follow through the existing articulation. Verified on `Tram` (Katenberg) with the NF6D: accelerates to 60 km/h, brakes to a stop. |
| AI trains | `AiTrain`, `--trains N` (default 3) | Rail vehicles of the map's `ailists.cfg` (`[type] 2`) with their `[couple_back]` chain, placed by their coupling distance behind the head; they appear 80–400 m from the camera on random tracks, run at ≈ 9 m/s and are removed at 600 m or at the end of the track. **Pool (2026-10-22):** `.zug` consists (up to 12 cars, joined by their coupling points) count as well as `.bus`/`.ovh` chains; when the map names a fleet of its own — a group called Tram/Strassenbahn/Variobahn/Stadtbahn or a depot group with a rail vehicle — only that fleet runs, otherwise every rail entry of the lists does (the Tram map's U-Bahn A3 cars and main-line trains are not its traffic; the depot's `NF6D_KI.bus` is an invisible control unit, the visible AI trams are the `Gladbeck_M6S` and `Gladbeck_Variobahn` consists). A train brakes (1.5 m/s², stopping 6 m short) for the player's vehicle and for units of other trains within 60 m ahead in its lane; the map's `unsched_*` density files are not read for rail yet. |

Not yet: reverse (R), switch state from the map's point objects (`[switch]`), `[rail_enh]` noise, bogie-accurate poses per unit, cab scripts (R4), timetables and headway (R5), signals (R6), and AI unit animation beyond wheel rotation.

### Unit placement (bogie-accurate)

Every unit of a consist (player and AI) is placed from the track, not from trailer kinematics: its origin lies `behind` metres behind the head, computed from the coupling points (`[coupling_back]` of the leader minus `[coupling_front]` of the follower; a unit coupled with the `true` flag hangs backwards, mirrored joint, rotated 180°, and closes the ring of a double-ended consist). The two bogies sit at `±y` of the unit's origin (`[boogies] y`, which for rail vehicles replaces `[rot_pnt_long]`; without it a 2 m chord is used). The body's origin is the midpoint of the track points at both bogies, its heading is the chord between them and its pitch the gradient along the chord. Very long cars on tight curves can still cut slightly (accepted). The bellows variables `articulation_<n>_alpha` come from the heading differences of consecutive units.

### Composition and head position

`VehicleComposition` (component on the head entity, also for road vehicles with trailers and for every AI train) stores the sub parts once at spawn: the head and the coupled units in order, each as a `Part` (entity, name, distance behind the head's origin, half bogie span, reversed flag, mesh and vertex counts). `find(name)`, `parts()`, `len()` and `length()` answer without walking `[couple_back]` or touching files. For a rail player the train's track point is the head's *front bogie*; the chassis position (`vehicle.get`, camera, HUD) is the **body's origin**, the midpoint of its two bogies (the train point lies one half span ahead of it), so reported and drawn positions agree. Rail players no longer get an `ArticulatedChain`: their units are placed on the track only.

## 8. What OMSI's own rail documents say (Tram SDK 1.2, NF6D manual, `.bus` tutorial)

Sources shipped with the install (read 2026-10-12, facts restated, nothing copied): the **Tram SDK 1.2** ("Bahn-SDK für OMSI 2", Philipp Zunk, 2017; `Vehicles/Tram_NF6D/manuals/Tram_SDK_1.1.pdf`, `…1.2.pdf`, German) written for map builders, the **NF6D manual** (Aerosoft, `Addons/Strassenbahn NF6D/Manual.pdf`) and the **in-file tutorial inside `NF6D_A.bus`** (German/English comments). The other 1,500 add-on PDFs contain no further rail technology (checked by keyword over all of them: only travel information and bus manuals). [SDK] = Tram SDK, [Man] = NF6D manual, [Bus] = `.bus` tutorial, [C] = corpus count.

### 8.1 Track side

| Fact | Source | Consequence for OPTS |
| :--- | :--- | :--- |
| OMSI is a bus simulator: rail vehicles are `.bus` files whose movement follows type-2 `[path]`s; user-driven trains only work reliably in OMSI 2.2.032 (2.3+ despawns them unpredictably) | [SDK] | The behaviours below are *workarounds* of that design; OPTS needs the data meaning, not the quirks |
| Rail `[path]` record: `2` (type), lateral offset, **height of the rail head above the spline datum (`0.449`)**, width (irrelevant for trams), direction (`2` both, `1` backward, `0` forward only) | [SDK] | Matches `lanes_of_kind(.., Rail)`; the rail-head height is the vehicle's ground height |
| `[realrail]` in `global.cfg` (41 maps): realistic points, AI trains set points, and trains run **on the spline centre line irrespective of the path offset**; on objects they follow the object's path | [SDK] | Rail lanes on splines use offset 0 for trains; objects use their paths. Maps without the flag: same rule (legacy behaviour unspecified ❓) |
| A player train that leaves its rail path (end of track, change to an opposite-direction path or a type-0 path) is **removed**; builders add tiny loops at the ends. All rail paths should be bidirectional (`2`) so trains can run both ways | [SDK] | OPTS stops the train at the end of a path (buffer stop) and never removes it; the diagnostics below flag one-way rail paths |
| A Y-fork without a point: the player train takes the **last-added** segment; AI trains err there, so builders use points | [SDK] | `Branch` choice (done) replaces the editor-order accident |
| Path connections with segments shorter than 0.5 m (better 1 m) make AI trains despawn; broken connections are the usual cause of random despawns | [SDK] | OPTS joins ends within 0.6 m; log a WARN with the position for rail segments under 1 m |
| **Spawning**: OMSI uses the *last-laid* rail path of the editor, the path must be long enough for the consist (30–40 m, longer for a double traction), loaded tiles are searched first, the entry point's influence is "unclear" | [SDK] | OPTS spawns at the rail lane nearest to the entry point that has room for the whole consist ahead of the head (and behind, for the reversed head); otherwise ERROR with the position and fall back to the entry point, never to the other end of the map |
| Collisions of player rail vehicles are always off; buses can collide with rail vehicles | [SDK] | Player trams are ghosts for road traffic; road AI and pedestrians still yield to trams |
| Points can be driven by **traffic-light phases** of a parent object (editor "parent to"): phases `0/1/2` are red, `6/7/8` green; `0` and `8` lock the point *straight*, `2` and `6` lock it *left/right*, `1` and `7` leave it adjustable, other phases leave it adjustable but re-set it on entering `0/8/6/2`; for a V-point "left" = straight. The lock keeps a point from switching under a train, otherwise the **train is torn apart** (the player may reset and rejoin) | [SDK] | `RailNetwork` points carry (state, locked) fed by the signal group of the parent object (`opts-traffic`); consist coupling does not break in OPTS, an unsafe switch is refused instead |
| AI trains: timetable, destination boards and `.hof` work; AI placement can be 20 m off and occasionally reversed; player takeover of AI trains is not supported (despawns); traffic-light *requests* by the player's train are impossible, AI trains trigger them from far away | [SDK] | OPTS can do better: place AI trains exactly, support takeover (the kinematic state is ours), and let a player tram raise requests |
| **Bidirectional operation**: bus stops expect forward entry and one door side; with the B head leading the stop does not see the vehicle, so builders add a second stop cube turned by 180° at the other end of the platform with its own invisible trip | [SDK] | OPTS' stop logic uses the *leading head's direction of travel* and the door side, so no mirrored cubes are needed |
| Doors: release needs a preselected side and standstill; a **double traction** needs a platform at least as long as the consist (`approachdist` of the stop cube) or passengers do not reach the rear car | [SDK], [Man] | Passenger boarding per part by its door position along the platform |
| Overhead wire height reaches the pantograph by the callback `GetHeightAbovePoint`, a constant, or the spline codes `9031–9047` (see `sli.md`) | [SDK] | Provide the callback from the ground/object index; the spline code wins when present |
| Track gauge is not part of the path: the NF6D has metre-gauge and standard-gauge bogies (`CTI_Drehgestelle`); minimum curve radius is about 15 m (20 recommended) for metre gauge and 27 m for standard gauge, below that the articulation shows errors | [SDK] | Report a WARN when a rail lane's curve radius is under the vehicle's limit; do not clamp |

### 8.2 Vehicle side

* **Unit structure and state bus.** A consist is a chain of independent `.bus` parts with their own script sets (`NF6D_A`, `…Gelenk_ACV`, `…DrehgestellV`, `…EEF_V`, `…C`, …). They exchange state only through `[control_cable_*]` links (see `bus.md`): variable pairs `SLR_x`/`SLW_x` copied between coupled partners every frame while the link's coupling variable is true. This resolves the open question of how parts share doors, power, timetable index, car number, brake and head activity.
* **Two cabs, one train.** Each head has its own scripts, its own IBIS device; the devices do not communicate and a head is active only after being switched on (`R` off, `N` on but cannot drive, `D` ready, with about 7 s of start-up sequence). Timetable data can only be accepted in the *first* part (the A head) and is forwarded to the B head over the control cables; battery, pantograph and timetable run from A. In a double traction only the two end cabs are usable. [SDK], [Man]
* **Direction** is the active head's: the same train drives "forward" with either head, which is why `E/N/R` do not mean what they mean for a bus. OPTS maps the three keys to the head state (script contract) and takes the travel direction from the active head.
* **Ride feel**: `[sinus]` (hunting motion of the wheel sets) and `[rail_body_osc]` (body sway), 186 and 294 definitions. They are small lateral/roll/vertical motions of the body on the bogies (optional second phase); `[rail_enh]` on the spline adds track irregularities.
* **Variants** are repaint-dependent: `.cti` `[setvar]` entries (the NF6D lists `CTI_Kupplung_FrontBRB`, `CTI_Drehgestelle`, `CTI_Wagennummer_DT2`, `CTI_Spiegel_Auto`, … ) set script variables at spawn; a double traction reads the second car's textures with a `_DT` suffix (`Wagenkasten_A_DT`). OPTS' repaint application must therefore also set the `[setvar]` variables on the vehicle's script engines per part.
* **Braking** constants of the player vehicle can be tuned with the constfile entry `Multiplikator_Bremse` (AI always uses `1`) — evidence that brake force is computed in scripts and only handed to the physics (`Brakeforce`).
* **Signal codes in the track** and the contact shoe: §8.1 and `sli.md`.

## 9. How OPTS runs trams without treating them as buses

| OMSI (bus simulator) | OPTS rule |
| :--- | :--- |
| Vehicle is a bus with rail-constrained axles; pedals, gears and steering exist | A rail vehicle has **no steering and no tyres**: head state (off/on/driveable), traction, brake notches; the track decides the heading |
| Pose from the vehicle's axle positions on the path | Pose from **bogie points on the track** (`[boogies]`), midpoint/chord/gradient (done) |
| `[realrail]` rules per map | Always: centre line on splines, object paths on objects |
| Leaving the path removes the train | Buffer stop at the end of the path, WARN for dead ends that are one-way |
| Entry point decides where a train appears, with undocumented side conditions | Nearest rail lane with room for the consist; explicit ERROR otherwise |
| Stop cubes only see forward-entering vehicles | Stops accept either direction; the door side comes from the active head |
| Signalling and speed limits hidden in `[third_rail]` | The contact shoe's track code is handed to the scripts as `volt_rail` (unchanged codes); OPTS itself only enforces what the scripts do with it |
| Parts talk via control cables | `[control_cable_*]` bridge between the part engines (script route VS1) |
| Points via parent-to traffic-light phases, trains torn apart on unsafe switching | Points have (position, lock); an unsafe switch is refused instead of tearing the consist |
| Collisions off for player trains | Same default, but road AI/pedestrians treat a train as an obstacle that does not yield |

### Implementation order derived from this

1. Rail lanes keep the **spline instance and profile index** so a lane can answer `[third_rail]` code and rail-head height at any `s` (needs `RailLane { third_rail: Option<f32> }`; corpus: 348 profiles).
2. Spawn rule (room for the consist ahead/behind), buffer stops, one-way and short-segment diagnostics.
3. Active head (`A`/`B`): direction from the head, camera/cab per head; double traction has two usable ends.
4. Points with lock state fed by the signal group; refusal instead of coupling break.
5. Contact-shoe query and `GetHeightAbovePoint`; then the control-cable bridge and per-part scripts ([vehicle scripts route](../architecture/vehicle-scripts-route.md)).
6. Ride-feel oscillations (`[sinus]`, `[rail_body_osc]`) last.

### 9.1 Status of the rules (2026-10-12)

* **Variants through `[setvar]`: works.** Choosing the NF6D repaint *Schoenau (Kurvenradius 27m)* in the vehicle menu sets `CTI_Drehgestelle = 1` on the head's script engine (checked with `vehicle.script_vars`); the coupled parts read their variables from the head engine (the OPTS form of `[scriptshare]`), so the standard-gauge bogies show on every part. Per-part script engines (route VS1) will give each part its own copy, set from the same repaint entries; the `_DT` texture slots of a double traction already follow the repaint's per-slot textures.
* **Two-headed operation and stop cubes: not yet implementable.** OPTS has no stop/passenger system yet (M-0050 open): there is no stop cube to mirror. The rule stands as a requirement for it: a stop evaluates the *leading head's direction of travel* and door side, so a train arriving with the B head first is recognised without a second, 180°-turned cube and invisible trip.

## Traction of several trams (2026-10-22)

A traction made in the formation editor (four NF6D, all facing forward: the A head of each car couples to the B head of the one before) spawns with the whole train's mass and drives: 70 km/h in about 12 s, every unit with scripts pulling and braking its own wheels. Two things made it unusable and are fixed:

* **Cab-inactive flags.** The control cables take the largest value on a line, which is right for the battery switch, the doors and the traction demand, but the flags `SLR/SLW_A_Kopf_Inaktiv` and `..._B_...` say "no occupied cab at this end of the train". With the other trams' inactive cabs writing 1 on the same line, the driven tram read "both ends inactive", throttled itself (`Drosselung` 30,000) and its passive cars braked with it. A line whose name contains `Inaktiv` is now 1 only while every unit that writes it writes 1: one occupied cab clears it for the whole train (`opts_sim::script::consist`, only units whose scripts write the variable vote). This is inferred from the scripts (Q-VS-1 stays open).
* **Torque and brake force of every unit.** The physics took only the head's `M_Wheel` and brake force, although it moved the mass of all trams. For a rail consist `script_drive_system` now sums the units with scripts of their own: the torque of a unit that hangs backwards counts with the opposite sign (the NF6D's B head writes −3,700 N·m while the tram drives forward), the brake forces add. One NF6D now has the pull of its A and B head (about twice what the head alone gave).

Open: points and route choice are those of a single tram; the passive cars' spring brakes and sanding follow their scripts (the holding brake below 5 km/h releases with the head's).
