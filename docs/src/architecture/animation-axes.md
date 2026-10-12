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
