# Train formation research (2026-10-15)

| Metadata | Value |
| :--- | :--- |
| **Task** | Train Formation Editor, phase A (research; no code) |
| **Branch** | `feat/train-formation-editor` (worktree `opts-wt-train-formation-editor`, from master `839651f`) |
| **Evidence tags** | **[K]** read in code (not run live in this session), **[C]** counted over the corpus, **[M]** measured for this report, ❓ unverified |
| **Clean room** | No executable inspected; `.zug`, `.ovh` and `.bus` are text files read from the installation, nothing copied into the repository except short excerpts of file names in section 2 |

## 0. Preconditions

| # | Check | Result |
| :--- | :--- | :--- |
| 1 | Branch `feat/train-formation-editor` in an isolated worktree | yes (`../opts-wt-train-formation-editor`) |
| 2 | `articulation-physics.md` exists | yes (merged 2026-10-14, extended by the implementation merge) |
| 3 | Spawn menu code in `opts-game/src` | yes: `in_game/vehicle_menu.rs` (Left Alt menu, *Select Vehicle* modal), `session_vehicles/`, `new_game_screen/` |
| 4 | Chain assembly in `opts-sim` | **partly: the chain assembly is in `opts-bevy` and `opts-game`, not `opts-sim`** (`opts-bevy/src/articulation.rs`, `opts-game/.../omsi_world/vehicle.rs` `spawn_trailer_chain`, `rail.rs` `load_spec`); `opts-sim` holds the follower state (`vehicle/trailer.rs`, `consist.rs`) and the rail model (`rail/`) |
| 5 | M-0096 to M-0103 present | yes (8 files) |
| 6 | At least 100 `.zug` files in `Trains/` | yes: **223** in `Trains/` (207 top level + 16 in `Trains/odreid/`); 245 in the installation (14 in `Vehicles/`, 8 in `Splines/`) |
| 7 | Local CI on master | passes on `ecafeb4` (verified 2026-10-15); master has since gained at least one more merge (`839651f`) that I did not re-run |

Count discrepancy: the catalogue and `misc-types.md` say 186 (+14 +8 = 208); this installation has 223 in `Trains/`. The extra 37 are add-on files (the documented 186 is from an earlier scan). All numbers below are from today's scan.

## A1. The `.zug` format

### 1.1 Structure

A `.zug` file is **not** a keyword file. It has no `[keyword]` lines, no header, no comments. It is a list of **line pairs**:

```text
<vehicle>          line 1: a vehicle file relative to the OMSI root, backslashes (`.ovh`; never `.bus`), or a name
<orientation>      line 2: 0 = the car runs forward in the train, 1 = the car is turned around
```

repeated once per car, front to back. Evidence: [C] over all 223 files in `Trains/`:

| Property | Value |
| :--- | :--- |
| Distinct keywords | **none** (positional pairs) |
| Encoding | 221 UTF-8/ASCII, 2 CP1252 (`Flirt.zug` has `MÁV` in a path); no BOM |
| Line endings | CRLF in all 223; the last line has no line break in 153 of 223 files (70 end with one) |
| Regular files (every pair is `<*.ovh>` + `0`/`1`) | **204** |
| Files with entries that are a *name*, not a file (`U-Bahnwagen`, `M-Train Cars`, `L-Train Mute`, no extension) | **18** (see 1.3) |
| Files that are not consists at all | **1**: `RUHR.zug` is an `.osc` vehicle script (168 lines of `{init}`, `(M.L.…)`) with the wrong extension |
| Orientation values | only `0` and `1` in the 204 regular files (880 and 550 of the 1,430 entries of the 222 consist files) |
| Comments or metadata | **none** |

### 1.2 Eleven samples (ten consists and one stray)

The first 20 lines of each (the files are short; the longest regular one has 44 lines). Only file names appear; the content is the list of vehicle files, nothing else.

### `Trains/a3.zug`  (62 bytes, 8 lines, UTF-8/ASCII, CRLF)
- entries (line pairs): 4; `.ovh` references: 0; flags `1`: 0
```text
U-Bahnwagen
0
U-Bahnwagen
0
U-Bahnwagen
0
U-Bahnwagen
0
```
### `Trains/TH_RS1_einzel.zug`  (29 bytes, 2 lines, UTF-8/ASCII, CRLF)
- entries (line pairs): 1; `.ovh` references: 1; flags `1`: 0
```text
Vehicles\TH_RS1\TH_RS1.ovh
0
```
### `Trains/Muenchen_BR423.zug`  (80 bytes, 4 lines, UTF-8/ASCII, CRLF)
- entries (line pairs): 2; `.ovh` references: 2; flags `1`: 0
```text
vehicles\Muenchen_ET423\ET-423_1.ovh
0
vehicles\Muenchen_ET423\ET-423_1.ovh
0
```
### `Trains/X10_481-8_high.zug`  (150 bytes, 8 lines, UTF-8/ASCII, CRLF)
- entries (line pairs): 4; `.ovh` references: 4; flags `1`: 2
```text
vehicles\BR481_X10\BR481_high.ovh
0
vehicles\BR481_X10\BR481_high.ovh
1
vehicles\BR481_X10\BR481_high.ovh
0
vehicles\BR481_X10\BR481_high.ovh
1
```
### `Trains/X10_112_IC-8.zug`  (303 bytes, 18 lines, UTF-8/ASCII, CRLF)
- entries (line pairs): 9; `.ovh` references: 9; flags `1`: 0
```text
vehicles\BR112_X10\br112.ovh
0
vehicles\Bm238\Bpmz291_or.ovh
0
vehicles\Bm238\Bpmz291_or.ovh
0
vehicles\Bm238\Bpmz291_or.ovh
0
vehicles\Bm238\Bpmz291_or.ovh
0
vehicles\Bm238\Bpmz291_or.ovh
0
vehicles\Bm238\WRmh132_or.ovh
0
vehicles\Bm238\Avmz111_or.ovh
0
vehicles\Bm238\Apmz121_or.ovh
0
```
### `Trains/VA_E1_c3_Wien_1.zug`  (441 bytes, 20 lines, UTF-8/ASCII, CRLF)
- entries (line pairs): 10; `.ovh` references: 10; flags `1`: 0
```text
vehicles\VA_E1_c3_Wien_1\E1_Drehgestell1.ovh
0
vehicles\VA_E1_c3_Wien_1\E1_1.ovh
0
vehicles\VA_E1_c3_Wien_1\E1_Gelenk.ovh
0
vehicles\VA_E1_c3_Wien_1\E1_2.ovh
0
vehicles\VA_E1_c3_Wien_1\E1_Drehgestell2.ovh
0
vehicles\VA_E1_c3_Wien_1\E1_Kupplung1.ovh
0
vehicles\VA_E1_c3_Wien_1\E1_Kupplung2.ovh
0
vehicles\VA_E1_c3_Wien_1\E1_Drehgestell3.ovh
0
vehicles\VA_E1_c3_Wien_1\c3.ovh
0
vehicles\VA_E1_c3_Wien_1\E1_Drehgestell4.ovh
0
```
### `Trains/TH_Gueter_Kohle.zug`  (821 bytes, 44 lines, UTF-8/ASCII, CRLF)
- entries (line pairs): 22; `.ovh` references: 21; flags `1`: 13
```text
vehicles\TH_BR232\BR232_random.ovh
0
vehicles\Gueterwagen\Eaos_load.ovh
1
vehicles\Gueterwagen\Eaos_load.ovh
1
vehicles\Gueterwagen\Eaos_load.ovh
0
vehicles\Gueterwagen\Eaos_load.ovh
0
vehicles\Gueterwagen\Eaos_load.ovh
1
vehicles\Gueterwagen\Eaos_load.ovh
1
vehicles\Gueterwagen\Eaos_load.ovh
0
vehicles\Gueterwagen\Eaos_load.ovh
1
vehicles\Gueterwagen\Eaos_load.ovh
1
... (24 more lines)
```
### `Trains/DBC_MTrain.zug`  (102 bytes, 12 lines, UTF-8/ASCII, CRLF)
- entries (line pairs): 6; `.ovh` references: 0; flags `1`: 1
```text
M-Train Cars
0
M-Train Cars
0
M-Train End
1
M-Train Empty
0
M-Train Empty
0
M-Train Empty
0
```
### `Trains/BR_628_Doppeltraktion.zug`  (56 bytes, 4 lines, UTF-8/ASCII, CRLF)
- entries (line pairs): 2; `.ovh` references: 2; flags `1`: 0
```text
vehicles\BR628\BR628.ovh
0
vehicles\BR628\BR628.ovh
0
```
### `Trains/Flirt.zug`  (142 bytes, 10 lines, CP1252, CRLF)
- entries (line pairs): 5; `.ovh` references: 4; flags `1`: 0
```text
vehicles\MÁV\Flirt\Flirt_a.ovh
0
vehicles\MÁV\Flirt\Flirt_b.ovh
0
vehicles\MÁV\Flirt\Flirt_c.ovh
0
vehicles\MÁV\Flirt\Flirt_d.ovh
0


```
### `Trains/RUHR.zug`  (3,862 bytes, CP1252)

Not a consist: an `.osc` vehicle script (comment header, `{init}`, `(M.L.…)` calls) saved with the extension `.zug`. Not quoted.

### 1.3 Keyword (field) reference

| Field | Parameters | Meaning | Frequency |
| :--- | :--- | :--- | :--- |
| line 1 of a pair | vehicle path (`vehicles\Folder\Name.ovh`) | one car; resolved relative to the OMSI root, case-insensitive | 1,351 `.ovh` of 1,430 entries (94.5 %); `.bus` **never** |
| line 1 of a pair | a name without extension | a **vehicle group name** resolved by the map (`M-Train Cars` is a group of `maps/DBC_Map/ailists.cfg`); the other names (`U-Bahnwagen`, ...) occur only in `.zug` files in this installation, so their definition lives in a map or add-on that is not installed here ❓ | 79 entries in 18 files |
| line 2 of a pair | `0` / `1` | orientation of that car in the consist: `1` = turned 180° (the corpus agrees with `misc-types.md` Q-MS05) | 880 / 550 |

Where the format is used: **`ailists.cfg`** names `.zug` files in 53 map lists (`trains\kohle_voll.zug` 19 times, `kesselwagen.zug` 18, `BR_628_Doppeltraktion.zug` 12), so a `.zug` is an AI-spawnable *vehicle* of its own, next to `.bus` and `.ovh` ([lists](../formats/map/lists.md)); the 14 in `Vehicles/` and 8 in `Splines/` sit next to the vehicles and splines of their add-ons (their use was not traced). Whether timetables (`.ttp`) can name a `.zug` was not checked ❓.

### 1.4 Do the entries reference `.bus` or `.ovh` files, and how?

**`.ovh`, 100 % of the file references** (and 0 `.bus`). Of 367 distinct file references, 347 resolve in this installation, 20 do not (add-on content that is not installed: `MÁV\Flirt`, `O3D_BR628`, `ICE1_BHD`, `RUHR_STcoaster`, `BR481_X10\BR481_high.ovh`, ...). Of the 347 resolved files:

| Property of the referenced `.ovh` | Count | Share |
| :--- | ---: | ---: |
| `[type]` 2 (rail) | 338 | 97.4 % |
| `[boogies]` (OPTS's rail test, `ai_profile.rs`) | 338 | 97.4 % |
| `[sound]` block | 306 | 88.2 % |
| non-zero `[mass]` | 347 | 100 % |
| **`[couple_back]`** | **19** | **5.5 %** |
| `[couple_front]` / `[coupling_front_character]` | 0 / 0 | 0 % |
| `[coupling_front]` and `[coupling_back]` (coupling *points*) | present in the samples checked (`Bpmz291_or.ovh`: front `0 13.2 1`, back `0 -13.2 1`); census over all not run ❓ |

The important consequence: **a `.zug` entry is normally one self-contained car with its own mass, bogies and two coupling points, and no `[couple_back]` link**. The `.zug` is the *only* thing that joins such cars; OMSI places car *i+1* so that its front coupling point meets car *i*'s back coupling point, and flips the car when the orientation flag is `1`. That is exactly the placement OPTS already does for `[couple_back]` chains (`behind += f.y - b.y`, section A4), so the two models are compatible.

### 1.5 Corpus census (A2)

| Measure | Value |
| :--- | :--- |
| `.zug` files in the installation | **245**: `Trains/` 223 (207 + 16 in `odreid/`), `Vehicles/` 14, `Splines/` 8 |
| By vendor/prefix (top level of `Trains/`) | `X10` 30, `RUHR` 19, `VA` 14, `DUS` 12, `TH` 11, `AL` 11, `Gladbeck` 10, `br481` 7, `ALU` 7, `WUP` 6, `PMI` 6, others fewer |
| Total entries (222 consist files) | 1,430 |
| Distinct vehicle files referenced | 367 (347 resolve) |
| Most used cars | `Gueterwagen\Eaos_load.ovh` 97, `Eaos_empty.ovh` 87, `Zas.ovh` 67, `DB_IC\Bm235_IC.ovh` 29, `TH_Eaos\Eaos_log.ovh` 28 |
| Vehicle folders referenced | 115 |
| Formation length | 1 car: 19, 2: 24, 3: 19, 4: 30, 5: 20, 6: 23, 7: 4, 8: 15, 9: 14, 10: 15, 11: 18, 12: 4, 13: 2, 14: 5, 15: 1, 16: 4, 19: 1, **21: 4 (maximum)** |
| Buckets | 1: 19, 2: 24, 3: 19, 4-6: 73, 7-10: 48, 11-21: 39 |
| Files with at least one turned car | 130 of 222 (58.6 %) |
| Comments or metadata beyond the vehicle list | none |

Two to ten cars are the normal case (74 % of the files have 2-10 cars); freight trains (a loco plus 10-21 wagons) are the long end. All are below OPTS's chain limit of 40 (A4).

## A3. The current spawn path

| Question | Answer (with the code that says so) |
| :--- | :--- |
| Where is the vehicle list filled? | The *Select Vehicle* modal of the Left-Alt menu (`in_game/vehicle_menu.rs`) and the new-game wizard (`new_game_screen/step_what.rs`) read `GameCatalogue.vehicles: Vec<VehicleItem>` (`new_game_screen/types.rs:35`); each item carries `class` (`road` / `rail` / `aircraft`), `bus_file`, `mass_tonnes`. The catalogue is built by `opts-catalogue/src/extract.rs`. |
| How is a rail vehicle classified? | For play: `[type]` 2 (`is_rail`, `rail.rs:238`); for AI: `[type]` 2 **and** `[boogies]` (`opts-core/.../ai_profile.rs`, M-0095). For the *list*: `classify_playable` (R1-R11): `.ovh` files are never playable (R1), trailer targets without a full cab are not (R3), a rail vehicle without `[sound]` is a passenger-car variant, not a head (R11). Result: the playable rail vehicles are **`.bus` heads** (about 52 per `rail-vehicles.md`), e.g. `Tram_NF6D/NF6D_A.bus`. |
| What happens when a player selects a rail vehicle? | `SpawnParams { bus_file, repaint, setvars, entry }` → `session_vehicles::ops` → `load_player_vehicle(job, ..., session_id, activate)` (`omsi_world/vehicle.rs:811`). It parses the `.bus`, spawns the model, then `if is_rail(&manifest)` inserts `RailPlayer::new(commands_mass(&manifest), bogie_half_span)` (`vehicle.rs:915`), then `spawn_trailer_chain` follows `[couple_back]` and spawns the rest of the units. |
| Blocked, partial or full? | **Partially supported.** Works for a *`.bus` head with a baked-in chain* placed within 12 m of a rail path (`SNAP_MAX_M`, `rail.rs:27`); further away the vehicle "stays a road vehicle" with an error log (`rail.rs:294`). **Not supported:** `.zug` files, `.ovh` heads, any list of cars, choosing the place on a track (the spawn point is a map *entry point*, `EntryIndex`, which is road-oriented). The AI side: `RailAiPool` spawns AI trains from `.bus`/`.ovh` chains (`rail.rs:592-726`) but `load_rail_variants` keeps only entries ending `.bus` or `.ovh`: **every `.zug` entry of the 53 AI lists is silently ignored**. The road AI spawner refuses rail by design (M-0095). |
| Signature and placement | `load_player_vehicle(job: &mut WorldLoadJob, stats, commands, meshes, materials, images, bindposes, session_id: VehicleId, activate: bool) -> Result<SpawnedVehicle, String>`; the position is `WorldLoadJob::spawn_point()`; the rail snap happens later, in `rail_player_step` (`rail.rs:275-292`) on the first fixed step: `RailNetwork::nearest(p, 12 m)`, direction from the heading difference. |

I did not run a rail spawn live in this session; the statements are from code ([K]). The screenshots of the NF6D in `rail-vehicles.md` §7 are the existing evidence that it works.

## A4. Existing chain assembly

| Question | Answer |
| :--- | :--- |
| How is a chain assembled? | At spawn, `spawn_trailer_chain` (`vehicle.rs:507`, for the player) and `load_spec` (`rail.rs:667`, for AI trains) walk **`[couple_back]` from file to file**: each file is parsed, its model loaded, its coupling points read (`[coupling_back]` of the leader, `[coupling_front]` of the follower, or the negated `[coupling_back]` when the link is `reversed`), and a unit is appended at `behind += f.y - b.y`. The walk stops on a missing file, a missing coupling point, or after a `reversed` link ("the ring of a double-ended consist closes here"). |
| Data model | Road: `ArticulatedSegment` components (`opts-bevy`), the kinematic state `TrailerKinematics` in `VehicleSimulation::consist` (`opts-sim/src/vehicle/trailer.rs`, since M-0053 D1), mass in `UnitLoad` (`consist.rs`). Rail: `VehicleComposition { head: Part, followers: Vec<Part> }` (`rail.rs:373`) with `behind`, `half_span`, `reversed` per part; AI: `TrainSpec { units: Vec<UnitSpec> }`. |
| Maximum depth | **40** in both walkers (`MAX_SEGMENTS = 40`, `vehicle.rs:517`; `for _ in 0..40`, `rail.rs:686`). The longest regular `.zug` has 21 cars; the NF6D alone is a chain of about 12 parts (cars, bogies, joints, coupler as separate `.bus` files). A 21-car freight train plus bogies-as-parts would still fit; a train made of NF6D-style part chains (about 12 parts per unit) would **not** fit 3 units in 40 (see A5). |
| Trailer mass | Fixed and merged on 2026-10-14 (M-0053 D2 + `consist-mass-fix`): a road consist's mass is the sum of its units (`consist_mass_kg`), traction counts the driven axles of all units. **Rail is unchanged**: `commands_mass` = the head's mass, at least 20 t, because "the other units usually state 0" (`vehicle.rs:789`); `RailDynamics::tram(mass)` is one body. So a rail consist of many cars is simulated with the head's mass only. |
| AI use | Road AI: the spawner does not read `[couple_back]` (P4 in `ai-vehicle-config.md`; D4 plans it); rail AI: `RailAiPool` builds trains from the same walk and moves them with `Train::advance` along the track. |
| Placement of units in rail | **Not by the articulation joint.** The rail consist is placed by arc length on the track: `unit_frame(train, net, ...)` and `unit_pose` (`rail.rs:435-480`) put each unit's two bogies on the track at `behind` and `behind + 2*span`, so curves and points are followed exactly with no yaw state. |

## A5. Rail vehicle corpus

Surveyed with a read-only script over `Vehicles/*/*.{bus,ovh}` (keywords in column order):

| Vehicle file | `[friendlyname]` | `[type]` | `[couple_back]` | `[couple_front]` | `[coupling_front_character]` | own traction / script | In a `.zug`? |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `Tram_NF6D/NF6D_A.bus` | Strassenbahn | 2 | `NF6D_Gelenk_ACV.bus` | - | - | cab, `[sound]`, 12 t | no (it is the head of its own baked-in chain) |
| `Tram_NF6D/NF6D_B_User.bus` | - | 2 | `NF6D_Gelenk_ACV.bus` | - | - | cab (the playable head), `[sound]` | no |
| `Tram_NF6D/NF6D_Drehgestell*.bus` | - | 2 | next part of the chain | - | - | bogie as a vehicle (`[boogies]`, mass 0) | no |
| `Tram_NF6D/NF6D_Kupplung_DT.bus` | - | 2 | `NF6D_A_DT2.bus` | - | - | the **coupler between two baked-in units** (the `_DT` variants are double traction) | no |
| `BR481_X10_FF/BR481_Drehgestell1A.ovh` | - | 2 | `.\BR481_Drehgestell2A...` | - | **yes** | `[sound]`, 31 t | not checked (the `X10_481-*` files use `BR481_X10\BR481*.ovh` cars) |
| `Bm238/Bpmz291_or.ovh` (IC coach) | - | 2 | - | - | - | `[sound]`, 40 t, coupling points ±13.2 m | yes, 25 times |
| `BR112_X10/br112.ovh` (loco) | - | 2 | - | - | - | `[sound]`, 83.8 t | yes (`X10_112_*`) |
| `Gueterwagen/Eaos_load.ovh` (wagon) | - | 2 | - | - | - | `[sound]`, 40 t | yes, 97 times |
| `VA_E1_c3/E1_1.ovh` | - | 2 | - | - | - | `[sound]`, 30 t | yes (Wien trams listed part by part: bogies, joint, couplers as entries) |

(The `[friendlyname]` is empty for most rail parts; the cab vehicles are the only named ones.)

Findings:

- **Two kinds of rail vehicle coexist.** *Part chains* (NF6D, E1, BR481, ET-422): one logical vehicle is 10-40 small `.bus`/`.ovh` files joined by `[couple_back]` (bogies, joints, couplers as separate vehicles). *Self-contained cars* (coaches, wagons, locos): one `.ovh` with mass, bogies, two coupling points and no `[couple_back]`. `.zug` files list the second kind (338 of the 347 resolved entries have `[boogies]`, only 19 have `[couple_back]`); the Wien E1 `.zug` lists the first kind part by part.
- **Which vehicles have traction?** Locos (`br112`, `BR146`, `BR232`) and motor cars. Traction is script-driven (`.osc`); OPTS has no per-car traction on rail: `RailDynamics::tram(head mass)` drives the whole consist (traction `1.1 m`, power `14 W/kg`, max 70 km/h, `dynamics.rs:57`). Coaches and wagons are passive.
- **A formation of "3 x NF6D" cannot be built from `NF6D_A.bus` three times**: each one drags its own chain of about 12 parts, so 3 would need 36 of the 40 allowed depth and, more importantly, the chain end (`NF6D_B_User`) is a *cab*, not a coupler. OMSI's own answer is the **`_DT` / `_DT2` files**, a pre-baked double traction with a `Kupplung` part between two chains. A formation editor must therefore treat an NF6D as a **unit with a front end and a rear end** whose inner chain is fixed (the first car part to the last), and join units by their end coupling points, like `.zug` entries do. That works if a unit can be described by its head file plus where the chain ends; the chain end is discoverable by the existing walker. ❓ Whether the rear cab of the chain (`B_User`) exposes a `[coupling_back]` usable for the next unit is not verified.

## A6. Control surface

- **Who is controlled?** The player controls **one vehicle entity**: the head (`PlayerVehicle` + `VehicleController` + `RailPlayer`). All other units are render entities positioned by the track (rail) or by the kinematic follower (road); they have no controller and no input.
- **Scripts of trailing cars:** `attach_scripts` runs for the root only (`vehicle.rs:924`); trailing segments are meshes plus, for road buses, `articulation_*` variables fed by the follower. The strategy's P2 (M-0098) plans one script host per consist with per-part engines, `[scriptshare]` stores and **control cables** (`[control_cable_front/back]` exist in the cars' files, e.g. `Bpmz291_or.ovh`); that is the mechanism by which a motor car at the rear would be driven from the head. Until M-0098, trailing cars run nothing.
- **Cab switching:** not supported for segments. F8 / *Switch* in the Alt menu changes the active **session vehicle** (`session_vehicles::ops::switch_to`), i.e. another independent vehicle, not another cab of the same consist. A reversed run of a double-ended unit (the Wien and Hamburg tram cabs) is done in OMSI by driving from the other cab; OPTS has none.
- **Direction:** `RailInput.direction` is fixed `1`; the player has no reverser; the train point is the *front bogie of the head* and the head's heading defines forward (`rail_player_step`), so a formation always drives forward from its first car.

## A7. Physics assessment (input to the design)

What a train needs and what the code has:

| Need | State |
| :--- | :--- |
| Units follow curves and points | **Done exactly** for rail: arc-length placement on the track (`unit_frame`, `behind`), no yaw state, no joint model. The articulation joint of buses is **not used** for rail. |
| One speed for the consist | Done: `RailDynamics` single body, `speed` shared; units are positions along the same track. |
| Mass of the consist | Not done: head mass (at least 20 t); other units contribute nothing. A 21-wagon freight train (about 800 t) would accelerate like 20-83 t. |
| Per-car brakes, per-car traction | Not modelled (no per-unit state). |
| Push and pull, coupler slack, buff | Not modelled; a single body cannot do it. |
| Collision between consists | Footprints exist (`Part.body`, `[boundingbox]`), no consist-vs-consist model. |

Whether the existing single-body model is *insufficient* has to be shown on a test case (the task's rule against choosing rigid-body coupling without a failing case): the design phase will propose two, the **mass error** (a 20-car freight formation at 1/40 of its real mass) and **brake distance** (all cars braking vs head only). Both are visible without any coupler physics, so they argue for *summing mass and brake force per unit* (the D2 approach, extended to rail), not for rigid bodies.

## A8. Gaps and risks found

1. **No `.zug` parser exists** anywhere in the code (`AiLists` keeps the name, nothing opens the file).
2. **`.zug` entries in AI lists are ignored** (53 lists), so no AI train from a `.zug` ever spawns today.
3. **Depth limit 40** counts parts, not units; the NF6D-style part chains eat it fast.
4. **Rail consist mass** is the head's only.
5. **Spawn place**: entry points are road-oriented; a formation needs a track and a direction.
6. **Name entries** (18 files) need a map's AI group, which does not exist outside a session.
7. **`RUHR.zug`** is not a consist: a parser must reject it by structure, not by extension.
8. **Orientation flag semantics for part chains** (a flipped *chain*, not a flipped car) is ❓.
9. The brief's path `crates/opts-sim/src/articulation.rs` does not exist; the articulation system is `crates/opts-bevy/src/articulation.rs`.

## A9. Questions for Checkpoint 1

1. **Unit of composition.** A formation segment is "one `.zug`-style entry": a vehicle file with an orientation. Do you accept that, for part-chain vehicles such as the NF6D, a segment is the *whole chain* of that head file (inner chain fixed), joined to the next by coupling points?
2. **Scope of the first deliverable.** `.zug` read-only presets plus a custom editor for **self-contained cars** (coaches, wagons, locos, the 338 `.ovh` kind) first, with part-chain units (NF6D) as a stretch goal depending on question A5's open item?
3. **Spawn place.** Spawn on the nearest track to an entry point (as today), or add a track picker (a later milestone)?
