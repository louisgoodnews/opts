# Wheel and steering animation census (P-1011, Phase A)

Tools: `scripts/wheel_census.py` (`[newanim]` census, `--csv`), `crates/opts-omsi/examples/o3d_matrix_census.rs`
(rotation part of the o3d 0x79 matrix), `crates/opts-omsi/examples/anim_axis_probe.rs` (axis hypotheses).

## Wheel spin (`Wheel_Rotation_*`)
Frame rotation (0,0,0) dominates: 3,983 uses with explicit `origin_trans`, 1,317 with `origin_from_mesh`.
Rare: (90,0,0) mesh 76, (-90,0,0) 34, (0,0,-90) trans 8 / mesh 4.

## Steering (`Axle_Steering_*`)
Most common frames: (0,90,0) explicit trans 2,071 (81 folders) and mesh 908 (79); (0,0,90) mesh 694 (10 folders)
and trans 146; then (-28,90,0) 84, (0,100,0)/(0,80,0) 73 each, (0,0,0) mesh 55, (-10,90,0) 46, (0,90,30) 45,
(32.25,-90,0) mesh 42, (90,0,0) 36. Suspension: (0,-90,0) `anim_trans` without origin 5,269 (145 folders).

## Mesh matrix and the NLC bug
NLC wheel meshes carry a y/z-swap 0x79 matrix with `origin_from_mesh` and `origin_rot_z 90`.
Probe over 1,569 `origin_from_mesh` steering/suspension animations, test "rotation axis is vertical":

| hypothesis | vertical |
|---|---|
| H1 frame in vehicle axes, matrix ignored | 902 |
| H2 matrix rotation applied after origin rotation | 611 |
| H3 transposed | 666 |

Classes by matrix: identity 382/382 under all; `(0,0,90)` + swap matrix: 180 only under H2/H3, 128 of `(0,90,0)` + swap
only under H1; 253 `(0,0,90)` + 45-degree style matrix and 42 `(32.25,-90,0)` satisfy none.
**Conclusion: the axis cannot be derived analytically from the matrix; no single rule is validated.** Phase B
needs a visual check per matrix class (which axis the wheel visibly turns about) before any change.

## Trailer wheel variables (done)
`Wheel_Rotation_<n>_L/R` indices per model cfg (935 cfgs with wheel spin): `(0,1)` 710, `(2)` 155, `(0,1,2)` 39,
`(2,3)` 12, `(1)` 8, `(3)` 6, `(0)` 5. Trailer cfgs (`*_trail*.cfg`, e.g. MAN_NL_NG `model_GN92_trail.cfg`,
NG313 `model_NG313_trail.cfg`) use **consist-wide numbering**: the head owns axles 0 and 1, the trailer's axle is 2
(`(2,3)`: two trailer axles; `(1)`/`(3)`: heads or trailers behind one resp. three earlier axles).
All units share the head's script engine (`[scriptshare]`), so the variable must be written under that number.

Fix (commit 6f9b788): `ArticulatedSegment` carries `first_axle` (axles of the head and earlier trailers) and its own
wheels (radius from its `.bus` axles); the articulation step rolls them by the segment's own travel along its model
(negated for reversed segments) and writes `wheel_rotation_<n>_L/R`, `wheel_rotationspeed_<n>_l/r`,
`axle_steering_<n>_*`, `axle_suspension_<n>_*`. No tractor ground speed is used. Test:
`trailer_wheels_roll_with_the_segments_own_travel`.

## Cockpit steering wheel
42 folders carry a steering-wheel mesh (`*lenkrad*`, excluding horn parts). 40 animate it with
`anim_rot Axle_Steering_0_L` (925 uses; factor in degrees per rad of front-wheel steering: 1680 ×505, 1080 ×127,
1300 ×114, 1450 ×113, 1350, 1413, 1200, 1575). Outliers: `lenkwinkel` (9 uses, one folder, a script variable),
`cp_steering_tilt` (column tilt, 2). `Halycon_AI_Pack_2` has unanimated wheels (AI pack). OPTS feeds
`axle_steering_0_l` as `-steer_angle_rad` of the front wheel, so the steering wheel needs no extra variable.

## MAN New Lion's City (`MAN_NewLionsCity`, `model_21C_4door_main.cfg`, mesh `solo\rad_vl_felge.o3d`)
```
[newanim] origin_from_mesh / origin_rot_x 90 / anim_rot Wheel_Rotation_0_L -57.2958   (spin)
[newanim] origin_from_mesh / origin_rot_z 90 / anim_trans Axle_Suspension_0_L 1
[newanim] origin_from_mesh / origin_rot_z 90 / anim_rot Axle_Steering_0_L 57.2958     (steering)
[newanim] origin_from_mesh / origin_rot_y 90 / anim_rot Axle_Steering_0_L 4           (side effect)
```
The mesh's 0x79 matrix is the y/z swap `[1,0,0|0,0,1|0,1,0]`. Compared with MAN_SD200 `model_SD77.cfg` (explicit
`origin_trans`, steering `origin_rot_y 90`, no matrix) the difference is **data**: SD77's frame is in vehicle axes
(rot_y 90 turns X into the vertical), NLC's frame is only vertical if the mesh matrix swap is applied after the
rotation (rot_z 90: X -> forward in vehicle axes = roll, which is the observed bug; with the swap: X -> vertical).
The tiny `factor 4` rot_y 90 entry becomes a camber-like lean under the same rule (a small yaw otherwise).

Probe of main steering (|factor| > 20, `origin_from_mesh`, `anim_axis_probe`): with the swap matrix, `rot (0,0,90)`:
130 of 130 vertical **only** if the matrix rotation is applied (H2); `rot (0,90,0)`: 12 vertical only without it.
Folders with `rot (0,0,90)` + swap main steering: MAN_NewLionsCity (116), C2-Fam_Vol-1 (14), Citybus_S31x, HB76_N4021.
So the rule H2 explains the NLC family and its 3 neighbours; 12 samples contradict it; not enough to change the general
case (separate investigation), but a cheap candidate for the NLC fix, pending the OMSI reference screenshot.

## Still open
- Visual record per matrix class in OPTS (screenshots).
- Bellows sign verification (needs the OMSI reference or geometric check).
