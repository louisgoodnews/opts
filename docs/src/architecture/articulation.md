# Articulated Vehicles (`[couple_back]`, joints, bellows)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Implemented for the player vehicle and train formations (kinematic trailers, shared simulation state, consist mass, fixed-step pitch/roll) and for AI consists (kinematic joint only); `[couple_front]`, parked consists and the signs against OMSI open |
| **Milestone** | [M-0022](../roadmap/phase-4/M-0022.md), addendum 2026-10-11 |
| **Code** | `opts_core::formats::vehicles::bus` (`CouplingLimits`), `opts_sim::vehicle::trailer`, `opts_bevy::articulation`, `opts-game` `omsi_world/vehicle.rs`, `vehicle/trailers.rs`, `vehicle/skins.rs`, `consist_plan.rs` and `traffic_chain.rs` |
| **Probe** | `cargo run -p opts-core --example articulation_vars -- <OMSI root>` |
| **Last Updated** | 2026-10-29 (rewritten for the shared state and checked against the code, plan step 8 of [articulation physics](articulation-physics.md)) |

This chapter covers how OPTS loads, moves and animates multi-segment vehicles: articulated buses, double-articulated buses and multi-car trams. The file keywords are specified in [`bus.md` → Coupling & Articulation](../formats/text/vehicles/bus.md#-coupling--articulation). This chapter covers what happens at runtime.

## What OMSI does

- **Segments are vehicles.** Each segment is a `.bus`/`.ovh` file with its own `[model]`, axles and cameras. The leading unit names the next one with `[couple_back] <file> <flag>`, and the chain continues recursively from there. The flag is **not** "the trailer has a further trailer" as `bus.md` once stated: in the 1,035 installed files with `[couple_back]` it is `false` on 158 links whose target has a `[couple_back]` of its own and `true` on 37 links whose target is the last unit. OPTS reads it as "the trailer is coupled facing backwards" (`BusManifest::couple_back_reversed`; Q-ART-13). Counts: [M] scan of 2026-10-14, see [articulation-physics.md](articulation-physics.md).
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
2. **Loading** (`spawn_trailer_chain` for the player vehicle and for train formations; AI consists are item 7):
   - The files behind the head are planned first by `consist_plan::plan`, pure geometry over manifests that is tested without a session: the head's `[couple_back]` chain, then each car of a train formation (M-0106) with its own chain, all placed along one axis. A unit that is turned around is the local layout mirrored (`turned`), and a car of several files pulls (`powered`) unless told otherwise. The plan takes at most 40 files for a single vehicle (`max_parts`) and `trains::MAX_PARTS` (64) for a formation. A link flagged reversed (the second `[couple_back]` value) ends its unit: the chain leads back into itself, as the NF6D's rear cab does. A loop without that flag is not detected and runs to the cap (Q-ART-13). The longest non-looping chain in the installed corpus has 4 units behind the head.
   - Each segment becomes a **top-level** entity carrying an `ArticulatedSegment`. It holds the leader's rear coupling `(x, y)`, its own front coupling `(x, y)`, the `y` of its rearmost axle (the front coupling `y − 6 m` if it has no axles) and `max_alpha`, which falls back to 52.5° (the most common value).
   - The root vehicle gets an `ArticulatedChain` listing the segments front to back.
   - The segment's meshes take their animations, light maps and hot spots from the **root's** scripts, the OPTS form of `[scriptshare]`.
3. **Kinematics** (`opts_sim::TrailerKinematics`, independent of Bevy; the state is `VehicleSimulation::consist`, stepped by `VehicleSimulation::step` right after the chassis and serialised with it; `VehicleSimulation::teleport` re-seeds it):
   - The segment's front coupling is pinned to the leader's hitch `H`, and its rear axle rolls without side slip, so `dψ/dt = (v_H · r) / L`.
   - `r` is the segment's right-hand unit vector and `L` the distance from the hitch to the rear axle (at least 1 m).
   - The step is split into sub-steps of at most 50 ms.
   - Afterwards `α = wrap(ψ − ψ_leader)` is clamped to `±max_alpha`, and the origin is placed so that the front coupling lies exactly on `H`.
   - Headings are compass angles (clockwise from north) in world `x` east, `y` north.
4. **ECS** (`opts_bevy::articulation_system`, a *reader* of the simulation's consist; it builds the consist from the segments' geometry once if the simulation has none):
   - Runs in `FixedUpdate`, after `vehicle_physics_step_system` and before `vehicle_script_system`.
   - Each segment follows the one in front of it, starting with the root's simulated chassis pose.
   - Each segment's transform is then written in Bevy space (`articulation_render_system`, interpolated between the last two fixed steps), at the height, pitch and roll the simulation computed for it from the ground under its own axles (see item 5; a segment without a ground sample, e.g. AI or parked, stays level at the root's height).
5. **Script variables.**
   - For joint `k` the system writes `articulation_<k>_alpha` in **degrees**, positive when the segment is turned clockwise relative to its leader, so it lags on a right-hand curve.
   - `articulation_<k>_beta` and `articulation_<k>_gamma` (since 2026-10-12) are the **pitch and roll of a unit against its leader in degrees**: each fixed step `opts_game::sample_consist_ground` (TEMPORARY until the M-0025 ground query, Q-ART-09) samples four ground heights per unit (rear axle and front joint, both sides; the head: its four wheel positions) and hands them to `VehicleSimulation::apply_ground`, which lays every unit on the ground, filters height, pitch and roll with the **fixed** time step (`opts_sim::vehicle::ground`) and gives `joint_angles_deg`; `articulation_system` publishes them (β clamped to the joint's limits). They are therefore a function of the fixed step, not of the frame rate (test `articulation_frame_rate`: bit-identical at 30, 60 and 144 fps). The head's visible body motion (`follow_ground`, per frame) is separate from the head pitch used for β. Pitch is nose up positive, roll right side up positive. The signs against OMSI are unverified (Q-ART-02).
   - The variables go into the root's script engine, so the scripts see them in the same fixed step.
6. **Bellows skinning** (`bind_skins`):
   - A mesh whose file stem contains `dum` and that has a driven `[newanim]` becomes a joint entity of its own, animated like any other mesh.
   - A `[smoothskin]` mesh gets a Bevy `SkinnedMesh` whose joint `i` is, in this order: the dummy whose stem contains the bone's name (`Gelenk_A.o3d` for `Gelenk_A`); the dummy of the same trailing letter or digit in the run of numbered dummies (`Bone_C` ↔ `Dum_1_C`); the dummy at number `i` of that run. A *run* is two or more meshes of the model whose file stems differ only in their last character and whose animations are driven by `articulation_*` (`Dum_A`…`Dum_D`, `Gelenk_Dummy_A`…, `przd_a`…) or the fixed names `bone_x`, `dummyx`, `dummy_x`; the skinned mesh numbers its bones along the run it touched last if that run is long enough for its highest `[setbone]` number, else the most recent run that is (damper pairs `…_l`/`…_r` form short runs of their own), else the run it touched last (`pick_run`).
   - The vertices are already in vehicle space, so the inverse bind poses are the identity.
   - A skin whose parts hang below an `[animparent]` pivot (the rear half of the Valiant and Heuliez bellows, `[animparent] tourelle`) moves its bones in that frame: each dummy is repeated as a child of the pivot with the same animation, and the bones that stay on the vehicle stay on the pivot (`joints_in_frame`, 2026-10-28). Bevy ignores the transform of a skinned mesh, so without this the half followed the dummies alone and missed the trailer by half the angle.
   - A bone without a dummy stays on the segment root, with a warning.
   - Skinned meshes get `NoFrustumCulling`, because their bounds move with the bones.
7. **AI consists** (`traffic_chain.rs`, M-0095): the trailing units of a coupled AI vehicle (articulated buses, lorries with trailers) follow the unit ahead through the same kinematic joint (`TrailerKinematics`), pulled behind the lead vehicle that the lane model drives; they despawn with the lead, and the gaps use the chain length. The state is the kinematic joint alone: there is no consist mass or ground following for them, and the code has no writer of `articulation_*` script variables for AI vehicles (`articulation_system` reads the player's consist only), so what an AI bellows shows is ❓ not checked.

### Deviation from `bus.md`

`bus.md` suggests that each segment should be a Bevy **child** of the one in front of it. OPTS deliberately does not do this. A child would inherit the leader's rotation, but a trailer's heading is a state of its own, integrated over time. Segments are therefore top-level entities placed in world space, and the link between them is the `ArticulatedChain` list rather than the transform hierarchy. Despawning goes through the `InGameEntity` marker, as for every in-game entity.

## Tests

| Test | Checks |
| :--- | :--- |
| `opts-core` `coupling_front_character_is_parsed` | All four values and the coupling point are read. |
| `opts-sim` `trailer::tests::*` (5) | Driving straight keeps `α = 0` and the coupling closed. A circle reaches a steady `α` with the expected sign, and the rear axle tracks inside the hitch. Mirrored curves give mirrored angles. The joint limit holds. A standing vehicle does not drift. |
| `opts-bevy` `tests/articulation.rs` | In a headless app the segment stays coupled (gap < 5 cm) while turning, `articulation_0_alpha` is published within the limit and equals the simulation's own joint angle at every step (S-curve); `beta` is 0 without ground samples. Driving straight keeps `α` near 0. Explicit `teleport` re-seeds. |
| `opts-sim` `vehicle::tests`, `trailer::tests`, `ground::tests`, `consist::tests` | Closed form `sin α = L/R_h`, serialisation round trip, teleport, consist mass and friction caps, hitch force, fixed-step ground following (zero on flat, grade difference). |
| `opts-game` `consist_plan::tests` (8) | The head's chain, formation cars, turned units and joints are placed continuously; power follows the unit; a link flagged reversed ends its unit and the next car still follows; the part limit and missing files end the train. |
| `opts-game` `vehicle::bone_tests` (3) | Bone dummies are `bone_<letter>`; a skin in the vehicle frame uses the dummies themselves and one below an `[animparent]` moves its bones in that frame; a skin takes the last run long enough for its bones. |
| `opts-game` `tests/articulation_frame_rate.rs` | α, β, γ per fixed step are identical at 30, 60 and 144 fps; flat ground gives β = γ = 0. |

## Limits and open questions

| ID | Question / limit | Status |
| :--- | :--- | :--- |
| Q-ART-01 | Does OMSI's sign of `articulation_<n>_alpha` match OPTS's (positive = segment clockwise relative to its leader)? This needs a mutation or screenshot comparison of a bellows in a right-hand curve. | ❓ open |
| Q-ART-02 | Is the sign of `articulation_<n>_beta` (`β = leader pitch − pitch`) the one OMSI's bellows meshes expect? Pitch now comes from the terrain under each unit's axles in the fixed step (since M-0053 Part 3); the sign still needs the screenshot check on a slope. | ❓ open |
| Q-ART-03 | What do `positive_/negative_articulation_offset` and `articulation_positive_/negative_offset` mean, and who writes them (script or engine)? | ❓ open |
| Q-ART-04 | Do the outlier `[coupling_front_character]` sets (β limits reversed, truck `kind 0`) mean something different, or are they author errors? OPTS uses only `max_alpha` today. | ❓ open |
| L-ART-01 | Trailers are kinematic, and since M-0053 Part 1 their state is one: `VehicleSimulation::consist` (`ArticulationJoint` is gone). The trailers' mass and axles load the longitudinal dynamics (`UnitLoad`, M-0053 Part 2; GN92: 8 t + 7.2 t instead of 8 t). | 🟢 resolved by M-0053 Parts 1–2 |
| L-ART-02 | `[couple_front]` (push-pull) is not read, and chains beyond 40 files (64 in a formation) are cut (none exists in the corpus without a loop; see Q-ART-13). Reversed units are loaded (`turned`). AI vehicles have consists since M-0095 (item 7); parked vehicles show only their leading unit (no chain code exists for them). | 🔧 follow-up |
| Q-ART-13 | What does the second line of `[couple_back]` mean, and what does OMSI do with a chain that refers back to an earlier unit? The corpus contradicts "has a further trailer" (see above); 13 head files (e.g. the `LTrain`/`MTrain` family, `4020.ovh`, `NF6D_KI.bus`) have a looping chain, and 489 links do not resolve to any `.bus`/`.ovh` file; the coaches of the `Bm238` folder, for example, name `..\Bm238\Bm238.bus`, but the folder holds `Bm238.ovh`. A link flagged reversed ends its unit in OPTS; a loop without the flag builds 40 segments. | ❓ open |
| L-ART-03 | A bone is matched to its dummy by a substring of the file name. Dummies with unrelated names leave the bone static. | ⚠️ heuristic |
| Q-ART-05 | How does OMSI resolve a `[setbone]` name? The MAN NG313 (`model_NG313_main_4.cfg`) names `Armature_Gelenk_A`–`D`, but no mesh file contains those names (its candidates are `NG313_Gelenk*.o3d`, `NL263_Dummy_*.o3d`). The names may refer to bones stored inside the skinned `.o3d`, or to a mesh's `[newanim]` rather than its file name. Found in the 2026-10-11 smoke run. | ❓ open |
