# AI Vehicle Configuration: A Profile Layer Between Vehicle File and Traffic Model

| Metadata | Value |
| :--- | :--- |
| **Status** | ✅ Implemented 2026-10-14 (phase 1: profile layer and formulas; the measured characterisation of phase 2 and the dynamic mode are open) |
| **Date** | 2026-10-14 |
| **Milestone** | [M-0095](../roadmap/phase-4/M-0095.md) (extends [M-0049](../roadmap/phase-4/M-0049.md)) |
| **Evidence** | [investigation report](../meta/ai-vehicle-config-2026-10-14.md) |
| **Crates** | `opts-core` (parse), `opts-traffic` (decide), `opts-game` (spawn) |

Evidence tags: **[C]** counted over the installed corpus, **[R]** observed in a run, **[K]** established by reading code, ❓ unverified or to be calibrated against OMSI.

---

## 📖 Overview

AI road traffic in OPTS is kinematic ([ai-traffic.md](ai-traffic.md)): the Intelligent Driver Model moves a point along a lane and the game draws a mesh on it. The model is parameterised by `IdmParams`. This document decides **where those parameters come from**. Today they come from two constants chosen by body length; the vehicle file is not consulted. The design adds a typed, testable hand-over (`AiProfile`) from the file to the model and makes the spawner use all groups of the map's AI lists. It does not replace the kinematic mode and does not build the scripted dynamic mode (M-0049 steps 4-7); it makes the profile that mode needs.

## 🎯 Problem Statement

**P1: No parameter flow.** `load_variant` keeps a model path, `[ai_deltaheight]` and a cabin flag; `spawn_traffic` picks `IdmParams::default()` or `::heavy()` by a 7.5 m body-length test and draws the desired speed from 50 km/h ±10 % ([K] `traffic.rs:336-379`, `636-659`). In a run, 110 spawns of 14 vehicle types had desired speeds of 45.0-55.0 km/h whatever the file says [R]; the declared `AI_v_max` is 15-240 km/h, `[mass]` 0.4-15 t, `AI_F_brems_max` 10,000-150,000 N over the 261 vehicles of the lists the spawner uses [C].

**P2: One group.** Only `NormalCars` (else the first group) is used. 781 distinct (group, vehicle) pairs exist over 45 maps; 261 are reachable. Trucks, buses, trams and all 51 `[couple_back]` entries are not [C].

**P3: Class by guess.** Bus iff the file has `[passengercabin]`, lorry iff over 7.5 m. The files document `[ai_veh_type]` themselves (0 car, 1 cab, 2 bus, 3 truck) and 80 files carry it; `[type]` (2 = rail, 3 = air; 884 files) is not tested [C][K].

**P4: No trailers.** The spawner creates one unit and never reads `[couple_back]` [K].

**P5: Invented geometry.** Wheelbase 0.6 × length and wheel radius 0.31/0.5 m feed the animation variables although `BusAxle` carries positions and wheel diameters [K].

**P6: Per-driver variation ignored.** 539 files declare `[rowdy_factor]` (two numbers, randomised per vehicle at creation, per OMSI's own comment); OPTS has an unrelated ±10 % speed jitter [C][K].

## 💡 Proposed Solution

### Commitment

Keep the kinematic traffic model and its pure-Rust boundary. Add one data type that carries *what the vehicle file says* and one function that turns it into *what the model needs*. Keep the two apart so the mapping can be re-calibrated without touching parsing, and so the dynamic mode can reuse the profile.

### D1: `AiProfile` in `opts-core`

`opts_core::formats::vehicles::ai_profile`, built from a parsed `BusManifest` plus the constfiles the manifest names (the existing `constfile` parser):

| Field | From | Missing means |
| :--- | :--- | :--- |
| `mass_kg` | `[mass]` × 1000 (no lower bound: 0.4 t is valid) | `None` |
| `domain` (`Road`, `Rail`, `Air`) | `[type]` (omitted/0/1, 2, 3) | `Road` |
| `kind` (`Car`, `Cab`, `Bus`, `Truck`) | `[ai_veh_type]` 0-3 | `None`; the spawner then guesses as today |
| `v_max`, `v_reach` (m/s) | `AI_v_max`, `AI_v_max_erreich`; without `AI_v_max` a bicycle's script constants: `bike_V_max` (`TH_Fahrrad`), else `vmax_min` + half of `vmax_add` (`HC_Fahrrad`, `HH20_Fahrrad`: 15 + 6/2 = 18 km/h) — 2026-10-26, their scripts cap the speed with them | `None` |
| `gas_max`, `brake_max` | `AI_M_gas_max`, `AI_F_brems_max` | `None` |
| `motor_curve` | `AI_Motorkennlinie` points | `None` |
| `brake_performance` | `[ai_brakeperformance]` | neutral |
| `rowdy` (lo, hi) | `[rowdy_factor]` | (0, 0) |
| `length`, `width` | `[boundingbox]` when present | the mesh extent |
| `wheelbase`, `wheel_radius`, `driven_axles` | `[newachse]` positions, diameters, driven flags | invented as today |
| `couple_back`, `reversed` | `[couple_back]` | no trailer |
| `delta_height`, `has_cabin` | as now | 0, false |

New `BusManifest` fields are needed for `[ai_veh_type]`, `[ai_brakeperformance]`, `[rowdy_factor]` and `[ai_deltaheight]` (so the spawner stops scanning raw text). `[rollwiderstand]`, `[inv_min_turnradius]`, `[schwerpunkt]` and `[rot_pnt_long]` are added to the manifest in the same change because the profile and, later, the dynamic mode need them; the player simulation gains them for free.

A corpus test (OMSI-gated, like `ai_lists_corpus`) builds a profile for every vehicle in every AI group and reports which fields fall back to a default; the accepted fallback list is part of the milestone's acceptance. **Measured (2026-10-14):** over 426 distinct vehicles, mass falls back in 6, brake force in 33, drive torque in 60, top speed in 109 and wheel radius in 0; the test fails above 2 %, 10 %, 15 %, 30 % and 5 % of the vehicles respectively.

### D2: `VehicleTraits` and the mapping in `opts-traffic`

`opts-traffic` stays file-free. It gets `VehicleTraits` (plain numbers: mass, kind, v_max, thrust and brake coefficients, rowdy range, length) and `IdmParams::from_traits(&VehicleTraits, &Calibration, &mut Rng)`. `Calibration` holds the constants that are OMSI behaviour rather than vehicle data (open-road speed, comfort fractions, headway per kind); its defaults are the literature values already in use and are ❓ until Q-AI01 is measured.

Proposed mapping, phase 1 (formulas, all clamped, all in `Calibration`):

| IDM parameter | Rule |
| :--- | :--- |
| `desired_speed` | `min(v_max, lane limit or Calibration::open_road)` × `(1 - rowdy·spread)`; no vehicle exceeds its declared top speed, a 15 km/h scooter stays at 15 |
| `max_accel` | `clamp(f_a · gas_max / (r_wheel · mass), 0.5, 3.5)`, with `r_wheel` from the driven axle (0.31 m default); `f_a` is the fraction of full throttle a driver uses |
| `comfort_decel` | `clamp(f_b · brake_max · brake_performance / mass, 1.0, 3.0)` |
| `max_decel` | `clamp(brake_max · brake_performance / mass, 4.0, 9.0)` (cannot exceed what the tyres give) |
| `time_headway` | 1.5 s car and cab, 1.7 s bus and truck (the literature values the plan already cites) |
| `length` | from the profile |
| `class` | from `kind`, else the existing heuristic |

The units of `AI_M_gas_max` and the exact meaning of `AI_v_max` against `AI_v_max_erreich` are inferred from the AI script's `M_Wheel = Throttle · M_gas_max · (1 - v / v_reach)` ([ai-traffic.md](ai-traffic.md) §2.2) and are ❓. *Phase 2 status (2026-10-14): implemented.* Phase 2 replaces the formulas for acceleration and braking by a **measured characterisation**: run the vehicle's own `main_AI.osc` in the script VM against `opts-sim` once per vehicle (full throttle to v_max, full brake from 50 km/h), cache `a(v)` and the stopping distance in the decoder cache, and feed those into `VehicleTraits`. This uses the real mass and the real constants without modelling every drivetrain, but depends on the consist-mass fix (M-0053) for articulated vehicles.

### D3: all groups, class, domain

*Implemented as designed; one rule was added after the corpus census: a vehicle is a rail vehicle only when `[type]` is 2 **and** it declares `[boogies]` (860 of the 877 `[type] 2` files do), because two road scooters declare `[type] 2` without bogies.*

The spawner builds its pool from every group of the map's `ailists.cfg` that `unsched_vehgroups.txt` marks with a non-zero density (per-group density through `unsched_trafficdens.txt`, which the pool already reads for one group). Entries whose `domain` is `Rail` or `Air` are never placed on road lanes. A group's own density decides how often its vehicles appear, so lorries and buses are rarer than cars where the map says so. Lane bans (`bus`, `trucks`, `no_cars`) use `kind`.

### D4: trailers

A profile with `couple_back` spawns a chain: the lead unit follows the lane as now; each trailing unit is a `TrailerKinematics` follower (the same kinematic joint the player's consist uses, the articulation physics design on branch `design/articulation-physics`, not yet on master) behind the previous one. The agent's `length` for gaps and spawn room is the sum of the units' lengths plus the coupling gaps. Despawn removes the whole chain.

### D5: animation inputs from the profile

Wheel radius, wheelbase and the steering angle come from `wheel_radius` and `wheelbase` of the profile; the 0.31/0.5 m and 0.6 × length constants remain only as fallbacks.

### Scope bound

Not part of this design: lane graph, signals, priority, the scripted dynamic mode, scheduled AI, where vehicles spawn, the speed law in curves. A curve speed limit is a separate model feature; the profile will carry the inputs it would need (`[inv_min_turnradius]`, `[schwerpunkt]`).

## 🏗️ Architecture Fit

| Crate | Change |
| :--- | :--- |
| `opts-core` | `BusManifest` fields; `ai_profile` module; corpus test. No new dependency. |
| `opts-traffic` | `VehicleTraits`, `Calibration`, `IdmParams::from_traits`; unit tests of the mapping. Still no Bevy, no I/O. |
| `opts-game` (`omsi_world/traffic.rs`) | build a profile in `load_variant`, use all groups, class and domain filter, trailer chain, animation inputs |
| `opts-bevy` | reuse of the trailer follower; no change expected |

## 🔗 Dependencies

* [M-0049](../roadmap/phase-4/M-0049.md): the traffic model this extends.
* [M-0053](../roadmap/phase-4/M-0053.md): consist mass, needed for correct dynamics of articulated vehicles in the later dynamic mode, not for phase 1.
* Calibration against OMSI (Q-AI01, Q-AI04) improves the numbers but does not block the structure.

## 📦 Implementation Plan

1. **Parse.** Manifest fields and `AiProfile`; corpus test of fallbacks.
2. **Map.** `VehicleTraits`, `Calibration`, `from_traits`; tests that two vehicles of different mass and brake force get different parameters and that the clamps hold.
3. **Spawn.** Profile in the loader, all groups with density, domain and kind filters; debug log line per spawn with the profile summary.
4. **Chains.** Trailer units for `couple_back`; despawn together.
5. **Visuals.** Wheel and steering inputs from the profile.
6. **Verify** (below) and update `ai-traffic.md` (Q-AI02, Q-AI12 answered, §2.2 and §7 gaps closed).

## 🧪 Verification Strategy

* **Unit tests** (`opts-traffic`): the mapping is monotone (more mass lowers `max_accel`, more brake force raises `max_decel`, a lower `v_max` lowers `desired_speed`), clamps hold, rowdy spread stays inside its range, headway differs by kind.
* **Headless comparison test:** two agents on the same straight lane from rest, a 0.8 t car and a 15 t truck with their real constants; the car reaches 40 km/h at least 30 % sooner, and from 50 km/h the stopping distances differ in the order of `brake_max / mass`.
* **Corpus test** (OMSI-gated): every vehicle of every AI group yields a profile; fallbacks are listed and only the accepted ones occur.
* **Runtime check:** the spawn log line (`AI vehicle spawned … profile …`) over a headless run shows per-type desired speeds that follow `AI_v_max` and the lane limit, and no `[type]` 2/3 vehicle on a road lane; a map with a `Gelenkbusse` group shows a head and trailer moving as a chain.
* **Regression:** all existing `opts-traffic` tests pass with the default `VehicleTraits`.

## ⚠️ Risks and Open Questions

| ID | Question or risk | Handling |
| :--- | :--- | :--- |
| Q-AIC-001 | Units and exact role of `AI_M_gas_max`, `AI_v_max`, `AI_v_max_erreich` (❓ from the script law) | Phase 1 formulas are clamped and flagged; phase 2 measures through the script VM |
| Q-AIC-002 | What `[rowdy_factor]` scales in OMSI (speed, headway, acceleration, braking?) | Phase 1 scales desired speed only; measure in OMSI |
| Q-AIC-003 | Second and later numbers of `[ai_brakeperformance]` (five numbers in at least one file) | First number only, as a brake multiplier; the rest ignored and listed |
| Q-AIC-004 | Real traffic mix: lorries and buses over a map's density may raise the load (heavier meshes, more `[couple_back]` chains) | Per-group density and the existing cap; measure on a dense map |
| Q-AIC-005 | `[type]` 2 on a road scooter in a `NormalCars` list contradicts "2 = rail" | Domain filter uses `[type]` only together with `ai_veh_type`/lane kind until clarified; log the case |
| Q-AIC-006 | Vehicle files with missing or zero mass or constants | Fallback to today's defaults, counted by the corpus test, never a panic |
| R1 | The mapping can look "right" in tests and still not match OMSI | Calibration plan in `ai-traffic.md` §9; the profile layer makes recalibration a table change |
| R2 | A 15 t `[mass]` with a 30,000 `AI_M_gas_max` may give an acceleration above what a lorry shows | the clamps (0.5-3.5 m/s²); phase 2 measurement |

## 🔄 Alternatives Considered

1. **Per-class constants only** (car, cab, bus, truck sets chosen by `[ai_veh_type]`). Cheapest, needs no constfile reading, and removes the worst uniformity. Rejected as the end state because it still ignores mass and the declared constants, but it is the **fallback** inside the mapping when a profile field is missing.
2. **Run the full AI script for every AI vehicle** (the plan's dynamic mode). Most faithful, and it is the end goal, but it needs the script bridge, a throttle controller for the IDM acceleration and the consist-mass fix; it cannot be the fix for the current symptom and costs far more per vehicle. The profile is its prerequisite.
3. **Read vehicle files lazily inside `opts-traffic`.** Would break the no-I/O rule and the headless, replayable design.
4. **Calibrate one global parameter set better.** Treats the symptom; every vehicle would still behave alike.

## 🔎 Discovered during investigation

* `[ai_veh_type]` is documented by the vehicle files (0 car, 1 cab, 2 bus, 3 truck), which answers Q-AI02 for that keyword and most of Q-AI12 ([C]: 50 trucks, 23 cars, 5 cabs, 2 buses among the 80 files).
* `[rowdy_factor]` exists in 539 files and is read by nothing in OPTS.
* The player simulation's `from_manifest` discards any `[mass]` of 0.5 t or less (`opts-sim/src/vehicle/mod.rs:126-129`).
* `[rollwiderstand]`, `[inv_min_turnradius]`, `[schwerpunkt]` and `[rot_pnt_long]` (1,900-2,700 files each) are read nowhere in the repository.
* `misc-types.md` names a parser module `opts_core::formats::ovh` that does not exist.

## 📝 Provenance

Files read and commands run are listed in the [investigation report](../meta/ai-vehicle-config-2026-10-14.md). Corpus numbers come from read-only scans of `<OMSI>/Vehicles` (2,752 `.bus`/`.ovh` files) and the 45 maps' `ailists.cfg`; the runtime numbers from one headless run on Waldhofen.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [AI Road Traffic: Research and Integration Plan](ai-traffic.md), [M-0049](../roadmap/phase-4/M-0049.md)
- [Vehicle scripts: the route for OPTS](vehicle-scripts-route.md), [`.bus` format](../formats/text/vehicles/bus.md), [`.const` files](../formats/text/vehicles/const.md)
- Articulation physics design (branch `design/articulation-physics`, not yet on master)

## Phase 2 results (2026-10-14)

`opts_sim::script::measure_ai_response` runs a vehicle's script with the AI flags (`AI`, `AI_Engine`, `Throttle` 1) at eight speeds and with `Brake` 1 and records `M_Wheel` and `Brakeforce`; the spawner feeds the peak torque and the brake force into `VehicleTraits` when the script is live and keeps the constants-based values otherwise. Over the 426 vehicles of the map AI lists (`ai_response` example): **317 follow the shared car law** (so the measurement equals the constants-based value; on Waldhofen all 91 spawns of a headless run have identical parameters before and after), **85 deviate** (ships, trains, bicycles, scooters, lorries; for bicycles and scooters the measurement supplies the first real torque curve), **23 are inert** and 1 has no script. Most deviating lorries give zero torque with the inputs the measurement feeds: their scripts need the gearbox and engine inputs of the bus script work ([bus script integration](bus-script-integration.md), phase P0), so the measurement doubles as a probe for the completeness of that input feed. A diagnostic log line `traffic state` (every 15 s, debug level) reports target, agents, road length, densities, vehicle slots and why spawn attempts were rejected.

