# AI Road Traffic — Research and Integration Plan

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Plan; kinematic traffic (cars on lanes, IDM, rules for speed and density) runs in the game; signals, priority, script-driven vehicles and calibration are open |
| **Date** | 2026-10-11 |
| **Milestone** | [M-0049](../roadmap/phase-4/M-0049.md) |
| **Crate** | [`opts-traffic`](../../crates/opts-traffic.md) (pure Rust, no Bevy) |
| **Specs it builds on** | [map lists](../formats/map/lists.md), [`.sco` paths and traffic lights](../formats/text/sco.md), [`.sli` paths](../formats/text/sli.md), [map tiles (`[rule]`)](../formats/map/tiles.md), [`.osc` scripts](../formats/text/vehicles/osc.md), [predefined variables](../formats/text/vehicles/predefined-variables.md), [script VM](script-vm.md) |

Evidence tags as elsewhere: **[C]** counted over the installed corpus, **[M]** measured with the tools of this repository, 📚 community wiki (facts only), 🌐 external literature, ❓ unverified.

---

## 1. Goal

OMSI maps are alive with background traffic: cars, lorries, cyclists, scheduled buses and trams that follow the road network, stop at signals and keep their distance. OPTS has the *data* for this (lists, paths, rules, signals, vehicle scripts) but no *behaviour*. This document records what the repository already knows, what the corpus adds, which algorithms fit, and how the pieces are cut so that AI traffic can be added step by step without touching the player's vehicle code.

**Out of scope for the first milestone:** scheduled public-transport AI (timetables, bus stops, depot rosters), pedestrians, rail signalling, collisions with the player. Section 8 records the hooks they need.

---

## 2. What OMSI gives us (inputs)

| Need | Source | Spec | Parsed in `opts-core`? |
| :--- | :--- | :--- | :--- |
| Which vehicles may appear, with relative frequency | `ailists.cfg` `[aigroup_2]` | [lists](../formats/map/lists.md) | ✅ `AiLists` (this milestone) |
| Which groups take part, default density class | `unsched_vehgroups.txt` | [lists](../formats/map/lists.md) | ✅ `parse_unsched_vehgroups` |
| Density over the day, per group and day type | `unsched_trafficdens.txt`, `global.cfg` `[trafficdensity_road]` | [lists](../formats/map/lists.md), [global.cfg](../formats/map/global-cfg.md) | ✅ `parse_unsched_trafficdens`; `global.cfg` curve ✅ (`MapManifest`) |
| Seasonal/dated replacements | `Chrono/*/ailists_#upd.cfg`, `unsched_trafficdens.txt` | [chrono](../formats/map/chrono.md) | ✅ same parsers, caller picks the file |
| Where vehicles may drive | `[path]` of `.sli` (splines) and `.sco` (junctions, crossings) | [sli](../formats/text/sli.md#path), [sco](../formats/text/sco.md) | ✅ `SplinePath`, `ScoPath` |
| Placement of those splines/objects | `tile_<x>_<y>.map` | [tiles](../formats/map/tiles.md) | ✅ `MapTile` |
| Speed limit, density, priority, lane/type bans per path | `[rule]` in tiles | [tiles](../formats/map/tiles.md) | ✅ with owner and path index (`MapTrafficRule::owner`, `kind()`), applied to lanes by `segments_from_tile` |
| Traffic-light programs and which paths they stop | `[traffic_lights_group]`, `[traffic_light]`, `[phase]`, `[use_traffic_light]`, `[approachdist]` | [sco](../formats/text/sco.md#-traffic-lights) | ✅ programs, light index per path, approach distance |
| How a vehicle turns throttle/brake into motion | `{frame}` of the AI script, `*_constfile.txt` `AI_*` constants | [osc](../formats/text/vehicles/osc.md), [const](../formats/text/vehicles/const.md) | ✅ script VM (`opts-sim`) |
| AI-only vehicle keywords | `[ai_deltaheight]`, `[ai_brakeperformance]`, `[ai_veh_type]`, `[sound_ai]` | [bus](../formats/text/vehicles/bus.md), [misc types](../formats/text/misc-types.md#ovh) | ⚠️ none are fields of `BusManifest`; the game reads `[ai_deltaheight]` straight from the file text (`omsi_world/traffic.rs`), the others are not read |

### 2.1 What the lists look like in the installed corpus [M]

`crates/opts-core/tests/ai_lists_corpus.rs` (needs `OMSI_PATH`):

* 101 `ailists*.cfg` files under `maps/`, 953 `[aigroup_2]` groups, 2,187 vehicle entries — **760 (35 %) have no weight**, so the default weight matters; OPTS counts a missing weight as `1` ❓. 5 lists have no `NormalCars` group.
* 32 `unsched_vehgroups.txt` (147 groups), 250 `unsched_trafficdens.txt` (1,561 groups); 28 density groups name an AI group that is absent from the sibling `ailists.cfg` (chrono folders use their own lists, so this is expected for some).
* **A density point can lie beyond 24 h** (`TH_Wald`: hour `25.000`, density `0.25`). Hours are therefore not clamped to `0..=24`; [`DensityCurve`](../../crates/opts-traffic.md) wraps modulo 24 ❓.

### 2.2 The vehicle contract: the AI "driver" only presses pedals [C][M]

The decisive finding for the architecture. OMSI's engine decides *where and how fast* an AI vehicle drives; the **vehicle's own script** turns that into forces. The shared AI car script (`Scripts/AI_Cars/main_AI.osc`, family of 20+ files; read-only inspection) contains, in `{frame}`:

```
(L.L.Throttle) (C.L.AI_M_gas_max) * 1 (L.L.Velocity) (C.L.AI_v_max_erreich) / - * (S.L.M_Wheel)
(L.L.Brake)    (C.L.AI_F_brems_max) * (S.L.Brakeforce)
(L.L.AI_Blinker_L) …  (S.L.lights_blinker_l)
```

So the engine-side AI writes **`Throttle`, `Brake`, `AI_Blinker_L/R`, `AI_Light`, `AI_target_index`, `AI_Engine`**; the script computes `M_Wheel` (torque, falling linearly to zero at `AI_v_max_erreich` km/h) and `Brakeforce` from constants. Emergency vehicles additionally read engine-provided `TrafficPriority`/`TrafficPriorityWarningNeeded` (siren logic). Constants used by AI vehicles, counted over all constfiles [C]: `AI_F_brems_max` (553), `AI_lights_blinkgeberintervall` (524), `AI_M_gas_max` (489), `AI_v_max_erreich` (474), `AI_v_max` and `AI_Motorkennlinie` (282 each), `AI_wandler_*` (99 each).

Consequences:

1. **A real AI vehicle is the same simulation as the player's vehicle**, driven by a controller instead of a keyboard. OPTS already has everything on that side (`VehicleController`, `VehicleScripts`, `VehicleSimulation`); the AI only has to produce `Throttle`/`Brake`/indicators. `opts-traffic` models exactly that output: [`DriverCommand`](../../crates/opts-traffic.md).
2. **A cheap kinematic mode is possible** for far-away vehicles: integrate position directly (the IDM), skip scripts and physics, place the model at the pose. Both modes share the same agent state, so a vehicle can change mode without a visible jump.
3. `{frame_ai}` blocks exist (`osc` spec), but the shared AI scripts above use plain `{frame}` and a `(L.L.AI)` flag; both forms must be supported (the script VM already runs `frame_ai` when asked).
4. Per-vehicle AI keywords: `[ai_deltaheight]` (vertical offset; 1,475 uses, **-0.10 m in 1,202, -0.12 in 109** [C]) lowers the model onto the road when driven by AI; `[ai_brakeperformance]` (141 uses, two numbers, values `1.3`/`1.0` then `0.0` [C]) scales braking ❓; `[ai_veh_type]` (76 uses, one number `0`–`3`: 0 ×23, 1 ×5, 2 ×2, 3 ×46 [C]) is a vehicle class ❓ (likely car/bus/lorry/… and used by path rules such as `trucks`, `bus`, `no_cars`).

---

## 3. The road network

### 3.1 Connectivity is geometric — confirmed at scale [M]

The specs state that OMSI never names a path's successor; lanes join where an end meets a start in position and heading ([sco](../formats/text/sco.md), [tiles](../formats/map/tiles.md): crossings "connect only geometrically"). `opts-traffic` builds the lane graph that way, and the `lane_graph_audit` example measures it on the real maps (all splines' `.sli` paths with their lateral offsets, all objects' `.sco` paths placed with the clockwise heading convention; road lanes only, tolerance 0.75 m / 15°):

| Map | Road lanes | Length | Lanes with a successor | Components (largest) |
| :--- | ---: | ---: | ---: | ---: |
| Grundorf | 260 | 8 km | **99.6 %** | 1 (260) |
| Hamburg109 | 8,325 | 188 km | 97.0 % | 45 (8,032) |
| HafenCityHamburg | 14,919 | 338 km | 96.8 % | 144 (12,833) |
| Bad_Huegelsdorf_2020 | 8,621 | 247 km | 96.4 % | 118 (3,058) |
| Rheinhausen | 11,514 | 329 km | 95.2 % | 211 (10,336) |
| Ruhrau V2 | 12,057 | 344 km | 91.1 % | 710 (1,563) |
| Berlin-Spandau | 7,258 | 282 km | 85.5 % | 595 (283) |
| Ruhrgebiet | 47,425 | 1,772 km | 81.1 % | 6,718 (5,783) |
| Düsseldorf | 12,253 | 518 km | 71.1 % | 3,159 (100) |

Over **42 maps** with lanes (419,554 road lanes, 13,440 km): median **92 %** of lanes have a successor, about two thirds of the maps are at 90 % or above, range 71–99.6 %. Build time for Ruhrgebiet (47 k lanes): a few milliseconds. Widening the position tolerance from 0.25 m to 0.75 m adds 0.1–0.4 % of the joins, to 2 m only another 0.1 %; the remainder are genuine dead ends (map edges, depots, cul-de-sacs) or broken joins ❓. **Conclusion: the geometric rule is right, the graph is cheap to build, and the defaults (0.75 m, 15°) are sound.** Maps with many components (Düsseldorf, Wuppertal2, Tram) need a look at what ends there before spawning on them ❓ (§9).

Data hazards found while doing this:

* Some splines have **non-positive lengths** (e.g. `-9.11`, `-0.14` in `Ahlheim 5` and `Waldhofen`). They are skipped; what OMSI does with them is unknown (reversed direction?) ❓.
* `Neuendorf` yields **no road lanes** although it has 254 tiles — a tile or profile variant that the parsers or this builder do not cover ❓.
* A lane's `direction` `2` (both ways) becomes two directed lanes on the same centre line; opposing lanes are therefore only distinguished by their lateral offsets (`offsetX`), as OMSI authors draw them.

### 3.2 Geometry conventions (verified in earlier work, reused here)

Compass heading (clockwise from +Y), positive radius turns clockwise, `-999.99` radius means straight, object-local paths rotate clockwise by the object heading, tile origin `300 m · (x, y)`. The conventions come from the `[spline]` mutation tests and the `crossing_audit` example; `opts_traffic::geometry` implements and tests them (arc, reverse, lateral offset, object placement).

### 3.3 Rules, priority, signals

* `[rule]` over the whole corpus [M] (524,909 rules): `trafficdensity` 342,849, `speedlimit` 83,072, `priority` 74,143, `trucks` 13,088, `overtaking_prohib` 6,329, `no_cars` 3,297, `bus` 2,131. Speed limits are km/h; priorities are `192` or `64`; the vehicle-type flags carry no value. Every rule follows a spline or object and addresses one of its paths by index ([tiles](../formats/map/tiles.md)). On the real maps the lanes pick them up: Ruhrau V2 has a density rule on 11,346 of 12,054 road lanes and a speed limit on 3,380; Hamburg109 on 4,075 of 8,325 and 720. With **no priority rule, AI vehicles follow right-before-left** (📚).
* Signals: a junction object defines programs (`[traffic_light]` + `[phase]` states) and each `[path]` names the light that governs it (`[use_traffic_light]`) and an approach distance (`[approachdist]`). Phase *state codes* (`0` red … `6` green … `9` yellow, sub-values for blinking variants) are only partly decoded ([sco](../formats/text/sco.md), Q-SCO-005). The AI rule must be: stop at the path's end when its light is red (or yellow and the vehicle can stop), i.e. treat the path end as a standing obstacle with speed 0 — which is exactly the IDM's "leader at gap *s* with speed 0".
* Stop lines other than traffic lights (`[traffic_light_stop]`, `[traffic_light_jump]`) have unexplained parameters (Q-SCO-005).

---

## 4. Algorithms (research summary)

OMSI's own AI algorithm is **not documented anywhere I could find**: web search returned only store pages and tool descriptions (the official Aerosoft "Power Toolkit" lists an *AI List Editor*; no description of path following, priority or spacing). The wiki facts already absorbed into the specs (lists, rules, right-before-left, depot rules) are all there is. OPTS therefore needs its own model, calibrated against observation of OMSI (§9). The literature offers standard, well-tested building blocks:

* **Car following — Intelligent Driver Model** (Treiber, Hennecke, Helbing 2000). `a = A·[1 − (v/v0)^δ − (s*/s)²]`, `s* = s0 + v·T + v·Δv / (2·√(A·B))`; collision-free, one equation, stops cleanly behind a standing obstacle, parameters have physical meanings. Reference values from the [traffic-simulation.de page](https://traffic-simulation.de/info/info_IDM.html): car `v0` 120 km/h (adapt to town), `T` 1.5 s, `s0` 2 m, `A` 0.3 m/s² (as a low-acceleration example; realistic 0.8–2.5), `B` 3 m/s² (realistic ≈ 2), lorry `T` 1.7 s. The page integrates ballistically with steps below 0.5 s and stops a vehicle exactly when speed would turn negative — the same update `opts-traffic` uses.
* **Lane changes — MOBIL, implemented as `opts_traffic::lanechange`** (Kesting, Treiber, Helbing 2007, [Transportation Research Record 1999](https://journals.sagepub.com/doi/abs/10.3141/1999-10)): a lane change is allowed if the new follower does not have to brake harder than a safe limit and the net acceleration gain (own plus politeness × others') exceeds a threshold. It composes with the IDM. Used for overtaking slow vehicles (`overtaking_prohib` forbids moves to the left) and for keeping right (*Rechtsfahrgebot*). OPTS departs from the paper in the safety part, because OMSI's oldest AI complaint is hard braking at lane changes: the **target lane has priority** (German rule, the changer must not endanger others): the new follower may need at most 1.5 m/s² of braking with the changer in front of it (MOBIL's default is 4), the changer at most 1.0 m/s² behind its new leader, nobody may be beside the target spot or changing into the same lane there, and the change is announced with the indicator for 1.2 s and re-checked before the vehicle moves. A faster follower in the target lane simply means "no gap yet": the changer waits instead of cutting in. The car is moved to the new lane at once (followers react immediately and gently) and glides sideways over 3 s with smoothed yaw, which keeps the picture continuous.
* **Junctions** — OMSI's junction objects already contain every possible movement as a path; conflicts between paths are the junction's business. A first-order solution that needs no data OMSI does not provide: conflict zones found geometrically (implemented, `opts_traffic::conflict`: crossing lanes within 1.6 m at 20°–160°, and lanes feeding the same successor) with turn-taking by estimated arrival time; real right of way (right-before-left, `priority` rules) and signals refine who goes first.
* **Routing** — unscheduled traffic does not have destinations; vehicles pick a branch at each fork (OPTS: seeded random choice, weighted by `trafficdensity` rules once those are associated with lanes). Scheduled vehicles need shortest-route search over the same graph (`LaneGraph::shortest_route`, Dijkstra by length).

---

## 5. Architecture

```
 opts-core (parse)            opts-traffic (decide)              opts-bevy / opts-game (show)
 ───────────────────          ──────────────────────────         ─────────────────────────────
 ailists / unsched_*  ──►     SpawnTable (weights, density) ──►  spawn system: picks a vehicle,
 tile .map + .sli/.sco ─►     segments_from_tile                  loads model + scripts (existing
                              LaneGraph (geometric joins)         vehicle loader, AI flavour)
                              TrafficWorld.step(dt)         ──►  kinematic mode: place model at pose
                                │  IDM, leaders, lane ends   ──►  dynamic mode: write Throttle/Brake/
                                │  signals, priority (later)        AI_Blinker_* into VehicleScripts,
                                ▼                                    VehicleController integrates
                              Agent { lane, s, v, command }
```

Principles:

1. **`opts-traffic` knows no Bevy and does no I/O** — like `opts-sim`. The host feeds it parsed data and the clock; it can run headless, be replayed (one seeded SplitMix64 generator, no hash-order dependence: agents are processed in lane/`s`/id order) and be unit-tested without a window.
2. **Parsing stays in `opts-core`**, simulation in `opts-traffic`, ECS glue in `opts-bevy` (an `AiTrafficPlugin` that owns the world as a resource, mirrors agents to entities and implements the two modes). The game decides *when* to run it (map loaded, player position, density settings).
3. **Two fidelity levels, one state.** *Kinematic*: the crate moves the agent and the host places a mesh. *Dynamic*: the host spawns the vehicle simulation and scripts and feeds `DriverCommand`; the agent's `s`/`speed` are then read back from the vehicle (the lane position follows from projecting the vehicle onto its lane). Dynamic is used within a radius of the player (≈ 150 m ❓), kinematic beyond, with hysteresis. Most background vehicles never need scripts at all.
4. **Streaming.** The lane graph is built per loaded tile set; because tiles stream in and out (M-0024, resident-tile streaming as implemented by `stream_world_tiles`), the graph is rebuilt (or extended) when the resident tile set changes. A whole map builds in milliseconds [M]; agents on a lane that disappears are despawned (`TrafficEvent`), agents must not outlive their tile's data. Spawn only on resident lanes, outside the camera frustum and beyond a minimum distance from the player; despawn beyond the despawn radius or at dead ends.
5. **Density.** Target vehicle count near the player = `lane_km_resident × density(hour, day) × base_per_km` where `density` comes from `unsched_trafficdens.txt` (× `global.cfg`), and per-lane `[rule] trafficdensity` scales it. OMSI's own scale is unknown ❓ — `base_per_km` is a game setting (default chosen by visual calibration).
6. **Time.** The simulation runs in `FixedUpdate` at the existing fixed step (≤ 0.1 s for the IDM) after the physics step; the day clock comes from the game's time of day.

### 5.1 `opts-traffic` today (this milestone's slice)

| Module | Content | Tests |
| :--- | :--- | :--- |
| `geometry` | `PathGeometry`: arc/straight, `point_at`, `reversed`, lateral `offset`, `to_world` (object placement) | 7 |
| `graph` | `LaneSegment`, `LaneGraph::build` (spatial-hash geometric joins, direction expansion, stable lane keys, `Elevation` profile), `shortest_route`, `lane_by_key`, `random_spot` | 8 |
| `idm` | `IdmParams` (car, heavy), `acceleration` with speed cap and obstacle | 5 |
| `conflict` | `LaneGraph::conflicts` (lazy per lane: crossing and merging zones with the other lane's zone) | 4 |
| `rightofway` | `precedence` (priority rules, left turners, right before left) | 4 |
| `signals` | `SignalProgram` (repeating cycle), `Aspect::from_state` | 3 |
| `neighbours` | `LaneGraph::neighbours` (lazy parallel same-direction lanes with the position mapping) | 3 |
| `lanechange` | MOBIL-like, target-lane priority, announcement | (agent tests) |
| `agent` | `TrafficWorld` (spawn with free-room check, `step`, lane transitions, leaders across lane borders, dead-end events, `replace_graph` when tiles stream), `DriverCommand` (throttle/brake from acceleration, indicators from the next lane's turn) | 7 |
| `density` | `DensityCurve` (wraps at 24 h), `weighted_pick` | 3 |
| `source` | `segments_from_tile` with the `PathSource` trait (spline `[path]` + object `[path]` → world lanes with `[rule]`s and height profile) | 4 |
| `rng` | SplitMix64 | 2 |
| `examples/lane_graph_audit` | the measurement of §3.1 | — |

---

## 6. Integration steps (what the game needs)

**State 2026-10-11 — steps 1–3 exist in `opts-game` (`omsi_world/traffic.rs`, switch `--traffic N`):** each loaded tile contributes its road lanes (`tile_segments`, with the load job's `.sli`/`.sco` caches as `PathSource` and the terrain height for objects); the graph is rebuilt when the resident tile set changes and agents keep their lane through the rebuild by lane key; the map's `ailists.cfg` group `NormalCars` (else the first group) supplies the vehicle types, loaded lazily (models come from `[model]` of the `.bus`/`.ovh`, lod 0, textures from the vehicle folder); spawning happens every 0.5 s on a random lane 80–220 m from the camera, outside the camera's view cone, thinned by the lane's `trafficdensity` rule (`0` = never), up to `min(--traffic, road length / 250 m × density(hour))`; vehicles are placed with the model origin behind the front bumper by the measured body length, lowered by `[ai_deltaheight]`, and removed at dead ends, on lane loss or beyond 320 m. **Verified in the running game** on Berlin-Spandau (screenshot from a free camera: a column of cars on both carriageways of the main road, aligned with the lanes) and in the log (28 vehicles spawned within 90 s, graph rebuilds of 0–2 ms). **Visuals (2026-10-11):** the AI models animate from a variable table (`opts_bevy::AiVariables`, read through `VariableSource` by the same animation, visibility and light-map systems the scripted vehicles use). Census of the 409 AI models of all installed maps [M] (`ai_model_vars` example): `wheel_rotation_<axle>_<l|r>` (≈ 840 uses), `axle_steering_0_<l|r>` (546), `axle_suspension_*` (≈ 800), light maps on `ai_light` (106) and `lights_blinker_l/r` (6 + 4 visible rules), passengers `ai_pax1…4` (≈ 120 visible rules), no `[spotlight]`/`[interiorlight]` at all. Values: wheel angle from speed and an assumed wheel radius (0.31 m cars, 0.5 m heavy), steering from the lane's curvature (bicycle model, wheelbase 0.6 × length; sign unverified ❓), indicators flash at 0.7 s period from `DriverCommand`, `ai_light` is `1` (dipped beam) when the sun is below 3° and `0` otherwise (OMSI's own AI scripts map `AI_Light` `0` off, `0.5` position lights, `1` dipped, `2` high beam), `ai_brakelight`/`lights_brems` while braking (only 36 script uses of `AI_Brakelight`: most AI cars show no brake lights in OMSI either), passengers drawn by chance (driver 90 %). Tested headlessly (table → wheel rotation, visibility, light-map glow); in the running game the cars drive, the night run (`OPTS_DEV_HOUR=22`) renders, but glow and wheel motion are too small to judge from an overview screenshot ❓. **Not there yet:** signals, right of way by rule (priority values, right-before-left), the dynamic mode (steps 4–7).

**Right of way and lane restrictions (2026-10-11).** Who yields where flows cross is decided by `opts_traffic::rightofway`, in this order: `priority` rules (higher number first; a lane without one ranks between 64 and 192), left turners yield to oncoming straight/right traffic (StVO §9 (3)), otherwise right before left (§8 (1)) judged by the headings where the flows meet; merges and near-parallel flows stay turn-taking by arrival time. A vehicle without right of way waits for others that reach the zone within 6 s; the one with it never waits for them (a vehicle already inside the zone is always respected); the 12 s deadlock breaker covers right-before-left cycles at four-way junctions. Lane restrictions: `bus` = lane for buses, `trucks` = for trucks, `no_cars` = no cars (`LaneRules::allows`); the spawner, the successor choice and lane changes respect them. Scenario tests: the vehicle from the right goes first and the other gives way; a priority road is not held up although the side road is nearer; no waiting for far-away traffic. `[traffic_light_jump]` is a controller clock jump and `[traffic_light_stop]` an arm-level stop line, so neither changes vehicle behaviour yet (`sco.md`, Q-SCO-005).

**Signals, lane changes and lookahead (2026-10-11).** Agents now carry a *plan* — the lanes after the current one, chosen ahead of time to cover 150 m — and look 100 m along it for the leader, a light and conflict zones (short junction lanes made the one-lane lookahead too short to brake for a light at the stop line). *Traffic lights*: a lane governed by a light (`[use_traffic_light]`, next path only) is entered at its start; a vehicle within the program's approach distance (60 m when it says `0`) stops 1 m before it on red and red + yellow, and on yellow when it can stop at 3 m/s² or less; programs repeat from midnight of the session clock ❓; vehicles standing at a red light do not count as "arriving" for other flows' conflict zones. *Lane changes*: parallel same-direction lanes 2.2–5.5 m apart are neighbours (25 % of the road lanes on Ruhrau V2, 64 % on Hamburg109, 40 % on Rheinhausen, none on the single-carriageway Grundorf); a lane change is considered once a second per vehicle, never within 80 m of a junction, a light or below 3 m/s, and at most one vehicle moves per step; it is announced with the indicator. Tests: the car behind a slow one changes lanes and nobody brakes harder than 2.2 m/s²; with a fast car approaching in the target lane the changer waits (and the same test fails when the follower-safety criterion is switched off); a free right lane is taken and kept; the car does not jump sideways at the change and its offset decays to zero in 3 s; no moves to the left where overtaking is prohibited; vehicles stop at red, go on green, run a yellow they cannot stop for; a car waiting at red does not block crossing traffic on green.

**Conflict zones (2026-10-11).** Where flows cross or merge, vehicles take turns: an agent that has not entered a zone looks 60 m ahead (also into the next lane), estimates its arrival time and that of every vehicle approaching the other lane's zone, and stops 1.5 m before the zone if another is earlier (a vehicle already inside always goes first; ties go to the lower id). Vehicles inside a zone are never stopped. A vehicle that has stood 12 s at a zone goes anyway for 5 s, which breaks cyclic waits. Measured on the real maps (`lane_graph_audit`): 27–34 % of the road lanes carry zones (Grundorf 89 of 260 lanes, Ruhrau V2 3,251 of 12,054 with 9,030 zones, Ruhrgebiet 14,643 of 47,450), computing all of them takes 0.3–3 s (they are computed lazily per lane when an agent reaches it, about 40 µs each). Tests: crossing and merging scenarios keep the bodies ≥ 1.4 m apart and both vehicles get through; the same crossing without the handling brings them within 1 m (negative control); parallel neighbours are not slowed. Not in the game screenshot yet — verified headlessly and by running the game for 100 s without errors.

1. **Loader glue (`opts-bevy`)**: implement `PathSource` over the existing content cache (`.sli`/`.sco` are already parsed for rendering and spline attachments), call `segments_from_tile` for each resident tile, build/extend the graph. Optional debug overlay: draw lanes and successors (gizmos) and agent states on F10.
2. **Spawner**: read `ailists.cfg` + `unsched_*.txt` of the active map (and the active chrono's replacements), choose group → vehicle with `weighted_pick`, load the vehicle through the normal vehicle loader with an AI flavour (script `ai = 1`, no driver, `[ai_deltaheight]` applied), skip entries that are not installed.
3. **Kinematic mode**: entity per agent, transform from `Agent::pose` (height from the terrain/spline height query already used for roads), wheel spin from speed, lights from `DriverCommand`.
4. **Dynamic mode**: attach `VehicleController` + `VehicleScripts`; a system writes `Throttle`, `Brake`, `AI_Blinker_L/R`, `AI = 1` before the script step. This needs a closed-loop adapter because the script's thrust law (`M_Wheel = Throttle·M_max·(1 − v/v_max)`) differs from the IDM's acceleration: scale `Throttle` so that the resulting force matches the requested acceleration for the vehicle's mass (a small PI controller on speed is enough and avoids modelling every drivetrain).
5. **Signals**: traffic-light phase clocks per junction object (`ScoTrafficLightGroup`), published to `opts-traffic` as `LaneId → aspect`; paths ending at a red light get a standing obstacle at their end.
6. **Rules**: associate `[rule]` with its spline (§7) and apply speed limits and bans.
7. **Settings / control protocol**: `traffic.set_density`, `traffic.dump` via the existing control server (`docs/src/architecture/control-protocol.md`) so agents can be inspected while the game runs.

---

## 7. Gaps in `opts-core` to close first

| Gap | Why it matters | Where |
| :--- | :--- | :--- |
| ~~`[rule]` blocks without the spline/object they follow~~ — closed 2026-10-11: `MapTrafficRule::owner`, `kind()`; 219 of 520,437 indices (one add-on) point past the owner's paths even with helper arms | — | `formats/map/tile.rs`, corpus test `tile_rules_corpus` |
| **Spline arms of objects** (`[splinehelper]`) have no known length/radius (the last two values are `0 0`, Q-SCO-007), so their paths are not lanes yet — a likely contributor to unconnected lane ends at crossings | complete junction lanes; rule indices already continue into them | spec + OMSI test |
| Traffic-light phase **state codes** are only partly decoded | red/yellow/green decision for AI | `docs/src/formats/text/sco.md` Q-SCO-005 + an observation test in OMSI |
| `[ai_brakeperformance]`, `[ai_veh_type]` kept as unknown fields | braking scale and vehicle class for rule matching | `formats/vehicles/bus.rs` |
| `[aigroup_depot*]` blocks and legacy `[ailist]`/`[aigroup]` are skipped | scheduled AI, old maps | `formats/map/lists.rs` |
| Splines with non-positive length | semantics unknown | spec + OMSI test |
| `Neuendorf`: no road lanes | missing coverage | investigate tile/profile variant |

---

## 8. Later extensions and their hooks

* **Scheduled AI** (buses, trams): `.ttl/.ttp/.ttr`, `Busstops.cfg`, `.hof`, depot groups. A scheduled agent has a route (`shortest_route` between stop lanes), dwell times at stops, and the `AI_Scheduled_*` variables / `ai_scheduled_settarget` trigger of the vehicle scripts ([script sets](../formats/text/vehicles/script-sets.md): `terminus.osc`). `Agent` needs a `route: VecDeque<LaneId>` and a stop plan; `pick_next` already centralises branch choice.
* **Lane changes and overtaking**: MOBIL on top of lane adjacency (parallel same-direction lanes within ≈ one lane width), gated by `overtaking_prohib`.
* **Pedestrians and cyclists**: the graph already keeps `LaneKind::Pedestrian`; cyclists are ordinary road paths with density default `0` in `unsched_vehgroups.txt` (📚).
* **Rail**: `LaneKind::Rail` with switches (`[switch]`, `[switchdir]`) and signal routes.
* **Player interaction**: the player's vehicle as an agent-shaped obstacle (position, speed) so AI brakes for it; honking/priority vehicles (`TrafficPriority` variables).

---

## 9. Calibration and verification plan

Nothing about OMSI's AI *behaviour* is measured yet. Before tuning numbers, observe OMSI (running OMSI is permitted by the clean-room policy; no binaries are inspected) and record results here and in [lists](../formats/map/lists.md):

1. **Speed**: free-road speed of a `NormalCars` vehicle on a road with and without a `speedlimit` rule (is it the limit, or limit × a factor?).
2. **Spacing**: headway behind a standing AI vehicle and at traffic lights (compare `s0`, `T`).
3. **Acceleration/braking**: speed curves (`AI_M_gas_max`, `AI_F_brems_max`, `ai_brakeperformance`).
4. **Density**: number of AI vehicles within 300 m at density factors 0.1, 1, 2 on a known road length (fix `base_per_km`).
5. **Priority**: a junction without rules — confirm right-before-left; with `priority` rules.
6. **Dead ends and the lane-graph audit**: why Düsseldorf (71 %), Tram (71.5 %) and Wuppertal2 (73.6 %) have so many lane ends; which of them OMSI treats as joined.
7. **Missing weights**: frequency of weight-less entries relative to weighted ones.

Automated checks already in place: unit tests of every module, the OMSI-gated lists corpus test, the lane graph audit example (numbers in §3.1 are its output; rerun after any change to the builder or tolerances).

---

## 10. Open questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-AI01 | What are AI vehicles' desired speeds and headways in OMSI (limit × factor? per `[ai_veh_type]`)? | 🔴 High |
| Q-AI02 | Meaning of `[ai_veh_type]` values 0–3 and the second `[ai_brakeperformance]` number. | 🟡 Medium |
| Q-AI03 | Default weight of a weight-less `ailists.cfg` entry (1? the group mean?). | 🟡 Medium |
| Q-AI04 | How OMSI converts a density factor into vehicles per kilometre, and the spawn/despawn radii. | 🔴 High |
| Q-AI05 | ~~Phase state code → aspect mapping~~ — OMSI's own signal-head script gives it (0–2 red, 3–5 red + yellow, 6–8 green, 9–11 yellow, else dark). Open: how AI drivers treat yellow, what `[traffic_light_stop]`/`[traffic_light_jump]` do, the reference instant of the cycle. | 🟡 Medium |
| Q-AI11 | Lane-change behaviour of OMSI's AI (does it change lanes at all, when, how fast)? OPTS: MOBIL-like with gentle thresholds. | 🟡 Medium |
| Q-AI12 | ~~`no_cars`/`bus`/`trucks`~~ — decided (see Q-AI10); how OMSI classifies AI vehicles into cars, trucks and buses (`[ai_veh_type]` 0–3?) is open: OPTS uses a bus for vehicles with a passenger cabin, a truck for others over 7.5 m. | 🟡 Medium |
| Q-AI06 | Semantics of splines with negative length. | 🟢 Low |
| Q-AI07 | Why some maps have so many unconnected lane ends (tolerances, tile borders, different path types). | 🟡 Medium |
| Q-AI08 | Does a density hour beyond 24 wrap or extend the curve (`TH_Wald` has 25.0)? | 🟢 Low |
| Q-AI10 | ~~Which `priority` value is the higher~~ — 192 (corpus probe: straight through-paths); ~~whether the vehicle-type flags allow or forbid~~ — decided: they mean what they say. Open: meaning of `trafficdensity` flag `2`; both to be checked in OMSI. | 🟢 Low |
| Q-AI09 | How `ailists_#low.cfg` / `_#upd.cfg` are selected (open since the lists spec, Q-L02). | 🟡 Medium |

---

## 11. Sources

* Repository specs listed in the header; corpus numbers marked [C]/[M] come from this repository's tools (`ai_lists_corpus` test, `lane_graph_audit` example, read-only keyword counts over `Vehicles/**/*.ovh|bus` and constfiles, inspection of `Scripts/AI_Cars/main_AI*.osc`).
* 🌐 M. Treiber, A. Hennecke, D. Helbing, *Congested traffic states in empirical observations and microscopic simulations*, Phys. Rev. E 62 (2000) — the IDM; summary and parameter table: [traffic-simulation.de, "The Intelligent-Driver Model and its Variants"](https://traffic-simulation.de/info/info_IDM.html).
* 🌐 A. Kesting, M. Treiber, D. Helbing, *General lane-changing model MOBIL for car-following models*, Transportation Research Record 1999 (2007), [doi:10.3141/1999-10](https://journals.sagepub.com/doi/abs/10.3141/1999-10).
* 📚 OMSI community wiki pages already condensed into [lists](../formats/map/lists.md), [tiles](../formats/map/tiles.md) and the [community cross-check](../formats/community-crosscheck.md) (`20-ki-verkehr-von-ailists-pkw-und-menschen`, traffic-rules page).
* A web search for OMSI's AI behaviour found only product pages ([Aerosoft Power Toolkit](https://www.aerosoft.com/en/shop/move/bus-train/omsi-2/downloadpacks-tools/4077/omsi-2-tools-power-toolkit) lists an AI List Editor) — no technical description.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [Map lists & calendars](../formats/map/lists.md), [Script VM](script-vm.md), [Vehicle physics](../simulation/vehicle_physics.md)
- [M-0049 — AI Traffic](../roadmap/phase-4/M-0049.md)
