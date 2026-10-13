# Bellows of Articulated Vehicles: Census and Capabilities

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Census 2026-10-12; binding generalised, `beta`/`gamma` implemented; signs against OMSI unverified |
| **Tool** | `scripts/bellows_census.py "<OMSI>/Vehicles" [--csv out.csv]` |
| **Related** | [Articulated vehicles](articulation.md), [model.cfg `[newanim]`](../formats/text/vehicles/model_cfg.md) |

## What is in the installation

Over all 1,487 `.bus` files (read through their model `.cfg`):

| Kind | Technique | Files |
| :--- | :--- | ---: |
| Articulated (`[couple_back]`) | skinned bellows + animated dummies (`both`) | 302 |
| Articulated | skinned only | 2 |
| Articulated | no bellows animation (trucks, the M5 tram, `NF6D` units: bogies and cables, not bellows) | 50 |
| Single unit | animated dummies only (rigid plates, towbars, kneeling) | 18 |
| Single unit | skinned only (riders on bikes and scooters, steering-column boots) | 30 |

59 vehicle folders animate by `articulation_*`; chains have 2 units in 306 of the head files, 3 in 9 (double-articulated buses), and up to 25 in the rail consists.

### One technique, many names

Almost every bellows is **one or two skinned meshes** (`[smoothskin]` + `[setbone] <bone> <n>`, outer and inner, plus `_LOD`/`_AI` copies) whose bones are driven by **a row of dummy meshes** with `[newanim]`:

```
[mesh] gelenk_v_19\DummyA.o3d
[newanim] origin_from_mesh  anim_rot articulation_0_beta 0.25
[newanim] origin_from_mesh  origin_rot_y -90  anim_rot articulation_0_alpha 0.25
[mesh] …DummyB.o3d … 0.50 … DummyC.o3d … 0.75 … DummyD.o3d … 1.00
```

The factors spread the joint angle over the rings (¼, ½, ¾, 1); yaw (`alpha`) turns about the vertical, pitch (`beta`) about the transverse axis, both at the pivot stored in the dummy's `.o3d` matrix (`origin_from_mesh`). The dummies are named in at least 15 schemes: `przd_a` (528), `przegub_kolo_*` (264+132), `GN_Dum_Gelenk_A` (208), `Bone_A` (104), `DummyA` (76), `Dummy_A` (50), `Gelenk2A` (43), `Artic_Dummy_A` (40), `7700AM_Bone_A` (36), `Gelenk_A` (22), `NL263_Dummy_A` (20), `Dum_Gelenk_A`, `77_Dummy_A`, `Gelenk_Dummy_A`, `Dum_1_A`, `Bremen_LC_Dummy_A`, and the Blender bones `Armature_bone_001`…; the bones they carry are named `Gelenk_A`, `Bone_A`, `Armature_Bone_001`, … **No name rule covers them all.**

### Variables

| Variable | Driven meshes (files) | Folders | OPTS |
| :--- | ---: | ---: | :--- |
| `articulation_0_beta` (pitch) | 2,045 | 59 | **since 2026-10-12** from the ground under the units |
| `articulation_0_alpha` (yaw) | 1,779 | 59 | kinematics (`TrailerKinematics`) |
| `articulation_1_alpha` / `_beta` | 67 / 49 | 3 | second joint of double-articulated buses |
| `articulation_0_gamma` (roll) | 36 | 1 | **since 2026-10-12** |
| `articulation_positive/negative_offset`, `_alpha_abs`, `_alpha_pos/neg`, `_beta_abs/plus/minus`, `articulation_sw` | 36 … 2 | 1–11 | computed by the vehicle's own `articulation.osc` from alpha and beta (nothing to do in OPTS) |

`beta` is as universal as `alpha`: every skinned bellows also pitches. Before 2026-10-12 OPTS kept it at 0, so bellows never compressed over crests and sags.

The scripts add behaviour on top: the anti-jackknife protection (`articulation_knickschutz_warning`, `engine_knickschutz_eingriff` at |α| > 49°, a buzzer and an engine cut), the creak sounds (`articulation_0_alpha_pitch/volume` from the joint's angular speed) and the `articulation_*_offset` values that slide the floor plates. They run with the scripts; the sounds wait for the audio layer (M-0027).

## What OPTS does (player vehicle)

| Capability | State |
| :--- | :--- |
| Units follow the leader kinematically, joint limit `max_alpha` | ✅ |
| `alpha` to the scripts, in degrees | ✅ |
| `beta`, `gamma` from the ground under the units' axles (head: four wheels), smoothed | ✅ new |
| Units lie on the ground (height, pitch, roll) instead of at the head's height | ✅ new (`ArticulatedSegment::ground`) |
| Skinned bellows bound to their dummies | ✅ generalised: by bone name, by trailing letter/digit inside a run of numbered dummies (`Bone_C` ↔ `Dum_1_C`), by the bone's number; runs found from the file names (`articulation_*` animations on stems that differ only in the last character) |
| Rigid-plate bellows without skin (`przegub_*`, `Gelenkplatte`) | ✅ animated like any mesh |
| Chains of 3+ units (`articulation_1_*`), up to 40 segments | ✅ |
| Verified (no unbound bone, trailer attached) | NLC 21C, AC O530 G, HB76 NG313, DBC Citybus 60LF, VA GS NG235, MB C2 Gn, MB O305G (trailer attached, no unbound bone); HH AGG300 (3 units, two trailers); Heuliez GX327 loads without warnings (the solo variant tested has no trailer) |
| Same census over the whole installation (bone has a dummy by the rules) | all bellows; the exceptions are the MVG tow band of `MAN_LC_A21_Zug` (one bone `Armature_003_Bone_001`, one dummy `MVG_LC_Dummy_Zugband`: no name link) |

## Gaps (the capabilities that are *not* there)

1. **Signs of `beta` and `gamma`** are a convention (nose up, right side up positive; `beta` = the follower's pitch minus the leader's). Q-ART-02: compare a bellows over a crest with OMSI.
2. **AI and parked articulated vehicles** have no trailers ([M-0053](../roadmap/phase-4/M-0053.md)); the AI loader uses the full model of the head only.
3. **`min_beta`/`max_beta`** of `[coupling_front_character]` clamp `articulation_<k>_beta` (P-1011; corpus: ±20 on 234 files, ±50 20, ±10 10, ±25 7, ±30 4, ±15 2, (-80,45) 2; three files list them as `20, -20`, so min/max are sorted). Test `beta_is_limited_to_the_coupling_character`.
4. **`[couple_front]` push-pull units:** no vehicle file in the installation uses a `[couple_front]` block (the only three matches, `TH_RS1.ovh`, `BR232_DB-VR.ovh`, `BR232_random.ovh`, contain it in a help comment `[couple_front]/[couple_back]`), so there is nothing to load and no test data; implementing it would be speculation. Open as a follow-up if a third-party vehicle needs it.
4. **Physics** ignores the articulation: the trailers do not load the tractor, brakes and steering are the head's alone.
5. **Sounds** of the joint (creaks, buzzer) are variables without an audio consumer.
6. The ground under a trailer comes from the same height query as the head; where the trailer stands on a different surface (a ramp on one side) roll is only as good as the four samples.

## Method

`scripts/bellows_census.py` parses every `.bus` and its model `.cfg` (case-insensitive paths), collects the meshes with `[smoothskin]`/`[setbone]` and the `[newanim]` drivers that start with `articulation_`, groups numbered dummies and reports the bones that find no dummy. The in-game check starts each family on Hohenkirchen and counts `no dummy mesh for bone` warnings (0 for all verified vehicles; before the generalisation the AGG300, Citybus 60LF and NG313 each logged some, and the census counted 2,871 unbound bones, from the Chicago, Bremen, VA GS and Polish families among others).

## Sign Verification (P-1011)

**Question:** is `articulation_<k>_beta` = follower pitch minus leader pitch (nose up positive) the right sign?

**Argument (numeric, no terrain needed):**
- The bellows dummies are `[newanim]` blocks with `anim_rot articulation_<k>_beta` and a positive factor in the
  large majority (1,312 of ~1,400 beta animations; no origin rotation, so the axis is the lateral X axis). OPTS
  applies `anim_rot` as a right-handed rotation by `-variable·factor` about the frame X axis; this is the same rule
  that makes the wheel top move forward for a growing `Wheel_Rotation` and that gives clockwise `alpha`.
- The end of the bellows joined to the follower (behind the joint) must land where the follower's front lands. A
  follower pitched nose up by `p` (rotation by `+p` about the lateral axis) moves points behind the joint **down**.
  The dummy with `beta = +p` moves them **up** (rotation by `-p`). So with the old definition the bellows bent
  against the body on every slope; the end only matches for `beta = leader_pitch - pitch`.
- Test `bellows_beta_moves_the_follower_end_like_the_follower_body` fails with the old sign (bellows end at
  y +0.278, body at y -0.278) and passes with the new one. The same test for gamma
  (`bellows_gamma_rolls_the_follower_end_like_the_follower_body`, frame `(0,90,90)`) already agrees, so gamma is unchanged.
- At a crest the front unit points down and the rear one up, so the follower is pitched nose up relative to the
  leader: `beta` is now negative there.

**Limits:** this is a geometric argument from OPTS's own `anim_rot` convention, not an OMSI comparison; it holds
if OMSI's bellows meshes were authored against the same convention that the wheels and `alpha` follow. The flat
start road of the Aachen map gives |beta| < 0.5 degrees, so no slope screenshot exists yet; confirm with an OMSI
reference or a slope when one is available.
