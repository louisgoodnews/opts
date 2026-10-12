# Animation axes of `[newanim]` (P-1011)

Implementation: `crates/opts-bevy/src/vehicle_anim.rs` (the task text names an `animation/` module; the pipeline is this file).

## Conventions in force
- Frame = `T(origin) · Rx·Ry·Rz(origin_rot)`; `anim_rot` turns about the frame's X axis, `anim_trans` moves along it.
- Explicit `origin_trans` frames are in vehicle axes (x right, y forward, z up): SD77 steering `origin_rot_y 90`
  makes X vertical, suspension `origin_rot_y -90`, wheel spin needs no rotation.
- `origin_from_mesh` takes the mesh's pivot (translation of the o3d 0x79 matrix).

## Problem: the mesh matrix swap (MAN New Lion's City)
NLC wheel meshes (`solo\rad_vl_felge.o3d`) carry the matrix rotation `[1,0,0|0,0,1|0,1,0]` (y/z swap) and
steer with `origin_from_mesh`, `origin_rot_z 90`. In vehicle axes `rot_z 90` turns X into the forward axis, so the
wheel **rolled** when steering. If the mesh matrix follows the origin rotation (frame `T · S · R`), the axis is vertical.

## Evidence
`anim_axis_probe` over 844 `origin_from_mesh` main steering animations (|factor| > 20): the 130 with `rot (0,0,90)`
and the swap matrix are vertical **only** with the matrix applied after the origin rotation (130/130; 0/130 without).
Same class with `anim_rot` of a small factor (NLC's second `origin_rot_y 90`, factor 4) is vertical without the
matrix and becomes a camber-like lean with it.

## The scoped rule
The matrix is composed into a rotation step's frame only when **all** hold:
1. the block uses `origin_from_mesh` (no explicit `origin_trans`);
2. the mesh's matrix swaps y/z;
3. it has `anim_rot` and the origin rotation gives a horizontal axis (|z| < 0.5) that the swap turns vertical (|z| > 0.97).
`anim_trans` steps of the block keep the vehicle-axes frame. A `WARN` is logged once per mesh when the rule fires.
Tests: `explicit_origin_steering_is_vertical_sd77`, `swapped_matrix_mesh_steers_about_the_vertical_nlc`,
`the_matrix_is_left_out_where_the_axis_is_already_vertical` (`vehicle_anim.rs`).

## Steering wheels and other rotated meshes (MAN NG272 GN92, 2nd scoped rule)
`GN92\GN92_lenkrad.o3d`: `[newanim] origin_from_mesh / anim_rot Axle_Steering_0_L 1680`, no origin rotation, matrix rows
`[0,0.985,-0.174 | 0,-0.174,-0.985 | -1,0,0]` (a proper rotation: a 3-cycle with the column's 10 degree tilt). The old frame
turned the wheel about the lateral X axis (rim nodding up and down). SD77's steering wheel has the same kind of
`origin_from_mesh` block with a different matrix; the NLC difference is again the matrix, not the cfg.

**Ground truth** (`anim_axis_geometry`): the rotation axis of a flat ring or disc is its vertex normal (least variance,
after descrambling). For the 578 steering wheels of the fleet (`*lenkrad*`, `origin_from_mesh`, vehicles installed):
the origin rotation alone (H1) matches the normal for 28; composing the matrix as `P = C·Mᵀ·C` (M: rows = the
object's axes in o3d coordinates, C: the y/z swap between o3d and vehicle axes; axis = `P·a`) matches for 575 (H4).
For GN92 the measured rim normal is (0,-0.177,0.984); H4 gives (0,-0.174,0.985) = the tilted column axis. The
y/z swap is the special case `P = S`, so the NLC rule is the same model.

**Rule 2** (`matrix_rotation`): `origin_from_mesh` without `origin_trans`, with `anim_rot`, a non-identity matrix that is
not the pure y/z swap, `origin_rot_x == 0` (all measured counter examples, 4 wheel spins with a yaw matrix, rotate about x)
and the matrix actually moves the axis (|cos| < 0.95). The frame orientation is `P·R`. Test:
`steering_wheel_turns_about_the_tilted_column_ng272` (axis parallel to the measured rim normal, and the nodding axis of
the old code is perpendicular to it).

**Broader than the steering wheel:** the rule is generic, so in GN92 it also fires for 73 meshes (panel switches, gauge
needles, clock hands, the cash desk's change dispensers), logged once each as WARN. No ground truth exists for them (a flat
needle should turn about its normal, but the census cannot separate it from toggles); they follow the same rule because
OMSI's frame for `origin_from_mesh` is the object's own, and the data say so for the only class that can be measured.
Verify gauges and switches against OMSI; restrict the rule if one of them turns the wrong way.

**Direction:** with a rotated or reflected frame the sense of the rotation follows from the composition (a reflection
reverses it). Whether wheels and the steering wheel turn the right way is not verified against OMSI.

## Outstanding class of issues (not part of P-1011)
The axis rule for `origin_from_mesh` is now two scoped fixes (swap with vertical intent, matrix `P·R` for other
matrices) on top of the vehicle-axes default. A general treatment would need a harness validating all animation samples
against ground truth, and ground truth exists only for flat parts (spin axes, steering wheels, needles). Open
questions: the 12 `rot (0,90,0)` + swap main steering samples, the 116 factor-4 entries, the 42 `(32.25,-90,0)`
steering animations, wheel spins with yaw matrices (9 samples where H3 fits, H4 not), `anim_trans` frames on swapped
meshes (NLC suspension). Separate investigation, not started.

## Classes left unchanged
- explicit `origin_trans` frames (the fleet majority, vehicle axes);
- identity-matrix `origin_from_mesh` (382 of 382 already vertical);
- `rot (0,90,0)` on a swapped mesh (12 main-steering samples, vertical only without the matrix) and the factor-4 entries;
- `(32.25,-90,0)` (42), `(31.63,0,90)`, `(27,0,-90)` etc.: vertical under neither rule, unexplained;
- NLC suspension (`anim_trans`, `rot_z 90`) stays in vehicle axes: out of scope of the approved rule (the suspension
  variable is fed as 0, so it has no visible effect today).
Unifying these conventions is a separate investigation.

## Open
The rotation sense with the swap (a reflection) is unverified against OMSI; check with the OMSI NLC reference.
