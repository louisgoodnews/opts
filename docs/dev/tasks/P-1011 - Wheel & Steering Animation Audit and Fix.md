# 🛞 P-1011 — Wheel & Steering Animation Audit and Fix

You are following up on the bellows census (P-1010a) with a similar
pass on wheel and steering animations. The bellows work exposed a
methodology that works: survey the fleet, categorize the variations,
fix the systematic bugs, verify against real vehicles.

Three user-reported findings motivate this task:

1. **Trailer wheels don't rotate.** On articulated vehicles, the
   tractor's wheels animate correctly, but the trailer's wheels
   stay static regardless of ground speed.
2. **Steering wheel animation is unreliable across the fleet.**
   Some vehicles animate the steering wheel; others don't. The
   variable used (`Axle_Steering_0_L` or a variant) and the axis
   of rotation vary.
3. **MAN New Lion's City front wheels rotate on the X axis when
   steering.** Steering rotation is being applied around the
   forward axis instead of the up axis. The wheels spin when they
   should turn.

These are one bug class. The `[newanim]` rotation-axis resolution
and variable-binding logic needs a systematic audit.

This is a MULTI-PHASE task with checkpoints. The branch policy from
P-1010 applies: create a dedicated branch, work there, merge only
on approval.

---

## 1. Configuration

### Working branch

Create a new branch from the current vehicle work:

```bash
git checkout bevy-0.20
git pull
git checkout -b fix/vehicle-animations
```

**Branch name:** `fix/vehicle-animations`

All work happens on this branch. `bevy-0.20` and `master` stay
untouched until Phase F.

**Merge target:** `bevy-0.20` (not `master`), since this is
continuing vehicle work.

### OMSI installation path
`/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`

### OPTS repository root
The current working directory.

### Output targets

- `docs/src/architecture/wheel-steering-census.md` — the census
- `docs/src/architecture/animation-axes.md` — the axis resolution design
- `crates/opts-bevy/src/animation/` — the fixes
- `crates/opts-sim/src/physics/` — trailer physics propagation
- The final report

### Related context (read first)

- `docs/src/architecture/bellows-census.md` — the methodology
  reference
- `docs/src/formats/text/vehicles/model_cfg.md` — `[newanim]` spec
- `docs/src/formats/text/vehicles/bus.md` — axle declarations
- `docs/src/formats/text/vehicles/paths_cfg.md`
- The bellows fix commit `6f64607` on `bevy-0.20`

---

## 2. Guardrails

### ❌ Forbidden

- Do not commit to `bevy-0.20` or `master` until Phase F.
- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify OMSI vehicle files.
- Do not commit OMSI assets.
- **Do not hardcode per-vehicle special cases.** If a vehicle needs
  different treatment, the difference must come from its `.cfg`
  data, not from `if vehicle_name == "..."`.
- Do not "fix" the trailer wheel bug by faking ground speed. The
  trailer must derive rotation from the actual physics of the
  coupled tractor.
- Do not guess axis conventions. Every axis decision must be
  verified against a test vehicle's visual behavior or a
  documented OMSI convention.
- Do not skip the census. The bellows work succeeded because it
  surveyed first. The same discipline applies here.

### ✅ Permitted

- Read every file in the OMSI installation.
- Read every vehicle's `.cfg`, `.bus`, `.ovh` in the fleet.
- Extend `model_cfg` parsing if axis metadata is being discarded.
- Extend the animation system in `opts-bevy`.
- Extend `opts-sim` physics if trailer coupling is incomplete.
- Add debug overlays showing the resolved axis and value for each
  wheel/steering animation.
- Write census scripts.

---

## 3. Preconditions

Verify and report:

1. Branch is `fix/vehicle-animations`.
2. Bellows commit `6f64607` is present in the branch history.
3. `docs/src/architecture/bellows-census.md` exists.
4. The animation pipeline in `opts-bevy/src/animation/` exists and
   is not a stub.
5. `model_cfg` parsing retains `origin_rot_x`, `origin_rot_y`,
   `origin_rot_z`, `anim_rot`, `anim_trans`, and the associated
   variable names.
6. The physics system in `opts-sim` writes wheel rotation
   variables. (Verify by inspecting the code, not by assumption.)

If any precondition fails, STOP and report.

---

## 4. Phase A — Census

Produce `docs/src/architecture/wheel-steering-census.md`. Follow
the bellows census format.

### Stage A1 — Wheel Rotation Census

For every vehicle in the installation:

1. Parse the `.bus` file and the model `.cfg`.
2. For each `[newanim]` block that drives a wheel-related variable,
   record:
   - Vehicle folder
   - Mesh file
   - Variable name (e.g., `Wheel_Rotation_0_L`,
     `Wheel_Rotation_1_R`, `Wheel_Rotation_2_L`)
   - Associated `origin_rot_*` values
   - Factor
3. Group by:
   - Vehicle class (bus, tram, truck, aircraft, etc.)
   - Axle index (0 for tractor, 1 and 2 for trailer/third axles)
   - Which vehicle segments the variable appears in

**Deliverable:** a table showing, for each vehicle family, which
wheel variables exist and where.

### Stage A2 — Steering Wheel Census

For every vehicle:

1. Find `[newanim]` blocks that drive the steering wheel mesh.
2. Record the variable name. Common candidates:
   - `Axle_Steering_0_L`
   - `Axle_Steering_0_R`
   - `cockpit_lenkrad_winkel`
   - `cp_lenkrad`
   - Any other variable starting with `steering`, `lenkrad`,
     `cp_lenk`
3. Record the axis of rotation:
   - Which `origin_rot_*` precedes it
   - What factor is applied

**Deliverable:** a table showing which variable each vehicle family
uses for the steering wheel, and the axis resolution.

### Stage A3 — Steering Wheel Axis Census

For the front wheels of every vehicle (the ones that steer, not
spin):

1. Find `[newanim]` blocks that drive steering.
2. Record the sequence of transforms:
   - `origin_trans`, `origin_rot_x`, `origin_rot_y`,
     `origin_rot_z`
   - Which one is the steering transform
3. Note any vehicle where the front wheels have unusual axis
   resolution.

**Deliverable:** focus especially on the MAN New Lion's City
family. Record exactly what its `model.cfg` says about the front
wheel steering animation.

### Stage A4 — Trailer Wheel Census

For articulated vehicles:

1. In the trailer segment's `.bus` and `.cfg`, find the wheel
   animation bindings.
2. Confirm which variables they use (`Wheel_Rotation_1_*`?
   `Wheel_Rotation_2_*`? Something else?)
3. Determine where the trailer segment gets its ground speed.
   From physics? From the shared script system?

**Deliverable:** for the 302 articulated vehicles, list the trailer
wheel variables and note any that differ from the majority.

### Stage A5 — Precedent Survey

Before proposing fixes, verify the conventions:

1. Open a vehicle that renders correctly (from the user's
   verification list: NLC 21C, AC O530 G, HB76 NG313, DBC 60LF,
   VA GS NG235, MB C2 Gn, MB O305G, HH AGG300).
2. Record exactly what the front-wheel steering animation looks
   like in `model.cfg`.
3. Record exactly what the wheel spin animation looks like.
4. Compare to MAN New Lion's City.
5. Identify the specific difference that causes the X-axis bug.

### Stage A6 — Checkpoint

Report the census. Include per-family capability tables.

**Wait for user approval before Phase B.**

---

## 5. Phase B — Root Cause Analysis

### Stage B1 — The Axis Resolution Bug

Hypothesis: the `[newanim]` parser or the animation system
resolves the rotation axis incorrectly when `origin_rot_y` is
absent or when multiple `[newanim]` blocks precede the axis-defining
one.

Investigate:

1. Read the `[newanim]` parser.
2. Read the animation system.
3. For the MAN New Lion's City front wheel steering, trace the
   exact sequence of transforms applied to the wheel mesh.
4. Compare to the correctly-rendering NLC 21C.
5. Identify the difference.

**Deliverable:** the exact code path and the exact line where the
axis diverges.

### Stage B2 — The Trailer Wheel Bug

Hypothesis: trailer wheels never receive a rotation value because
either:
- (a) The physics system only writes `Wheel_Rotation_0_*` for the
  tractor.
- (b) The trailer's physics body isn't connected to the tractor.
- (c) The animation binding looks for a variable that doesn't
  exist.

Investigate by adding debug logging:

1. Log every write to `Wheel_Rotation_*` variables.
2. Log every read by the animation system.
3. Compare. If writes are missing for axles 1 and 2, the bug is in
   physics. If reads are missing, the bug is in animation binding.

**Deliverable:** the exact location of the propagation gap.

### Stage B3 — The Steering Wheel Bug

Hypothesis: the steering wheel is animated correctly on some
vehicles but not others because the variable name varies. Some use
`Axle_Steering_0_L`, some use a cockpit-specific variable, some
use both.

Investigate:

1. Take three vehicles that render correctly and three that don't.
2. Compare their `model.cfg` files for the steering wheel mesh.
3. Identify what's different.

**Deliverable:** the pattern that distinguishes working from broken.

### Stage B4 — The "Signs Are a Guess" Follow-Up

The bellows fix assumed "beta = follower pitch − leader pitch" and
"nose-up positive." Verify or refute.

If the user can provide an OMSI screenshot of an articulated bus
on a slope, use it as ground truth. If not, verify through
geometry:

1. Place the same bus on a synthetic slope in OPTS.
2. Screenshot the bellows.
3. Compare with the user's expectation of what OMSI shows.

**Deliverable:** confirmation of the sign convention or a proposed
flip.

### Stage B5 — Checkpoint

Report the root causes.

**Wait for user approval before Phase C.**

---

## 6. Phase C — Design

Produce `docs/src/architecture/animation-axes.md` before fixing
anything.

### Stage C1 — Axis Resolution Rules

Define the rules the animation system applies:

1. How is `anim_rot`'s rotation axis determined?
2. What does `origin_rot_*` contribute?
3. What happens when `origin_rot_*` is absent?
4. How do multiple `[newanim]` blocks compose?

The design must match OMSI's behavior. Where OMSI's behavior is
unknown, state the hypothesis and how it was validated.

### Stage C2 — Trailer Propagation Model

Define how trailer wheel rotation is computed:

1. Trailer physics body is coupled to tractor.
2. Trailer ground speed derives from its own physics, not from the
   tractor's variables.
3. `Wheel_Rotation_1_*` and `Wheel_Rotation_2_*` are computed from
   the trailer's wheel angular velocity.
4. The trailer's `.osc` scripts handle gear and braking logic
   (they don't exist for a trailer — trailers are passive).

### Stage C3 — Steering Wheel Variable Normalization

Define how the animation system handles the variable-name variance
across the fleet:

1. The animation binding reads the variable name from the
   `[newanim]` block verbatim.
2. If the variable doesn't exist in the vehicle's varlist, log a
   `WARN`.
3. Do not normalize names. If a vehicle uses
   `cockpit_lenkrad_winkel`, the vehicle declares that variable in
   its varlist. The system does not guess.

### Stage C4 — Checkpoint

Report the design.

**Wait for user approval before Phase D.**

---

## 7. Phase D — Implementation

### Stage D1 — Axis Resolution Fix

Fix the axis resolution logic in the animation system. This is the
fix for the MAN New Lion's City bug.

### Stage D2 — Trailer Wheel Propagation

Extend the physics system to write trailer wheel rotation
variables. This is the fix for the trailer wheel bug.

Note: this may depend on the physics trailer coupling work flagged
in the bellows report ("physics: the trailers still don't load the
tractor"). If that's a blocking dependency, implement it first.

### Stage D3 — Steering Wheel Fixes

For each vehicle family where the steering wheel doesn't animate,
either:
- Confirm the animation is correct (the census may reveal the
  user's sample was misleading), or
- Fix the binding.

### Stage D4 — Bellows Sign Verification

Apply the outcome of Stage B4. If the sign convention was wrong,
flip it. If correct, document the verification method.

### Stage D5 — Push-Pull `[couple_front]` Loading

The bellows report noted that push-pull units with `[couple_front]`
don't load. This is adjacent to the trailer work and belongs in the
same pass.

### Stage D6 — Beta Limits

The bellows report noted that `min_beta` and `max_beta` from
`[coupling_front_character]` are ignored. Apply them now.

### Stage D7 — Checkpoint

Report the implementation.

**Wait for user approval before Phase E.**

---

## 8. Phase E — Verification

### Stage E1 — Fleet Test

Using the test list from the bellows verification, load and drive
each vehicle. Confirm:

- [ ] Front wheels spin when driving.
- [ ] Front wheels steer when turning. **Check the axis is
      correct, not just that they move.**
- [ ] Rear wheels spin when driving.
- [ ] Trailer wheels spin when driving (articulated vehicles).
- [ ] Trailer wheels steer appropriately (articulated vehicles).
- [ ] Steering wheel animates in the cockpit.
- [ ] Bellows compress and stretch correctly (bellows regression
      check).

The test list: NLC 21C, AC O530 G, HB76 NG313, DBC 60LF,
VA GS NG235, MB C2 Gn, MB O305G, HH AGG300. Add the MAN New
Lion's City to this list specifically for the axis bug.

### Stage E2 — Visual Verification

Take screenshots for each test vehicle showing:
- Wheels during acceleration (spin axis correct)
- Wheels during turning (steer axis correct)
- Cockpit view showing the steering wheel (animated)
- Trailer view showing bellows and wheels

### Stage E3 — Regression Check

Run the bellows test again. Confirm no regression in the previous
work.

### Stage E4 — Automated Tests

Add tests:

- Axis resolution: given a `[newanim]` block, the correct local
  axis is used.
- Trailer wheel propagation: after a physics step, trailer wheel
  variables have non-zero values.
- Sign convention: the bellows respond correctly to positive and
  negative pitch differences.

### Stage E5 — Final Report

Produce `docs/src/meta/vehicle-animations-YYYY-MM-DD.md`:

- Which bugs were fixed.
- Which vehicles were tested.
- Fidelity compared to the previous state.
- Remaining issues.
- The exact test commands the user can run.

---

## 9. Phase F — Merge

Merge `fix/vehicle-animations` into `bevy-0.20` — **not** `master`.

Use `--no-ff` to preserve the branch history.

```
git checkout bevy-0.20
git merge fix/vehicle-animations --no-ff -m "fix(vehicle): wheel and steering animation corrections

Fixes trailer wheel rotation, steering wheel animation across the
fleet, and the MAN New Lion's City steering axis bug.

See docs/src/meta/vehicle-animations-YYYY-MM-DD.md for details."
```

After merge, run `cargo check --workspace`,
`cargo test --workspace`, and `cargo clippy -- -D warnings` on
`bevy-0.20`. All must pass.

Delete the branch if the user approves.

---

## 10. Reporting Cadence

1. Phase A (census) — approval before Phase B
2. Phase B (root cause) — approval before Phase C
3. Phase C (design) — approval before Phase D
4. Phase D (implementation) — approval before Phase E
5. Phase E (verification) — approval before Phase F
6. Phase F (merge) — final report

---

## 11. Exit Criteria

- ✅ All work done on `fix/vehicle-animations`.
- ✅ `bevy-0.20` and `master` untouched until Phase F.
- ✅ Census document exists.
- ✅ Axis resolution document exists.
- ✅ MAN New Lion's City front wheels steer on the correct axis.
- ✅ Trailer wheels rotate when driving.
- ✅ Steering wheel animation works across the tested fleet.
- ✅ Bellows work from `6f64607` is not regressed.
- ✅ `[couple_front]` push-pull units load.
- ✅ `min_beta` and `max_beta` are applied.
- ✅ Bellows sign convention verified or corrected.
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ Merged into `bevy-0.20` via `--no-ff` with user approval.
- ✅ No OMSI asset committed.
- ✅ No OMSI binary inspected.
- ✅ No per-vehicle hardcoded special cases.

---

## 12. What the User Can Do to Help

Two things only the user can provide:

1. **A side-by-side comparison of bellows on a slope.** If the user
   can load the same articulated bus on the same slope in OMSI and
   in OPTS and share screenshots, the beta/gamma sign question is
   resolved definitively.

2. **A confirmation of which vehicles in the test list actually
   animate the steering wheel in OMSI.** The census will reveal
   what the `.cfg` files say, but only the user can confirm
   OMSI's runtime behavior. This distinguishes "the vehicle's data
   says X" from "the vehicle animates as X in OMSI."

Neither is blocking. The agent can proceed with the fixes and
note where user verification would strengthen confidence.

When finished, respond with the summary from Stage E5.

Then respond with the exact string:
"VEHICLE ANIMATION FIXES COMPLETE — ready for user review"