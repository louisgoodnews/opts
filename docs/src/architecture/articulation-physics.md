# Articulation Physics: One Joint Model, Consist Mass and Deterministic Pitch

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Draft — awaiting review |
| **Phase** | Phase 4 (`opts-sim` & `opts-game`) |
| **Milestone** | [M-0053](../roadmap/phase-4/M-0053.md) (existing, Phase A "Specification"); boundary to [M-0073](../roadmap/phase-10/M-0073.md). A follow-up implementation milestone is **not yet assigned** |
| **Author** | Claude Sonnet 5.5 (the task prompt named a different agent, "Gemini 3.8 Flash"; the project lead corrected this on 2026-10-14) |
| **Date** | 2026-10-14 |
| **Related ADRs** | [ADR-0002](../adr/0002-clean-room-policy.md) (clean room: no executable was inspected) |
| **Related Specs** | [articulation.md](articulation.md), [animation-axes.md](animation-axes.md), [bellows-census.md](bellows-census.md), [vehicle physics](../simulation/vehicle_physics.md), [determinism-replays.md](determinism-replays.md) (stub), [chain-assembly.md](chain-assembly.md), [critical review](../meta/critical-review-2026-10-11.md) F-002, F-003 |

## 📖 Overview

An articulated bus is two or more units joined by a turntable. OPTS today moves such a consist with **three unrelated mechanisms**: a kinematic follower for the yaw of each trailing unit, a frame-rate-driven ground follower for pitch and roll, and a leftover single-body physics model that knows nothing about trailers. This document decides what the *coupling model* is, so that the consist has one state, one mass, one clock.

The question it answers is the one M-0053 Phase A leaves open: "how the joint transmits force". The answer given here is **a commitment to keep the kinematic joint as the authoritative pose model and to add what is missing around it** (shared state in `opts-sim`, consist mass in the longitudinal dynamics, a fixed-step pitch/roll source), instead of replacing it with a two-rigid-body constraint simulation. Sections "Proposed Solution" and "Alternatives Considered" give the argument and the conditions under which the commitment must be reopened.

## 🎯 Problem Statement

Four defects are established by the code as it stands (2026-10-14, branch base `master` at `788c9f2`). None of them is a hypothesis.

**P1 — Two answers to "what is the joint angle" (F-002).** `VehicleSimulation` still builds an `ArticulationJoint` with hard-coded geometry (`ArticulationJoint::new(coupling_y, 1.2, 6.0)`, [mod.rs:150](../../../crates/opts-sim/src/vehicle/mod.rs)) and steps it every tick ([mod.rs:258](../../../crates/opts-sim/src/vehicle/mod.rs)). Nothing outside `opts-sim` reads the result (`grep ArticulationJoint\|\.articulation\b crates` finds only the definition, the re-export and these two sites). The angle the models and scripts see comes from `TrailerKinematics`, whose state lives in a Bevy component (`ArticulatedSegment.state`, [articulation.rs:47](../../../crates/opts-bevy/src/articulation.rs)), *outside* the simulation struct.

**P2 — The consist is simulated as the head unit alone.** The loader takes the mass from the head's `.bus` only: `from_manifest` sets `total_mass_kg` from `manifest.mass_tonnes` of that one file ([mod.rs:125-150](../../../crates/opts-sim/src/vehicle/mod.rs), [chassis.rs:25](../../../crates/opts-sim/src/vehicle/chassis.rs)); `spawn_trailer_chain` builds trailing units as render entities and does not touch the mass ([vehicle.rs:508-516](../../../crates/opts-game/src/in_game/omsi_world/vehicle.rs)). The only mass adjustment in that file is for rail consists (`commands_mass`, at least 20 t, [vehicle.rs:754-757](../../../crates/opts-game/src/in_game/omsi_world/vehicle.rs)). Evidence of the effect [M, `consist_census` example, branch `agent/consist-mass` commit `e4f1949`; first made with an ad-hoc scan, see Provenance]: in the 480 head/trailer links OPTS can see where both vehicle files state a mass (501 on the raw text, see "Discovered during investigation"), the trailer weighs a median 0.90 of the head (p10 0.60, p90 1.11). A typical articulated bus therefore accelerates and brakes like a vehicle roughly **half its real mass**. Torque from the scripts (`ScriptDrive`) is divided by the head's mass only ([mod.rs:189-200](../../../crates/opts-sim/src/vehicle/mod.rs)).

**P3 — Pitch and roll are not tied to the simulation clock.** `articulation_<k>_alpha` is computed in `FixedUpdate` (64 Hz default, `--physics-hz`, [args.rs:169](../../../crates/opts-cli/src/args.rs)), but `articulation_<k>_beta`/`_gamma` and every trailer's height, pitch and roll are computed in `follow_ground`, a system in **`Update`** ([in_game/mod.rs:147](../../../crates/opts-game/src/in_game/mod.rs)), with a first-order filter that uses the frame's `delta_secs` (`let dt = time.delta_secs().min(0.1)`, [ground_height.rs:228](../../../crates/opts-game/src/in_game/omsi_world/ground_height.rs); filters at lines 304 and 359). The same drive at 30 and at 144 frames per second therefore publishes different β/γ sequences, and the values written into the script engine are not a function of the fixed step. This contradicts the stated requirement that scripts run "deterministic by construction: fixed 60 Hz step … replays record only inputs" ([vehicle-scripts-route.md:75](vehicle-scripts-route.md)). (That page also names 60 Hz; the implemented default is 64 Hz — documentation drift, noted in Follow-ups.)

**P4 — The trailing state cannot be saved.** Trailing-unit state (`TrailerKinematics.heading`, `origin`, `prev_hitch`) is private to a component that is not part of `VehicleSimulation`, which is the struct that derives `Serialize`/`Deserialize` ([mod.rs:43-62](../../../crates/opts-sim/src/vehicle/mod.rs)). A restored session ([M-0069](../roadmap/phase-10/M-0069.md), in flight on its own branch) can restore the head but must re-seed the trailers straight behind it (the same path as a position jump, `JUMP_THRESHOLD_M`, [articulation.rs:27](../../../crates/opts-bevy/src/articulation.rs)): a bus saved in a tight curve would come back with a straightened bellows.

What is **not** a defect, and must not be "fixed": the yaw of trailing units follows from a rolling-without-slipping rear axle ([trailer.rs:1-14](../../../crates/opts-sim/src/vehicle/trailer.rs)), which is consistent with the head, whose yaw is itself a kinematic bicycle model (`yaw_rate = v/L · tan δ`, [mod.rs:225](../../../crates/opts-sim/src/vehicle/mod.rs)). The whole vehicle has no tyre-slip model today.

## 💡 Proposed Solution

### Commitment

**D0 — The joint stays kinematic.** The pose of every trailing unit is a function of the leader's path (`TrailerKinematics`), not of integrated forces on a second rigid body. Forces are added only where the current model has a *measured* error (P2): longitudinal mass and load. This is a hybrid by design:

| Quantity | Model | Why |
| :--- | :--- | :--- |
| Yaw `α` of each joint | Kinematic: hitch pinned to the leader, rear axle rolls without side slip (unchanged) | Matches the head's no-slip bicycle model; deterministic by construction; reproduces the closed form below |
| Longitudinal motion of the consist | **New:** one body of mass `Σ mᵢ`; per-unit axle loads and brake shares | Fixes P2; needs no constraint solver |
| Hitch force (report only) | `F_h,i = (Σ_{j>i} m_j)·a + resistances`, computed, not integrated | Gives scripts/diagnostics a value; no feedback into poses |
| Pitch `β`, roll `γ` | Geometric: unit lies on the ground under its own axles; **sampled and filtered in the fixed step** | Fixes P3; sign convention unchanged |
| Joint limits | `max_alpha` hard clamp; `min_beta`/`max_beta` clamp; `γ` unlimited (no data) | Maps `[coupling_front_character]` field by field, see D4 |

Closed form that the model must reproduce (and the existing test `circle reaches a steady α` already approximates): for a hitch on the leader's rear axle and a trailing axle `L` behind the hitch on a circle whose hitch radius is `R_h`, `sin α = L / R_h`.

### D1 — One state, in `opts-sim`

`TrailerKinematics` moves from the `ArticulatedSegment` component into `VehicleSimulation` as `consist: Vec<TrailerKinematics>` (serialised, including `prev_hitch`). `ArticulationJoint` is deleted. `opts-bevy::articulation_system` becomes a *reader*: it steps the simulation, then copies poses to transforms and script variables. This is the F-002 fix as M-0053 Phase B states it ("replace `ArticulationJoint` with `TrailerKinematics` inside `VehicleSimulation`"), with the addition that the state is serialisable (P4). Jump handling (spawn, respawn, push-out) becomes an explicit `VehicleSimulation::teleport(pos, heading)` call that re-seeds the consist, instead of a distance heuristic inferred from two frames of the leader's position ([articulation.rs:205-215](../../../crates/opts-bevy/src/articulation.rs)); the heuristic stays only as a safety net for callers that move the chassis directly.

### D2 — Consist mass in the longitudinal dynamics

`opts-sim` receives the per-unit data the loader already parses (`mass_tonnes`, axle positions per `.bus`), as a list `UnitLoad { mass_kg, axle_y: Vec<f32>, … }` (names illustrative). The longitudinal equation uses the sum of masses. Normal loads on driven and braked axles use the per-unit axle positions, so that brake force is limited by `μ · ΣN_braked` and drive force by `μ · ΣN_driven` (the cap `ROAD_FRICTION · m · g` in [mod.rs:191](../../../crates/opts-sim/src/vehicle/mod.rs) becomes the sum over the relevant axles). The hitch force is derived, not integrated. Scripts continue to supply torque and brake force (`ScriptDrive`); only the mass they act on changes.

**Implemented interface** (`opts_sim::vehicle::consist`, M-0053 Part 2): `UnitLoad { mass_kg: f32, axles: Vec<UnitAxle { longitudinal_m: f32, is_driven: bool }> }`, filled from the unit's `.bus` by `UnitLoad::from_manifest(manifest, reversed)` (`mass` in tonnes, more than 0.5 t, else the head's generic default of 11.5 t; axle positions mirrored for a reversed unit). `VehicleSimulation` keeps one per following unit (`trailer_loads`, serialised); the head's own mass and axles stay in `chassis`/`axles`. Normal load per axle is the unit's weight distributed by the suspension model's static rule with the centre of gravity at the middle of the axle span (hitch load on the unit ahead neglected). Every axle brakes (no brake layout exists in the files); if no axle of the consist is marked driven the driven sum falls back to all axles. Consequence for a solo vehicle under script drive: the traction cap is now `μ·N_driven` (half the weight on a symmetric two-axle bus) instead of `μ·m·g`.

The interface to M-0073 is deliberately narrow: **M-0073 owns how `UnitLoad` is filled** (empty mass, passengers, inertia, axle loads, spring and damper data); this document owns **what the consist does with it**. Until M-0073 exists, `UnitLoad` is filled from `mass_tonnes` and `[axle]` blocks exactly as the head is today.

### D3 — Pitch and roll in the fixed step

The ground sampling that `follow_ground` does per frame moves into the fixed step as a pure function of (unit pose, ground heights), with the filter's `dt` equal to the fixed timestep. Because `opts-sim` has no Bevy dependency, the Bevy side supplies the sampled heights per fixed step (four heights per unit, as `follow_ground` already samples them: rear axle ± half track, front joint ± half track, [ground_height.rs:330-343](../../../crates/opts-game/src/in_game/omsi_world/ground_height.rs)); `opts-sim` computes elevation, pitch, roll and `β`, `γ` from them. The render system keeps interpolating between the last two fixed poses, as it does for yaw today (`articulation_render_system`). The *head's* own body motion (fall under gravity, `RISE_RATE`, `TILT_RATE`) stays in `follow_ground` for now: it is a render-side smoothing of a unit whose pose the simulation does not integrate vertically. Moving it is M-0073's business (suspension heave/pitch/roll).

### D4 — What the joint carries

Mapping of the `[coupling_front_character]` fields (`max_alpha`, `min_beta`, `max_beta`, `kind`, [articulation.md](articulation.md) "What OMSI does") to the model:

| Field | Role in the model |
| :--- | :--- |
| `max_alpha` | Hard clamp of `α` (already in `TrailerKinematics`, `max_alpha`); reaching it is reported as "at stop" |
| `min_beta`, `max_beta` | Clamp of the published `articulation_<k>_beta` (already done, `ArticulatedSegment::clamp_beta_deg`); the *pose* of the unit is not clamped, only the bellows variable, because the unit rests on the ground |
| `kind` | `1` bus turntable, `0` truck coupling. The probe of `articulation.md` found one file with `0` (reversed β limits). **No behavioural difference is modelled** until Q-ART-04 is answered |
| roll (`γ`) | No field exists. The joint is treated as free in roll (Q-ART-07) |
| yaw/pitch/roll *stiffness* | No field exists; none is modelled. The joint is a pin plus clamps, not a spring |

### D5 — The kinematic follower is the only follower

There is no "physics mode" to fall back from. AI and parked vehicles at LOD distance use the same step with ground sampling skipped (pitch and roll zero) — which is what `articulation_render_system` already does for a segment without a ground pose. Consequently the existing kinematic tests remain valid as they are; they move with the code into `opts-sim`.

### Scope bound

This document does **not** cover: chains of more than two *interacting* bodies beyond the kinematic chain (a double-articulated consist works because each follower only reads its leader); tyre/ground contact forces within a trailer; drag and rolling resistance on trailer axles beyond summing `m·g·Crr` over the consist; any change to how `.osc` scripts compute the variables `positive_/negative_articulation_offset` etc. (the vehicle's own `articulation.osc` derives those from α and β, [animation-axes.md](animation-axes.md) table); push-pull units (`[couple_front]`, 21 files [M]); the external camera (M-0053 Phase C).

## 🏗️ Architecture Fit

| Layer / crate | Change |
| :--- | :--- |
| `opts-core` (L0) | None. `BusManifest` already exposes `mass_tonnes`, couplings, `CouplingLimits` |
| `opts-sim` (L1, no Bevy) | Owns the consist state (`TrailerKinematics`, serialisable), `UnitLoad`, the consist mass in `VehicleSimulation::step`, the β/γ computation from supplied ground heights. `ArticulationJoint` removed |
| `opts-bevy` (L2) | `articulation_system` reads the simulation; `ArticulatedSegment` keeps render-side data only (model offsets, wheel spin, `reversed`) |
| `opts-game` (L3) | `spawn_trailer_chain` fills `UnitLoad`s; `follow_ground` keeps the head's body motion and hands per-unit ground heights to the fixed step |

Boundary rules respected: `opts-sim` stays free of Bevy; the sampled heights cross the boundary as plain numbers. No boundary moves. One consequence needs a decision (Q-ART-09): where the fixed-step ground heights come from, because `GroundIndex` lives in `opts-game` while `FixedUpdate` systems live in `opts-bevy`; the ground query of [M-0025](../roadmap/phase-4/M-0025.md) is the intended home (not merged at the time of writing).

## 🔗 Dependencies

| Dependency | State |
| :--- | :--- |
| `TrailerKinematics`, `ArticulatedSegment`, `ArticulationPlugin` | Available |
| Parsing of `mass`, axles, `[coupling_front_character]` | Available (`bus.rs`) |
| Ground height at arbitrary `(x, y)` in a fixed-step system | **Blocked on M-0025** (`GroundElevationQuery` exists on an unmerged branch; the game's `GroundIndex` is not reachable from `FixedUpdate` in `opts-bevy`) |
| `VehicleSimulation` serialisation used by session save | In flight ([M-0069](../roadmap/phase-10/M-0069.md), `feat/session-save`); D1 must agree with its section list |
| `UnitLoad` contents beyond `mass`/axles | M-0073 (planned); not needed for the first step |
| OMSI reference for the β sign and α sign | **Blocked on a human check** (Q-ART-01, Q-ART-02); the design does not change with the answer |
| New external crates | None |

## 📦 Implementation Plan

1. **Move the state (D1).** `VehicleSimulation` gains `consist`; `TrailerKinematics` derives `Serialize/Deserialize`; `ArticulationJoint` and its test are removed; `articulation_system` reads the sim. Verifiable: existing `trailer::tests` and `tests/articulation.rs` pass unchanged; a new test asserts the published α equals `consist[0].alpha()` (F-002 acceptance in M-0053 Phase D).
2. **Explicit teleport (D1).** `VehicleSimulation::teleport`; spawn and respawn call it; the jump heuristic stays as a safety net and logs when it fires. Verifiable: after `teleport` the consist hangs straight with zero velocity; no wheel-speed spike (regression for `chain-assembly.md` behaviour).
3. **Serialisation round trip (P4).** A bus is driven into a 15 m circle, serialised, restored, driven on: poses and α continue within 1 mm / 0.01° of the unbroken run.
4. **Consist mass (D2).** `UnitLoad` filled by `spawn_trailer_chain`; longitudinal equation and friction caps use sums. Verifiable: acceleration from rest with a given wheel torque scales with `1/Σm`; stopping distance of head-only vs consist differs by the mass ratio within 2 %.
5. **Fixed-step ground heights (D3).** Interface for per-unit sampled heights (depends on M-0025); `follow_ground` keeps the head. Verifiable: determinism test (Verification, V3).
6. **β/γ and trailer pose from the fixed step.** `follow_ground`'s per-segment block is removed; `articulation_render_system` interpolates. Verifiable: V3 and the existing bellows tests (`bellows_beta_moves_the_follower_end_like_the_follower_body`).
7. **Hitch force (report).** Computed in `opts-sim`, exposed as a diagnostic (and to the control protocol only if M-0073 asks). Verifiable: `F_h` of the last unit equals `m_last·a + resistances` on a straight run.
8. **Documentation closure.** `articulation.md` is rewritten to describe the shared state, deletes L-ART-01 and the "at most 4 segments" statement (the code allows 40, [vehicle.rs:508](../../../crates/opts-game/src/in_game/omsi_world/vehicle.rs)), and `vehicle_physics.md`'s hitch formula section is aligned with the implemented equation.

Steps 1–4 do not depend on M-0025; steps 5–6 do. A milestone should be scoped so that 1–4 can land first.

## 🧪 Verification Strategy

| # | Test | Where | Evidence it provides |
| :--- | :--- | :--- | :--- |
| V1 | One-state test: in a headless app, the α published to the script engine equals the simulation's own joint angle at every fixed step for a 600-step S-curve | `opts-bevy/tests/articulation.rs` | F-002 closed |
| V2 | Closed form: hitch on the rear axle, circle of radius `R_h`, `sin α = L/R_h` within 0.2° after settling | `opts-sim` | Kinematic model unchanged by the move |
| V3 | **Frame-rate independence**: run the same input script with `Update` frame times of 16.7 ms and 7 ms (virtual time); the sequence of published `alpha`, `beta`, `gamma` per fixed step is bit-identical. This test is **expected to fail on the current code for β/γ** (P3; predicted from the code, not run) and is the acceptance test of steps 5–6 | `opts-bevy/tests` | Determinism |
| V4 | Serialisation round trip (plan step 3) | `opts-sim` | P4 closed |
| V5 | Mass: `a = F/Σm`, stopping distance ratio (plan step 4), on the MAN NG272/NG313 manifests with the trailers' real `mass` | `opts-sim` + corpus (skipped without `OMSI_PATH`) | P2 closed |
| V6 | Corpus sweep: every `.bus`/`.ovh` with `[couple_back]` (1,035 files [M]) runs a standard manoeuvre (5 m/s, 15 m circle, then S-curve) headless: finite poses, |α| ≤ `max_alpha`, coupling gap < 5 cm | example + test | No regression across the fleet |
| V7 | Visual: an articulated bus on a crest and in a tight curve, screenshot pair before/after (M-0053 Phase D) | game | β/γ are plausible |

**Fidelity target.** There is no OMSI reference data for α or β at a known manoeuvre in the repository, so the target is the closed form (V2) and the mass ratio (V5), not "matches OMSI". The OMSI comparison is Q-ART-01/Q-ART-02 and is done by a person with OMSI running; it validates *signs*, which the design leaves independent of the model. A milestone following this document should name **one test vehicle** — the MAN NG272, as in P-1011 — and treat the fleet sweep V6 as a no-regression check only.

## ⚠️ Risks and Open Questions

| ID | Question / risk | Status |
| :--- | :--- | :--- |
| Q-ART-06 | **Is the kinematic yaw model good enough?** Estimate (assumptions, not project data): at 5 m/s on a 15 m circle the lateral acceleration is 1.7 m/s²; a trailer carrying 7.4 t on essentially one axle needs ≈ 12 kN of side force; with an assumed twin-tyre cornering stiffness of 200–300 kN/rad per axle the side slip is 2–3°, of the same order as the resulting error of α. It grows with `v²`. The head has the same no-slip assumption, so the consist is internally consistent. **Reopen the commitment if** a vehicle file or test shows tyre data that the model can use, or if scenarios with low friction, high speed or emergency braking in curves (jackknifing) are put on the roadmap. | ❓ open (estimate unverified) |
| Q-ART-07 | `γ` (roll) has no limit field in `[coupling_front_character]`. Is it unlimited in OMSI, or limited by something else (the `kind`)? | ❓ open |
| Q-ART-08 | Does `articulation_<k>_alpha` pass through the script before the bellows (e.g. via `articulation.osc` producing `articulation_alpha_abs`, `_pos`, `_neg`)? If scripts transform α, the OPTS value is only the *input* of that chain; the signs Q-ART-01 refers to would then be those of the input. | ❓ open |
| Q-ART-09 | Which crate hands per-unit ground heights to the fixed step: a new `opts-bevy` resource backed by the M-0025 ground query, or a trait in `opts-sim`? Depends on how M-0025 is merged. | ⏳ waits for M-0025 |
| Q-ART-10 | Cross-platform bit-identity. The sim uses `f32` (`VehicleSimulation`) and `f64` (`TrailerKinematics`) with `sin`/`cos`; Rust's `std` transcendental functions are not guaranteed to be bit-identical between platforms. The requirement stated for this design is *same build, same machine* replay and save/load; networked multiplayer is a long-term vision item only ([vision.md:91](../vision.md)) with no milestone. State the requirement explicitly in `determinism-replays.md`, which is still a stub. | ❓ open (requirement not decided) |
| Q-ART-11 | Should the hitch force feed back into the leader (reaction on the head's yaw)? The kinematic model gives none; a loaded trailer on a slope at the hitch would. Not modelled; decide when M-0073 defines the head's body dynamics. | ❓ open |
| Q-ART-12 | Rail consists use a separate mass rule (`commands_mass`, ≥ 20 t). Does the consist mass of D2 replace it for trams, or stay a separate path? | ❓ open |
| R-1 | The jump heuristic and `teleport` can disagree (a collision push-out the caller does not announce). Mitigation: step 2 keeps the heuristic as a safety net and logs it. | known |
| R-2 | β/γ move from per-frame to per-fixed-step: at low `physics_hz` (20 is the clamp minimum, [app.rs:95](../../../crates/opts-game/src/app.rs)) the filter is coarser; render interpolation hides it for pose, not for script variables, which change at the fixed rate (as α does today). | accepted |
| R-3 | Chains that loop back on themselves are not detected by the loader and run to its cap of 40 segments (3 head files under the OPTS parser, 13 on the raw text). V6 must report the longest chain it ran and the loops it skipped; loop handling is Q-ART-13. | open |
| Q-ART-13 | Second line of `[couple_back]` and loops: the corpus contradicts "the trailer has a further trailer" (158 links with `false` whose target has its own `[couple_back]`, 37 with `true` whose target is the last unit); OPTS reads it as "coupled facing backwards". What does OMSI do with a chain that refers back to an earlier unit? Also recorded as Q-ART-13 in [articulation.md](articulation.md). | ❓ open |
| Q-ART-14 | Does OMSI honour an indented keyword (`\t[couple_back]`)? **Investigated 2026-10-14** (`docs/src/formats/text/keyword-indentation.md` (branch `fix/keyword-indentation`)): most likely not. Indented keywords are editor help blocks and disabled blocks (indented `.ovh` `[couple_back]` names an existing file in 3.6 % of 497 cases against 98.8 % at the start of a line), the scanner stays as it is, and the "496 lost trailers" were mostly dead references. A minority (8 `.bus` trailers, 18 `.ovh`, indented `[friendlyname]` in 1,350 vehicle files) is unexplained and needs an OMSI test. | 🟡 mostly answered |
| R-4 | Q-ART-02 (β sign). If OMSI's bellows meshes were authored against the opposite convention, the P-1011 sign (`β = leader_pitch − pitch`, [bellows-census.md:78-100](bellows-census.md)) is wrong for every articulated bus. **This design does not change the sign** — the definition depends only on the two pitch values, not on how they are produced — but D3 changes *when* and *how smoothly* β is produced, so the slope screenshot that the sign check needs should be taken after step 6, not before. | dependency |

## 🔄 Alternatives Considered

**A1 — Two simulated rigid bodies with a pin constraint.** Each unit gets mass, inertia and its own tyres; a constraint solver (sequential impulses or a spring-damper joint) keeps the hitch closed. *Rejected for now:* (a) the head is a kinematic bicycle with a yaw torque smoothing term, not a rigid body with tyre forces ([mod.rs:225-233](../../../crates/opts-sim/src/vehicle/mod.rs)); a force-based trailer behind it would be more detailed than its leader and would need a tyre model that no vehicle file parameterises (`BusAxle` carries spring rate, max force, damping, no cornering stiffness, [bus.rs](../../../crates/opts-omsi/src/formats/vehicles/bus.rs)); (b) a stiff constraint needs sub-stepping or implicit solving to stay stable at 20–1000 Hz (`physics_hz` is user-selectable), so determinism across rates becomes a property to be *proved* instead of one that holds by construction; (c) its main visible benefit (side slip of a few degrees, Q-ART-06) is below what the bellows animation shows. *Reopen when* Q-ART-06's condition is met.

**A2 — Augment the kinematic yaw with a force-derived correction** (kinematic base + slip-angle offset on the trailer axle from a lateral-force estimate). *Rejected:* it keeps the kinematic model *and* adds a tyre model that has no data behind it; the correction cannot be validated against anything in the repository, and it complicates the closed-form test of V2.

**A3 — Keep everything as it is (state in the component, mass of the head).** *Rejected:* P2 is a measured error (about a factor of two in mass for the median pair) and P4 blocks session restore of articulated vehicles.

**A4 — Add trailer mass by scaling the head's mass in the loader** (no change in `opts-sim`). *Rejected:* it fixes the sum but not the per-axle loads that brake and drive limits need, and it keeps the consist outside the serialised state. It is, however, a valid *first commit* if step 4 must land before step 1 (noted in the plan).

## 🔎 Discovered during investigation

Added 2026-10-14 after the design review. Nothing here changes the commitment; items 2–4 change what the milestone must verify.

1. **The frame-rate prediction is confirmed.** `crates/opts-game/tests/articulation_frame_rate.rs` (branch `agent/articulation-probe`, commit `eecff08`, `#[ignore]` because it fails by design) drives a bus with one trailer over rolling terrain for 900 fixed steps at 64 Hz, once with 60 and once with 144 rendering frames per second, and compares the three joint variables at every fixed step. Result: `articulation_0_alpha` differs by 0.000000° (peak 13.0°); `articulation_0_beta` by up to 0.46° (peak 29.7°); `articulation_0_gamma` by up to 0.47° (peak 10.7°). A first run over gentle terrain (peak β 0.38°) gave a difference of 0.003°: the size of the divergence follows the terrain, the existence of it does not. The terrain is extreme on purpose (a peak β of 30° is outside real roads); the test establishes *that* β/γ depend on the frame rate, not by how much they do on a road.
2. **Chain length.** The "115 consists at depth 8" in an earlier version of this document were an artefact of the ad-hoc scan (it followed loops and unresolved names up to its own cap). `consist_census` (`agent/consist-mass`, `e4f1949`) gives, for the 383 head files OPTS can see: 333 with one trailer, 15 with two, 4 with three, 2 with four, and a few long consists of trams (6, 7, 14, 22 and 24 units behind the head); 3 heads loop back on themselves (`NF6D_KI.bus`, `Tram_wagen_1.bus`, `Tram_wagen_1_L.bus`). No chain without a loop exceeds the loader's cap of 40. What OMSI does with a loop is unknown (Q-ART-13).
3. **The second line of `[couple_back]`.** `bus.md` called it "the trailer has a further trailer" and marked that resolved; the corpus contradicts it (Q-ART-13), and the loader already treats it as "coupled facing backwards" (`couple_back_reversed`). `bus.md` now carries a warning instead of "resolved".
4. **Indented keywords: hypothesis raised and refuted.** The shared scanner `scan_blocks_with` (`crates/opts-omsi/src/formats/text.rs`) accepts a keyword only at the start of a line, and a keyword alone on an indented line occurs in 7,112 `.sco`, 1,338 `.bus`, 1,128 `.ovh`, 1,354 `.cfg` and 524 `.sli` files. This document first called that a scanner bug that hides 496 `[couple_back]` links. The follow-up (`fix/keyword-indentation`; `docs/src/formats/text/keyword-indentation.md` (branch `fix/keyword-indentation`)) shows otherwise: an experimental scanner that accepted indentation made 371 of the 1,139 object types of Rheinhausen fail to parse (the OMSI editor template carries indented *help blocks*, e.g. `\t[NightMapMode]` followed by prose, then the real keyword), and the indented `[couple_back]` lines of the `Bm238` family name files that exist in 3.6 % of 497 cases (98.8 % at the start of a line): disabled blocks. The scanner was left unchanged. For this design the practical meaning is: the 539 OPTS-visible `[couple_back]` files are the right set; the raw 1,035 count over-states the trailers. Unexplained minorities are Q-ART-14.

Commits referenced by this document: tile version-10 fix `28bbd5a` and mesh-audit merge `3b2d823` (both on `master`); `eecff08` (probe test); `e4f1949` (consist census).

## 📝 Provenance

Read for this document (2026-10-14):

- Code: `crates/opts-sim/src/vehicle/{mod,trailer,articulation,chassis}.rs`; `crates/opts-bevy/src/articulation.rs`; `crates/opts-game/src/in_game/omsi_world/{ground_height,vehicle}.rs`; `crates/opts-game/src/in_game/mod.rs` (system registration); `crates/opts-game/src/app.rs`, `crates/opts-cli/src/args.rs` (`physics_hz`).
- Docs: [articulation.md](articulation.md), [animation-axes.md](animation-axes.md), [bellows-census.md](bellows-census.md) ("Sign Verification (P-1011)"), [vehicle_physics.md](../simulation/vehicle_physics.md), [determinism-replays.md](determinism-replays.md) (stub), [vehicle-scripts-route.md](vehicle-scripts-route.md), [critical review](../meta/critical-review-2026-10-11.md) F-002/F-003, milestones [M-0053](../roadmap/phase-4/M-0053.md), [M-0073](../roadmap/phase-10/M-0073.md), [M-0069](../roadmap/phase-10/M-0069.md), [vision.md](../vision.md).
- **Ad-hoc corpus scan [M], 2026-10-14**, superseded by the committed `consist_census` example (see "Discovered during investigation"; its figures for OPTS-visible files differ because OPTS ignores indented keyword lines, see item 4): a Python pass over the 2,738 `.bus`/`.ovh` files of `Vehicles/` counting `[couple_back]` (1,035 files), `[couple_front]` (21), the chain depth by following `[couple_back]` paths (depth 1: 848; 2: 35; 3: 16; 4: 9; 5–7: 12; 115 reached the scan's depth cap of 8, cause not examined), and the `mass` ratio of trailer to head over the 501 pairs where both files state one (median 0.90, p10 0.60, p90 1.11). Figures differ in scope from the probe in `articulation.md` (1,273 files with a coupling *and* a readable model) and must not be compared with it. No executable was inspected ([ADR-0002](../adr/0002-clean-room-policy.md)).
- The side-slip estimate in Q-ART-06 is arithmetic on stated assumptions (tyre cornering stiffness is an assumption, not read from any source in this project).

Corrections of the task brief found while writing (so that later readers do not inherit them): the brief called the chain assembly a "fixed simulation tick" process and the determinism requirement one for "multiplayer"; yaw is indeed computed in `FixedUpdate` (`ArticulationPlugin`, [articulation.rs:332](../../../crates/opts-bevy/src/articulation.rs)) but pitch/roll are not (P3), and multiplayer is a long-term vision item without a milestone (Q-ART-10). The brief also suggested that the trailer pose could influence which scenery is loaded; nothing in `stream_world_tiles` reads a trailing unit's pose (the observer is the camera), so the open Rheinhausen scenery diagnosis is independent of this design.

## 🔗 Related Documents

- [Articulated Vehicles](articulation.md) — runtime behaviour as implemented (rewritten 2026-10-29, plan step 8)
- [Animation axes](animation-axes.md), [Bellows census](bellows-census.md) — variable meaning and sign argument
- [Vehicle Physics & Kinematics](../simulation/vehicle_physics.md) — the older specification whose hitch formula differs from the implementation
- [Chain assembly](chain-assembly.md) — spawn re-seeding of a chain
- [M-0053](../roadmap/phase-4/M-0053.md), [M-0073](../roadmap/phase-10/M-0073.md), [M-0069](../roadmap/phase-10/M-0069.md), [M-0025](../roadmap/phase-4/M-0025.md)
- [Critical review 2026-10-11](../meta/critical-review-2026-10-11.md) F-002, F-003
