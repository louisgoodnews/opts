# Animation axes of `[newanim]` (P-1011)

Implementation: `crates/opts-bevy/src/vehicle_anim.rs` (the task text names an `animation/` module; the pipeline is this file).

## Conventions in force (one handedness, 2026-10-26)
- OMSI turns in its own left-handed axes (o3d: x right, y up, z forward). In the right-handed vehicle axes used
  here (x right, y forward, z up) every angle therefore takes the **opposite sign**: the origin rotation is
  `Rx(-x)·Ry(-y)·Rz(-z)` and `anim_rot` turns by `-(variable × factor)` about the frame's X axis.
- Frame = `T(origin) · P · R(origin_rot)`, with `P` the mesh matrix (`origin_from_mesh` without `origin_trans`, see
  below) or identity; `anim_trans` moves along X (without `P`, except the NLC's sliding leaves).
- `origin_rot_y 90` turns X **up** (SD77 steering: a positive `Axle_Steering` turns the wheels right), `-90` down
  (suspension: a positive `Axle_Suspension`, OMSI's extension, moves the wheel down).
- `origin_from_mesh` takes the mesh's pivot (translation of the o3d 0x79 matrix).

### Evidence for one handedness
- The scripts' automatic indicator cancel (`lights.osc` of the VW T3 and the TA cars) arms the right indicator on a
  positive `Axle_Steering_0_L`: positive is a right turn. The old right-handed origin rotation needed the opposite
  sign to steer the wheels right.
- The levelling of the brake scripts fills a bellows while `Axle_Suspension` is below -0.105 and drains it above:
  positive is a higher body, the wheel further down; `origin_rot_y -90` must point X down.
- Tilted steering wheels: SD77 (`rot_x -10, rot_y 90`, identity matrix) has the rim normal (0, -0.174, 0.985), the
  column leaning back towards the driver; the old rotation leaned it forward (the rim wobbled by 20 degrees). The
  Valiant 7700 (`rot_x 30.7, rot_z 90`, scaled y/z-swap matrix, normal (0, -0.487, 0.873)) and its needles
  (`rot_x 52, rot_z 90`, dial normal (0, -0.738, 0.675)) match only with the negated angles and the matrix; the old
  axis lay in the wheel's plane (the project lead's "turns on the y axis", "needles turn in the z axis").
- The four sign patches of the old convention all follow from it and are gone: door leaves (`door*` factors),
  the NLC's blind arms, mesh-origin translations on `origin_rot_y ±90` (buttons, the Bremen blind), the swap rule.
- Census (`anim_axis_probe`, 2026-10-26, all installed `.cfg`): 4,600 `origin_from_mesh` steering and spin
  animations. Steering wheels: every one turns about its measured rim normal and clockwise (seen by the driver) on
  a right turn (before: the 49 tilted ones turned about a wrong axis, 89 more, without origin rotation or with
  `rot (32.25,-90,0)`, the wrong way). Road wheels: as before (1,000+
  vertical and turning right). Wheel spins on matrix meshes: 479 of 550 roll forward (112 before).
Tests: `the_tilted_steering_wheel_turns_about_its_column_valiant`, `the_needles_turn_on_the_dial_valiant`,
`the_column_of_the_sd77_leans_towards_the_driver`, `steering_wheels_turn_with_the_road_wheels`,
`a_positive_steering_variable_turns_the_wheels_right` (`vehicle_anim/tests.rs`).

### When the matrix is composed (`matrix_rotation`)
`origin_from_mesh` without `origin_trans`, an `anim_rot`, a non-identity matrix, except: on a y/z-swapped mesh an
axis the origin rotation already makes vertical (12 main steering entries `rot_y 90`, the NLC's factor-4 entry), and
a pure `origin_rot_x` on a proper rotation matrix (4 wheel spins with a yaw matrix). The sections below record how
the two earlier scoped rules were found; they are the special cases of this one.

### Wheel travel in the animation
The scripts read OMSI's travel (about -0.1 m at rest); the models draw the wheels where they stand at rest, so the
animation shows the travel from rest (`suspension_rest`; the rest value of an axle without a body model, the trailer
of an articulated bus, is the head's mean). Whether OMSI draws the body lower by the static deflection instead ❓.

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
reverses it). Since 2026-10-26 checked by the census above (steering wheels turn right on a right turn).

## Outstanding class of issues (not part of P-1011)
The axis rule for `origin_from_mesh` is now two scoped fixes (swap with vertical intent, matrix `P·R` for other
matrices) on top of the vehicle-axes default. A general treatment would need a harness validating all animation samples
against ground truth, and ground truth exists only for flat parts (spin axes, steering wheels, needles). Open
questions: the 12 `rot (0,90,0)` + swap main steering samples, the 116 factor-4 entries, the 42 `(32.25,-90,0)`
steering animations, wheel spins with yaw matrices (9 samples where H3 fits, H4 not), `anim_trans` frames on swapped
meshes (NLC suspension). Separate investigation, not started.

## Vertical translations of mesh-origin animations (2026-10-20)

A translation (`anim_trans`) on an `origin_from_mesh` frame (no `origin_trans`) with a pure `origin_rot_y` of ±90 is negated: the push buttons (`cockpit_gang4` factor 0.002 with -90, `cockpit_tuertaster1` -0.005 with +90: 934 and 3,103 entries in the installed vehicles) press down into the console, and the Bremen MAN's sun blind (`cp_rollo_pos` -1 at load, factor 0.5, drawn by `mouse_y / 500`) is retracted up, where the unflipped convention drew it as a dark panel over the lower windscreen (the project lead's "object obstructing the driver's view"). Rotations (doors, steering, wheels) and explicit `origin_trans` frames are unchanged (checked: the front doors of the Bremen MAN open outward). Reading from usage; not checked against OMSI ❓.

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

## Doors turn the other way (NLC and Bremen MAN)
Top views of the three door types of the NLC (`vis_doortype_* 0/1/2`: interior swing, outward swing, outward-swinging
sliding) and of the Bremen MAN NG313's default interior-swing doors (`vis_doortype_*_int 0`) showed every leaf swinging
the wrong way: the interior-swing leaves went *out* of the bus, the outward-swinging ones *into* it, and the sliding
leaves of the SST hung below the floor. The wheels of the same vehicles steer correctly (a negative
`Axle_Steering_0_L` turns the front wheels right, test `a_negative_steering_variable_turns_the_wheels_right`), so the
rule is limited to variables that start with `door`:

* `anim_rot` factors of `door*` variables are negated (`door0 -177` of the NLC's IST leaf now opens into the bus,
  `door0 -130` of the AST leaf out of it; the Bremen's `door_0 75` into it, `door_0 -130` out);
* the translation of a `door*` variable on a y/z-swapped mesh with `origin_rot_y ±90` (the SST's `door_0_schiebe 0.47`)
  runs in the swapped frame, along the bus, and is negated: the leaves swing out and part to the front and the back.

Checked on screen from the side and from above (closed, half open, open) for all three types. Not checked against OMSI
itself: the arm angles of the SST in between (`door_N_stange`, `door_N_trapez`).
