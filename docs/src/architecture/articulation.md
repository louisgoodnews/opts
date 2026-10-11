# Articulated Vehicles (`[couple_back]`, joints, bellows)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Implemented for the player vehicle (flat, kinematic trailers); pitch/roll, AI vehicles and physics coupling open |
| **Milestone** | [M-0022](../roadmap/phase-4/M-0022.md), addendum 2026-10-11 |
| **Code** | `opts_core::formats::vehicles::bus` (`CouplingLimits`), `opts_sim::vehicle::trailer`, `opts_bevy::articulation`, `opts-game` `omsi_world/vehicle.rs` |
| **Probe** | `cargo run -p opts-core --example articulation_vars -- <OMSI root>` |
| **Last Updated** | 2026-10-11 |

This chapter covers how OPTS loads, moves and animates multi-segment vehicles: articulated buses, double-articulated buses and multi-car trams. The file keywords are specified in [`bus.md` → Coupling & Articulation](../formats/text/vehicles/bus.md#-coupling--articulation). This chapter covers what happens at runtime.

## What OMSI does

- **Segments are vehicles.** Each segment is a `.bus`/`.ovh` file with its own `[model]`, axles and cameras. The leading unit names the next one with `[couple_back] <file> <has further trailer>`, and the chain continues recursively from there.
- **Coupling points.** `[coupling_back] x y z` (on the leader) and `[coupling_front] x y z` (on the follower) are points in each unit's own frame. When the vehicle is assembled, the two points coincide.
- **Joint limits.** The follower's `[coupling_front_character]` holds `max_alpha`, `min_beta`, `max_beta` and `kind`, with angles in degrees. `kind` is `1` for a bus joint and `0` for a truck coupling.
- **Joint variables.** The joint angles reach the models through script variables named `articulation_<n>_alpha`, `_beta` and `_gamma`, where `<n>` is the joint index, starting at 0 for the first joint behind the leader. Models rotate the bellows rings and joint dummies by them with `[newanim] … anim_rot articulation_0_alpha <factor>`. The factors in the corpus are fractions such as `0.25`, `0.5`, `0.75`, `1`, and also `-0.5`, which spreads the angle over the rings of the bellows.
- **Shared variables.** Trailer models read the same variables as the leader. For example, `model_GN71_trail.cfg` reads `articulation_0_alpha`. So a segment's animations evaluate against the vehicle's shared script variables (`[scriptshare]`), not against variables of their own.
- **The bellows.** The bellows is a skinned mesh: `[smoothskin]` plus one `[setbone] <name> <index>` per bone. Each bone is driven by a dummy mesh whose file name contains the bone name, e.g. `GN_Dum_Gelenk_A.o3d` for `Gelenk_A`. Only the dummy carries the `[newanim]`, and its geometry is often empty or protected.

### Corpus statistics (2026-10-11, reference install)

| Measure | Value |
| :--- | :--- |
| `.bus`/`.ovh` files with a coupling and a readable model | 1,273 |
| `[smoothskin]` meshes (lod 0) | 625, all of them with `[setbone]` |
| `[coupling_front_character]` sets | 52.5 / −20 / 20 / 1 (112), 57 / −20 / 20 / 1 (101), 50 / −20 / 20 / 1 (29), 52.5 / −50 / 50 / 1 (20), 54 / −10 / 10 / 1 (10), 15 rarer sets between 38° and 90.5° |
| Outliers | `110 \| 20 \| −20 \| 0` (the only truck coupling; β limits reversed), `80 \| −80 \| 45 \| 1` (2) |
| Animation uses | `articulation_0_beta` 1,874, `articulation_0_alpha` 1,386, `articulation_0_gamma` 36, `articulation_1_alpha` 16, `articulation_1_beta` 14 |
| Related script variables | `positive_/negative_articulation_offset` (112 each), `articulation_positive_/negative_offset` (44 each) |
| Non-sound `.cfg` files reading `articulation_0_alpha` | 203, trailer models included |

Provenance: these figures come from the probe above and a `grep` over the installed `Vehicles/` folder. No executable was inspected ([ADR-0002](../adr/0002-clean-room-policy.md)).

## Pipeline in OPTS

1. **Parsing.** `BusManifest` reads `coupling_front`, `coupling_back`, `couple_back` and `coupling_front_limits: Option<CouplingLimits>`. It accepts a `[coupling_front_character]` only when all four of its values parse.
2. **Loading** (`spawn_trailer_chain`, player vehicle only):
   - The loader follows `[couple_back]` for at most 4 segments.
   - Each segment becomes a **top-level** entity carrying an `ArticulatedSegment`. It holds the leader's rear coupling `(x, y)`, its own front coupling `(x, y)`, the `y` of its rearmost axle (the front coupling `y − 6 m` if it has no axles) and `max_alpha`, which falls back to 52.5° (the most common value).
   - The root vehicle gets an `ArticulatedChain` listing the segments front to back.
   - The segment's meshes take their animations, light maps and hot spots from the **root's** scripts, the OPTS form of `[scriptshare]`.
3. **Kinematics** (`opts_sim::TrailerKinematics`, independent of Bevy):
   - The segment's front coupling is pinned to the leader's hitch `H`, and its rear axle rolls without side slip, so `dψ/dt = (v_H · r) / L`.
   - `r` is the segment's right-hand unit vector and `L` the distance from the hitch to the rear axle (at least 1 m).
   - The step is split into sub-steps of at most 50 ms.
   - Afterwards `α = wrap(ψ − ψ_leader)` is clamped to `±max_alpha`, and the origin is placed so that the front coupling lies exactly on `H`.
   - Headings are compass angles (clockwise from north) in world `x` east, `y` north.
4. **ECS** (`opts_bevy::articulation_system`):
   - Runs in `FixedUpdate`, after `vehicle_physics_step_system` and before `vehicle_script_system`.
   - Each segment follows the one in front of it, starting with the root's simulated chassis pose.
   - Each segment's transform is then written in Bevy space. The chain is flat: every segment takes the root's height.
5. **Script variables.**
   - For joint `k` the system writes `articulation_<k>_alpha` in **degrees**, positive when the segment is turned clockwise relative to its leader, so it lags on a right-hand curve.
   - `articulation_<k>_beta` and `articulation_<k>_gamma` are always `0`.
   - The variables go into the root's script engine, so the scripts see them in the same fixed step.
6. **Bellows skinning** (`bind_skins`):
   - A mesh whose file stem contains `dum` and that has a driven `[newanim]` becomes a joint entity of its own, animated like any other mesh.
   - A `[smoothskin]` mesh gets a Bevy `SkinnedMesh` whose joint `i` is the dummy whose stem contains the name of `[setbone] … i`.
   - The vertices are already in vehicle space, so the inverse bind poses are the identity.
   - A bone without a dummy stays on the segment root, with a warning.
   - Skinned meshes get `NoFrustumCulling`, because their bounds move with the bones.

### Deviation from `bus.md`

`bus.md` suggests that each segment should be a Bevy **child** of the one in front of it. OPTS deliberately does not do this. A child would inherit the leader's rotation, but a trailer's heading is a state of its own, integrated over time. Segments are therefore top-level entities placed in world space, and the link between them is the `ArticulatedChain` list rather than the transform hierarchy. Despawning goes through the `InGameEntity` marker, as for every in-game entity.

## Tests

| Test | Checks |
| :--- | :--- |
| `opts-core` `coupling_front_character_is_parsed` | All four values and the coupling point are read. |
| `opts-sim` `trailer::tests::*` (5) | Driving straight keeps `α = 0` and the coupling closed. A circle reaches a steady `α` with the expected sign, and the rear axle tracks inside the hitch. Mirrored curves give mirrored angles. The joint limit holds. A standing vehicle does not drift. |
| `opts-bevy` `tests/articulation.rs` (2) | In a headless app the segment stays coupled (gap < 5 cm) while turning, `articulation_0_alpha` is published within the limit and `beta` is 0. Driving straight keeps `α` near 0. |

## Limits and open questions

| ID | Question / limit | Status |
| :--- | :--- | :--- |
| Q-ART-01 | Does OMSI's sign of `articulation_<n>_alpha` match OPTS's (positive = segment clockwise relative to its leader)? This needs a mutation or screenshot comparison of a bellows in a right-hand curve. | ❓ open |
| Q-ART-02 | `articulation_0_beta` is used more often than `_alpha`, but OPTS keeps it at 0 because the chain is flat. Pitch follows from terrain under each segment's axles and requires ground-height queries ([M-0025](../roadmap/phase-4/M-0025.md)). | ⏳ waits for M-0025 |
| Q-ART-03 | What do `positive_/negative_articulation_offset` and `articulation_positive_/negative_offset` mean, and who writes them (script or engine)? | ❓ open |
| Q-ART-04 | Do the outlier `[coupling_front_character]` sets (β limits reversed, truck `kind 0`) mean something different, or are they author errors? OPTS uses only `max_alpha` today. | ❓ open |
| L-ART-01 | Trailers are kinematic. The older `opts_sim::vehicle::articulation::ArticulationJoint` inside `VehicleSimulation` does not feed into rendering, and the trailers' mass and axles do not load the physics. Merging the two models is follow-up work. | 🔧 follow-up |
| L-ART-02 | `[couple_front]` (push-pull, reversed units) and chains deeper than 4 segments are not loaded. AI and parked vehicles show only their leading unit. | 🔧 follow-up |
| L-ART-03 | A bone is matched to its dummy by a substring of the file name. Dummies with unrelated names leave the bone static. | ⚠️ heuristic |
| Q-ART-05 | How does OMSI resolve a `[setbone]` name? The MAN NG313 (`model_NG313_main_4.cfg`) names `Armature_Gelenk_A`–`D`, but no mesh file contains those names (its candidates are `NG313_Gelenk*.o3d`, `NL263_Dummy_*.o3d`). The names may refer to bones stored inside the skinned `.o3d`, or to a mesh's `[newanim]` rather than its file name. Found in the 2026-10-11 smoke run. | ❓ open |
