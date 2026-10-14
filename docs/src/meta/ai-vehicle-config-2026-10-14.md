# AI Vehicle Configuration Investigation (2026-10-14)

| Metadata | Value |
| :--- | :--- |
| **Branch** | `research/ai-vehicle-config` (diagnostic; closed after this report, not merged in the usual sense) |
| **Base** | `master` at `2038a3e` |
| **Question** | Does the AI traffic simulation respect the per-vehicle configuration declared in `.bus` and `.ovh` files? |
| **Fix proposal** | [design](../architecture/ai-vehicle-config.md), [M-0095](../roadmap/phase-4/M-0095.md) |

Evidence tags: **[C]** counted over the installed corpus, **[R]** observed in a run of the game, **[K]** established by reading code (file and line). Nothing was fixed and no source file was changed.

---

## Executive summary

**No.** AI vehicles are *kinematic* in OPTS: the lane-following model (IDM) moves a point along a lane and the spawner places a mesh on it. The spawner reads exactly three things from a vehicle file (`[model]`, `[ai_deltaheight]`, whether `[passengercabin]` exists) and measures the body length from the meshes. Everything else that decides how a vehicle moves is **never read**: `[mass]`, `[type]`, `[newachse]`, the AI constants of the constfile (`AI_v_max`, `AI_M_gas_max`, `AI_F_brems_max`, the motor curve), `[ai_brakeperformance]`, `[ai_veh_type]`, `[couple_back]`. The IDM parameters are one of two hard-coded sets (`IdmParams::default()` for cars, `IdmParams::heavy()` when the body is longer than 7.5 m), with the desired speed jittered by ±10 % around 50 km/h. In a headless run on Waldhofen, 110 spawned vehicles of 14 types all had desired speeds between 45.0 and 55.0 km/h (median 49.9) although their files declare top speeds of 160 km/h and braking forces that differ by a factor of 2.5 per unit mass. A second, larger effect hides behind it: the spawner uses **one group** of the map's `ailists.cfg` (`NormalCars`), so of 781 distinct group/vehicle pairs in the 45 installed maps only 261 vehicles can ever appear, and no articulated bus, tram or lorry-with-trailer ever spawns. The fix is a *profile layer* between vehicle file and IDM (design and milestone attached); the scripted, dynamic mode that the AI plan already describes is a separate and larger step.

Of the 17 parameters traced, 14 have no effect on how an AI vehicle moves (7 of them are not even parsed, among them OMSI's own per-driver `[rowdy_factor]` in 539 files); two causes dominate (no parameter flow, one group only) and seven secondary issues follow.

---

## Symptom enumeration (Phase A)

**Limits of this phase.** Watching AI traffic in the running game for ten minutes was skipped on the project lead's instruction (no access to the machine's desktop; live verification postponed). What was done instead: a **headless run** (`--headless`, no window) on Waldhofen with debug logging, and code reading. Statements about what *looks* wrong on screen are therefore not part of this report; the claims below are about parameters and are testable by the commands in "Provenance".

| # | Claim | Evidence | Specific to a type? |
| :--- | :--- | :--- | :--- |
| S1 | All cars accelerate alike: one `max_accel` of 1.4 m/s² for every vehicle of at most 7.5 m body length, 0.8 m/s² above | [K] `idm.rs:49-76`, `traffic.rs:636-643` | Two classes only, split by body length |
| S2 | All vehicles have the same free-road speed: 50 km/h ±10 %, capped by the lane's speed limit; heavy vehicles included | [K] `traffic.rs:656-659`; [R] 110 spawns, 45.0-55.0 km/h, per-type ranges overlap completely (table below) | Universal |
| S3 | Following distance is the same for every vehicle: `T` 1.5 s, `s0` 2 m (the plan promised 1.7 s for lorries) | [K] `idm.rs:49-76` (`heavy()` only changes accel, decel, length, class) | Universal |
| S4 | Braking is the same: comfortable 2.0 m/s² (1.5 heavy), hardest 9.0 m/s²; the files declare 6.7-22.5 N/kg of brake force per mass on Waldhofen alone | [K] `idm.rs`; [C]/[R] table below | Two classes |
| S5 | Cornering speed is the same as straight speed: **no curvature-based speed limit exists**; the only cap is the lane's `speedlimit` rule | [K] `agent.rs:615` (`cap = lane.speed_limit`), no use of `radius` in the speed law | Universal |
| S6 | Buses, lorries and cars stay on their lanes: `bus`/`trucks`/`no_cars` rules are honoured through a class guess (bus iff the file has `[passengercabin]`, lorry iff longer than 7.5 m without one) | [K] `traffic.rs:127-133`, `524-534` | Heuristic; `[ai_veh_type]` unused |
| S7 | Trams or other `[type] 2` vehicles are not kept off road lanes, only by luck of the list: nothing tests `[type]`. None is in a chosen group today, except two motor scooters that declare `[type] 2` | [K] no `vehicle_type` use in `traffic.rs`; [C] `HH4_motorroller` and `_f` on HamburgLi20 | Latent |
| S8 | **No AI vehicle has a trailer or bellows.** 0 of the 261 vehicles in the chosen groups have `[couple_back]`; the 51 entries that do (`Trucks` 8, `Gelenkbusse` 4, trains) sit in groups the spawner never picks; and the spawner has no code to build a chain anyway | [C], [K] `traffic.rs` has no coupling code | Articulated vehicles |
| S9 | Vehicles of the same file differ only by the ±10 % speed jitter and by chance (passengers, indicator phase) | [K] `traffic.rs:656`, `681-697` | Universal |

### Runtime observation: declared against used (Waldhofen, `NormalCars`)

Headless run of about 2.5 minutes with debug logging (`AI vehicle spawned`), 110 spawns. "Desired speed used" is the logged spawn speed divided by the 0.8 the spawner applies. Brake N/kg is `AI_F_brems_max / ([mass] × 1000)`.

| Vehicle | `[mass]` t | `AI_v_max` km/h | `AI_M_gas_max` | `AI_F_brems_max` N | brake N/kg | Spawned | Desired speed used (km/h) |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | :--- |
| ai_hh109kipkw1 | 1.3 | 160 | 1000 | 18000 | 13.8 | 15 | 45-55 |
| ai_hh109kipkw2 | 1.3 | 160 | 1000 | 18000 | 13.8 | 6 | 48-55 |
| ai_hh109kipkw3 | 1.5 | 160 | 1000 | 18000 | 12.0 | 23 | 45-55 |
| ai_hh109kipkw4 | 1.3 | 160 | 1000 | 18000 | 13.8 | 12 | 45-51 |
| ai_hh109kipkw4_stufenheck | 1.3 | 160 | 1000 | 18000 | 13.8 | 14 | 46-54 |
| ai_hh109kipkw5_ziv | 1.6 | 160 | 1000 | 18000 | 11.2 | 15 | 47-55 |
| ai_hh109kipkw6 | 0.8 | 160 | 1000 | 18000 | 22.5 | 7 | 47-52 |
| ai_hh109kipkw7_ziv | 2.0 | 160 | 1000 | 18000 | 9.0 | 3 | 46-49 |
| ai_hh109kipkw7_paket | 2.0 | 160 | 1000 | 18000 | 9.0 | 1 | 54 |
| ai_vw_golf_2 | 1.0 | 160 | 1000 | 10000 | 10.0 | 2 | 46-51 |
| AI_mb_w123_230e | 1.0 | 160 | 500 | 10000 | 10.0 | 5 | 50-55 |
| AI_mb_w123_230e_cab | 1.0 | 160 | 500 | 10000 | 10.0 | 2 | 46-51 |
| VW_T3_Van | 1.5 | 160 | 1000 | 10000 | 6.7 | 2 | 48 |
| BX | 1.0 | 160 | 1000 | 10000 | 10.0 | 3 | 46-53 |

(Four further list entries, `kipkw7_ambu`, `kipkw5_taxi`, `kipkw5_pol` and `ai_opel_manta_b` (weights 1 to 5), were not drawn in this run; all 110 spawns together: minimum 45.0, median 49.9, maximum 55.0 km/h.) The cars of Waldhofen declare almost the same AI constants, so the *visible* difference between them would be small even with a correct flow; the corpus table below shows where it becomes large.

### The same constants over the corpus

261 distinct vehicles in the group each of the 45 installed maps uses (`NormalCars`, else the first group) [C]:

| Declared | min | median | max |
| :--- | ---: | ---: | ---: |
| `[mass]` (t) | 0.4 | 1.0 | 15.0 |
| `AI_v_max` (km/h) | 15 | 160 | 240 |
| `AI_v_max_erreich` (km/h) | 30 | 190 | 260 |
| `AI_M_gas_max` | 400 | 1,000 | 30,000 |
| `AI_F_brems_max` (N) | 10,000 | 10,000 | 150,000 |

The extremes are real vehicles in `NormalCars` lists: a 15 t `AI_MAN_F90_Wechselbruecke` (Express 91.06), a 14 t, three-axle `Citybus_L_AI` and a 5 t `Luzern_Truck` (Lucerne Line 24), a 15 t `ai_atego` (München) and two 0.4 t motor scooters (`[type] 2`) with `AI_v_max` 15 (HamburgLi20). OPTS drives all of them with one of two parameter sets at about 50 km/h.

---

## Parameter trace (Phase B)

### B1/B2: what files declare, what the AI sim uses

Counts over **2,752** `.bus`/`.ovh` files of the installation [C]. "Parsed" = a field of `BusManifest` (`crates/opts-core/src/formats/vehicles/bus.rs`). "Reaches AI" = the value arrives anywhere in `opts-game/src/in_game/omsi_world/traffic.rs` or `opts-traffic`. "Used" = it changes the motion or class of an AI vehicle.

| Parameter | Files | Parsed | Reaches AI | Used by AI sim | Notes |
| :--- | ---: | :---: | :---: | :---: | :--- |
| `[mass]` | 2,745 | yes (`mass_tonnes`) | no | **no** | The player sim uses it (`opts-sim` `from_manifest`) |
| `[type]` | 884 | yes (`vehicle_type`) | no | **no** | 877 are `2` (rail), 7 are `3`; `2` also appears on two road scooters |
| `[newachse]` | 2,752 | yes (`axles`) | no | **no** | Wheelbase is invented as 0.6 × body length, wheel radius as 0.31/0.5 m |
| `[boundingbox]` | 1,947 | yes | no | **no** | Body length is measured from the meshes instead |
| `[schwerpunkt]` | 1,908 | **no** | no | no | Not read anywhere, player sim included |
| `[rollwiderstand]` | 2,731 | **no** | no | no | Not read anywhere |
| `[inv_min_turnradius]` | 2,479 | **no** | no | no | Not read anywhere |
| `[rot_pnt_long]` | 2,731 | **no** | no | no | Not read anywhere |
| `[couple_back]` / `[coupling_back]` | 539 / 1,191 | yes | no | **no** | No trailer chain for AI |
| `[constfile]` → `AI_*` constants | 2,451 | constfile parsed for scripts only | no | **no** | `AI_v_max`, `AI_v_max_erreich`, `AI_M_gas_max`, `AI_F_brems_max`, motor curve |
| engine torque / transmission | – | player scripts only | no | **no** | No engine model for AI; `main_AI.osc` is not run |
| `[ai_brakeperformance]` | 141 | **no** | no | no | Two numbers; semantics open (Q-AI02) |
| `[ai_veh_type]` | 80 | **no** | no | no | One number; the vehicle files' own legend: `0` passenger car, `1` cab, `2` bus, `3` truck (50 trucks, 23 cars, 5 cabs, 2 buses) |
| `[rowdy_factor]` | 539 | **no** | no | no | Two numbers; the comment in the files: "spread of rowdy-factor: will be calculated on creation for each vehicle randomized between first and second value" (most common: `0 0.5` in 150 files, `0 0` in 91). Per-driver style, the only per-vehicle randomness OMSI declares; OPTS has a ±10 % speed jitter instead |
| `[passengercabin]` | 1,536 | yes (cfg path) | yes (a yes/no) | class `Bus` | |
| `[ai_deltaheight]` | 1,485 | **no** (raw scan in the spawner) | yes | visual offset only | |
| `[model]` | – | yes | yes | meshes | |

The IDM inputs and where they come from [K]:

| Location | Inputs used | Source |
| :--- | :--- | :--- |
| `idm.rs` `acceleration` | `desired_speed`, `time_headway`, `min_gap`, `max_accel`, `comfort_decel`, `delta`, `max_decel` | `IdmParams::default()` or `heavy()`, chosen by `size.0 > 7.5` (`traffic.rs:636-641`) |
| `traffic.rs:642-643` | `length`, `class` | meshes' bounding extent; cabin flag and length |
| `traffic.rs:656-659` | `desired_speed` | default 50 km/h × (0.9..1.1), capped by the lane's limit |
| `agent.rs:615` | speed cap | `lane.speed_limit` from `[rule]` only |
| `agent.rs` spawn check, lane change, conflict zones | `length`, `min_gap`, `class` | as above |
| `traffic.rs:692-693` | wheel radius, wheelbase for the animation variables | 0.5 / 0.31 m by the 7.5 m rule; 0.6 × length |

### B4: hypotheses

| # | Hypothesis | Verdict | Evidence |
| :--- | :--- | :--- | :--- |
| 1 | Mass is ignored | **Confirmed** | `traffic.rs` never touches `mass_tonnes`; there is no dynamics to apply it to. [R] speeds independent of mass (table above). Side finding: the player sim's `from_manifest` replaces any `[mass]` ≤ 0.5 t with 11,500 kg (`opts-sim/src/vehicle/mod.rs:126-129`), which would wrongly turn the 0.4 t scooters into buses if the AI used it as is |
| 2 | `[type]` is ignored | **Confirmed** | no `vehicle_type` use in the spawner; the class comes from cabin/length. Lane bans work through that class; rail and air vehicles are not filtered |
| 3 | Axle configuration is ignored | **Confirmed** | wheelbase `0.6 × length`, wheel radius 0.31/0.5 m, steering from that (`traffic.rs:692-693`, `736-743`) although `BusAxle` carries positions and wheel diameters |
| 4 | Articulated AI vehicles have no trailer | **Confirmed**, with a twin cause | (a) only one `ailists.cfg` group is ever used (`traffic.rs:400-403`); (b) there is no coupling code for AI: `traffic.rs` never reads `[couple_back]` or builds a second unit |
| 5 | Engine and transmission are ignored | **Confirmed** | no engine, no script: the AI "driver" is the IDM; the plan's dynamic mode (steps 4-7 of M-0049) is not built (`ai-traffic.md` §6) |
| 6 | Dimensions are ignored | **Partly refuted** | the body *length* (measured from the meshes) is used for gaps and spawn room; `[boundingbox]` is not read; the minimum gap `s0` is a constant 2 m for every vehicle |

### B5: runtime confirmation

Hypotheses 1, 2 and 5 were confirmed in a run, not only by reading. Setup: `opts-game --headless --omsi-path <OMSI> --map-path …/Waldhofen --vehicle-path …/MAN_NL_NG/MAN_EN92_main.bus --autostart --traffic 30 --log-level debug`, throw-away config and data directories. The existing `tracing::debug!("AI vehicle spawned")` line of the spawner (agent, vehicle, speed) was the instrument, so **no temporary logging was added and nothing needed reverting**. Result: 110 spawns of 14 vehicle types, spawn speeds 10.0-12.2 m/s, i.e. a desired speed of 45.0-55.0 km/h for every type, the 0.8 t `kipkw6` and the 2.0 t `kipkw7` alike. The binary was built from `agent/screen-capture` (`da1ca76`); `git diff master agent/screen-capture` for `crates/opts-traffic` and `traffic.rs` is empty, so the traffic code is master's.

---

## Primary root cause (C1)

**The vehicle file never reaches the traffic simulation as parameters.** `load_variant` (`traffic.rs:336-379`) parses the manifest and then keeps only the model path, `[ai_deltaheight]` (re-read from the raw text because it is not a manifest field) and the cabin flag. `spawn_traffic` builds `IdmParams` from two constant sets by body length (`traffic.rs:636-643`). That one decision explains S1-S5 and S9: every car brakes, accelerates, follows and corners alike, whatever the file says. It is also why the question "does the AI respect the vehicle's mass" has no code to point at: the kinematic mode has no place where mass could enter.

**Its twin: one group only.** `load_traffic_variants` picks `NormalCars` (else the first group) and ignores the other groups of `ailists.cfg` and `unsched_vehgroups.txt` (`traffic.rs:400-426`). Of 781 distinct (group, vehicle) pairs over the 45 maps, 261 are reachable; `Trucks` (22 entries), `Commercials` (22), `Emergency` (12), `Ambulance` (12), `Taxi`, `Gelenkbusse`, the trains and all 51 `[couple_back]` entries never appear. This explains S8 and the impression that the traffic is cars only.

## Secondary issues (C2)

1. **No cornering limit** (`agent.rs:615`). Only a `speedlimit` rule caps the speed; a 90° turn is taken at straight-road speed. This is a missing model feature, not a missing file read, but `[inv_min_turnradius]`, `[schwerpunkt]` and the declared brake/gas constants are the natural inputs. Unlike S1-S4 it affects every vehicle equally.
2. **Class by guess** (`traffic.rs:127-133`): bus iff `[passengercabin]`, lorry iff over 7.5 m. `Citybus_L_AI` declares no `[passengercabin]` in its file and would be classed by its length; `[ai_veh_type]` (80 files; the files document it as car, cab, bus, truck) and `[type]` (884 files) are unread. Rail and air vehicles in a road list would spawn on roads.
3. **No trailer chain for AI.** Even with the right group, `spawn_traffic` creates one unit. The player path has `spawn_trailer_chain` and `TrailerKinematics`; the AI side has no equivalent.
4. **Invented wheel and axle geometry** for the animation variables (wheel radius, wheelbase, steering), although `BusAxle` has positions and wheel diameters. Visual only.
5. **Keywords nobody reads**: `[rollwiderstand]` (2,731 files), `[inv_min_turnradius]` (2,479), `[schwerpunkt]` (1,908), `[rot_pnt_long]` (2,731), `[ai_brakeperformance]` (141), `[ai_veh_type]` (80), `[rowdy_factor]` (539). Not an AI-only gap: the player simulation does not read the first four either.
6. **Mass filter in the player simulation** (`from_manifest`: `m > 0.5`) drops legitimate 0.4 t vehicles to the 11.5 t default. Latent for AI until a dynamic mode reuses `from_manifest`.
7. **Documentation**: `misc-types.md` names a parser module `opts_core::formats::ovh` that does not exist (`.ovh` is read by `BusManifest`, as `ai-traffic.md` assumes), and says `[type] 2` is rail while a road scooter in a `NormalCars` list declares it.

## Proposed direction (C3)

A **design is needed**, but a modest one. The AI plan already separates the *decision* (opts-traffic, no files) from the *vehicle* (loader). What is missing is the typed hand-over between them. The fix is a **profile layer**: a plain `AiProfile` parsed from the vehicle file and its constfile (mass, class, axle geometry, `AI_*` constants, AI keywords, coupling), a documented mapping from profile to `IdmParams`, a spawner that uses all groups, and a trailer chain for coupled vehicles. It is a medium change, not a rewrite of the traffic model. The full scripted dynamic mode (M-0049 steps 4-7) stays a separate, larger item; the profile layer is its prerequisite (it needs the same data) and a useful end state on its own (kinematic traffic that differs by vehicle). Details: [ai-vehicle-config.md](../architecture/ai-vehicle-config.md); milestone: [M-0095](../roadmap/phase-4/M-0095.md).

## Out of scope (C4)

No IDM "feel" rebalancing, no change to the lane graph, signals or right of way, no change to where or when vehicles spawn in the map, no change to the player vehicle simulation (the 0.5 t mass filter is noted as a follow-up for whoever reuses it). Adjacent concerns found: OMSI's own AI behaviour is still unmeasured (Q-AI01, Q-AI04), so the mapping from declared constants to IDM numbers needs calibration against the running original; this investigation cannot supply it.

## Follow-ups

1. Answer Q-AI02 and Q-AI12 in `ai-traffic.md`: the vehicle files themselves state the `[ai_veh_type]` legend (0 car, 1 cab, 2 bus, 3 truck) and describe `[rowdy_factor]` (per-vehicle random spread between two numbers); the AI plan still lists both as unknown.
2. Calibrate against OMSI (desired speed against `AI_v_max` and the lane limit, acceleration against `AI_M_gas_max`/mass, braking against `AI_F_brems_max`/mass): Q-AI01, needs OMSI running.
3. Read `[rollwiderstand]`, `[inv_min_turnradius]`, `[schwerpunkt]`, `[rot_pnt_long]` in `BusManifest` (also benefits the player simulation).
4. Fix the `0.5 t` mass filter in `from_manifest`.
5. Correct `misc-types.md` (parser module name, `[type]` 2 on a scooter).
6. The remaining part of the original task, watching AI traffic live for ten minutes on two maps, was not done and would add the on-screen side of S1-S9.

---

## Provenance

* **Read:** `docs/src/architecture/ai-traffic.md`, `vehicle-scripts-route.md`, `formats/text/vehicles/bus.md`, `formats/text/misc-types.md`, `design/articulation-physics` (the consist-mass finding, read from that branch because it is not on master: trailer mass is a median 0.90 of the head's over 480 links, relevant to a later dynamic mode), `crates/opts-traffic/src/*.rs` (idm, agent, lanechange, conflict, rightofway, graph, source, density), `crates/opts-game/src/in_game/omsi_world/traffic.rs` (all), `crates/opts-core/src/formats/vehicles/bus.rs`, `crates/opts-sim/src/vehicle/mod.rs`.
* **Corpus scans (read-only, in the session scratchpad, not committed):** a Python reader that lists every `ailists.cfg` group of the 45 maps in `<OMSI>/maps`, resolves each vehicle file and its `[constfile]`, and counts keywords over all 2,752 `.bus`/`.ovh` files of the installation. Executables and DLLs were not opened. No OMSI file was changed or copied into the repository.
* **Run:** the headless game run described under B5 (binary from `agent/screen-capture` `da1ca76`, traffic code identical to master); log lines filtered with `grep 'AI vehicle spawned'`.
* **Not done:** the ten-minute live observation of Phase A (skipped by instruction), screenshots and recordings.
* **Clean-room (ADR-0002):** no OMSI binaries were inspected; no GPL code was read; no OMSI assets were committed.
