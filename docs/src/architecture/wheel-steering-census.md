# Wheel and steering animation census (P-1011, Phase A)

Tools: `scripts/wheel_census.py` (`[newanim]` census, `--csv`), `crates/opts-core/examples/o3d_matrix_census.rs`
(rotation part of the o3d 0x79 matrix), `crates/opts-core/examples/anim_axis_probe.rs` (axis hypotheses).

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

## Open census items (not done yet)
- Trailer wheels: `feed_predefined` (`vehicle_script.rs`) writes `wheel_rotation_*` from the head's axles only;
  trailer segments' variables are never written. Per-cfg trailer variable census over the 302 articulated vehicles pending.
- Cockpit steering-wheel variables: no steering-angle variable in the top lists (`cp_lenksaeule_*`, `anim_steering_left`,
  `c2_cp_bt_lenkrad_*`, `vdv_lenkstock_*`); targeted census pending.
- Deviation: `crates/opts-bevy/src/animation/` does not exist; the pipeline is `vehicle_anim.rs`.
