# 🚌 P-1010 — Vehicle Driveability

You are implementing the vehicle driveability layer for OPTS: the
system that turns a static vehicle mesh into a driving vehicle with
its own transmission, engine, brakes, animations, and sounds.

This is the **single most impactful subsystem** for making OPTS a
simulator. Currently, vehicles move because a generic script tells
them to. After this task, vehicles drive because their **actual
OMSI scripts run** — engine RPM, gear selection, door operation,
bellows articulation, wheel rotation, indicator clicks, wiper
sweeps, and every other per-vehicle behaviour are driven by the
vehicle's own `.osc` code, const files, and config.

**This is the primary focus of the project until it is complete.**
Do not sidetrack into other features. Do not accept "close enough."
A bus that doesn't shift gears like its real counterpart is not
done.

This is a MULTI-PHASE task with checkpoints. Do not proceed between
phases without user approval.

---

## 1. Configuration

### OMSI installation path
`/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`

### Test vehicle (Phase A will confirm)
Candidate: `MAN_SD200` — ships with OMSI, well-documented in the
specs, clean `.osc` scripts.

Alternative: the vehicle currently loaded in the running app —
screenshot suggests `CZ_Fam-VoV_1_C2G_4-Door_ZF` — which is a
community vehicle with more complex scripting. Do not start with
this one. Confirm the test vehicle in Phase A.

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Working branch

**This task must be performed on a dedicated feature branch.**
`master` must remain stable and functional throughout.

Create the branch before writing any code:

```bash
git checkout master
git pull                                # if remote exists
git checkout -b feat/vehicle-driveability
```

**Branch name:** `feat/vehicle-driveability`

**Branch rules:**

1. **All work happens on this branch.** Every commit, every file
   change, every experiment.
2. **`master` is untouched.** Do not merge, rebase, or push to
   `master` during this task.
3. **The branch is disposable until Phase H.** If the task fails,
   the branch is deleted and `master` is unaffected.
4. **Merge happens only at Phase H**, after the user has approved
   Phase G's verification.
5. **Merge is `--no-ff`** to preserve the branch history. The
   whole feature lands as one auditable unit.
6. **Squashing is forbidden.** The branch may have dozens of
   commits; each is a step in the migration. Preserve them.

If the branch already exists (e.g., resuming work), verify its
state before continuing:

```bash
git log master..feat/vehicle-driveability --oneline
```

Report the commit count. If the branch has diverged significantly
from `master`, **STOP and report** before continuing. Do not
silently rebase.

### Target crates
- `crates/opts-sim/` — the variable table, script VM, physics
- `crates/opts-bevy/` — animation and sound application
- `crates/opts-core/` — parsers (existing)
- `crates/opts-game/` — wiring

### Output targets
- `crates/opts-sim/src/` — new modules
- `crates/opts-bevy/src/` — new modules
- `docs/src/architecture/vehicle-driveability.md` — the design doc
- `docs/src/architecture/variable-table.md` — the variable system
- `docs/src/architecture/script-vm.md` — the `.osc` interpreter
- `docs/crates/opts-sim.md` — updated
- `docs/crates/opts-bevy.md` — updated
- ADRs as needed

### Related context (read first)
- `docs/src/formats/text/vehicles/bus.md`
- `docs/src/formats/text/vehicles/model_cfg.md` (`[newanim]` section)
- `docs/src/formats/text/vehicles/sound_cfg.md`
- `docs/src/formats/text/vehicles/passengercabin_cfg.md`
- `docs/src/formats/text/vehicles/paths_cfg.md`
- `docs/src/formats/text/vehicles/predefined-variables.md`
- `docs/src/formats/text/vehicles/varnamelist.md`
- `docs/src/formats/text/vehicles/const.md`
- `docs/src/formats/text/vehicles/script-sets.md`
- `docs/src/formats/text/vehicles/osc.md`
- `docs/src/contributing/standards.md`
- `docs/src/adr/0002-clean-room-policy.md`

---

## 2. Guardrails

### ❌ Forbidden

- **Do not commit to `master`.** All work happens on
  `feat/vehicle-driveability`. Any commit that touches `master` is
  a violation of this task's scope.
- **Do not push to remote during the task**, unless the user
  explicitly asks for a backup push. The branch is local until
  merge.
- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not modify OMSI vehicle files.
- Do not commit any OMSI asset to the repository.
- Do not hardcode vehicle-specific behavior. Every behavior comes
  from the vehicle's own scripts and const files.
- Do not implement per-vehicle `if bus == "MAN_SD200" { ... }`
  special cases. If a vehicle needs special handling, the scripts
  provide it.
- Do not skip the variable table architecture. Every subsystem
  reads and writes through it.
- Do not implement sounds as one-shots without a triggering system.
  All sounds are triggered by events or driven by variables.
- Do not implement animations without going through the `[newanim]`
  pipeline. Nothing is animated by ad-hoc code.
- Do not leave any of the four subsystems (scripts, physics,
  animations, sounds) as stubs. All four must work together for the
  task to be complete.
- Do not skip Phase A. The audit determines the actual scope.
- **Do not merge, rebase, or fast-forward `master` at any point
  before Phase H.**

### ✅ Permitted

- Read every file in the OMSI installation (except executables).
- Read the existing specs.
- Extend `opts-sim` with the variable table, script VM, and physics.
- Extend `opts-bevy` with animation and sound application.
- Add dependencies via the workspace (e.g., `mlua` is not needed —
  this is OMSI's own scripting language).
- Add tests, examples, and docs.
- Create a debug overlay showing live variable values.

---

## 3. Preconditions

Verify and report before starting:

1. **Branch is created and checked out.** Verify with
   `git rev-parse --abbrev-ref HEAD`. The output must be
   `feat/vehicle-driveability`. If it is `master` or anything else,
   STOP and create the branch before proceeding.
2. `cargo check` passes
3. A vehicle currently loads and renders (even with generic motion).
4. `model_cfg` parsing includes `[newanim]` blocks (verify by
   reading the parser and the spec).
5. `sound_cfg` parsing exists (verify).
6. `bus.md`, `predefined-variables.md`, `osc.md`, `const.md`,
   `varnamelist.md` exist and are 🟢 or 🟡 Specified.
7. The format specs cover the `.osc` instruction set (if not, that
   is a blocker to report, not a workaround to invent).
8. `opts-sim` exists and is where simulation logic lives.

If any precondition fails, STOP and report. Do not attempt to work
around a missing parser.

**Before writing code, report which vehicle will be the test case
and why. Wait for user confirmation.**

---

## 4. Phase A — Audit

### Stage A1 — Current State

Open `crates/opts-sim/` and `crates/opts-bevy/`. Report:

| Subsystem | Exists? | Partial? | Missing? |
| :--- | :---: | :---: | :---: |
| Variable table | ? | ? | ? |
| Script VM (`.osc`) | ? | ? | ? |
| Vehicle physics | ? | ? | ? |
| Animation pipeline | ? | ? | ? |
| Sound pipeline | ? | ? | ? |
| Const file loader | ? | ? | ? |
| Varlist loader | ? | ? | ? |

For each, cite the file and what's implemented vs. stubbed.

### Stage A2 — Test Vehicle Audit

For the proposed test vehicle (e.g., `MAN_SD200`):

1. List all `.osc` scripts in the vehicle folder.
2. List all `.txt` varlist and const files.
3. Read the main `.osc` script. Report its size and complexity.
4. List all `[newanim]` blocks across the `.cfg` files. Group by
   target: wheels, doors, bellows, gauges, wipers, switches.
5. List all `[sound]` and `[loopsound]` blocks across the sound
   `.cfg` files. Group by type: engine, gearbox, door, indicator,
   retarder, etc.
6. Identify the transmission type (automatic, manual, none) from
   the `.bus` file and the `.cfg` scripts.

### Stage A3 — Coverage Assessment

For the test vehicle, estimate the coverage of the format specs:

- Are all `.osc` instructions used by this vehicle documented?
- Are all `[newanim]` sub-parameters documented?
- Are all `[sound]` parameters documented?
- Are all variables used by this vehicle listed in
  `predefined-variables.md`?

If any coverage is incomplete, list the gaps.

### Stage A4 — Checkpoint

Report the audit. Include a revised estimate of what's actually
required.

**Wait for user approval before Phase B.**

---

## 5. Phase B — Architecture Design

Produce `docs/src/architecture/vehicle-driveability.md` before any
code.

### Stage B1 — The Variable Table

The variable table is the central nervous system. Every subsystem
reads and writes through it.

Design:

1. **Data structure.** A type-erased store. Recommended: an
   `IndexMap<String, Value>` where `Value` is an enum (`Float`,
   `Bool`, `Int`, `String`). Or a `HashMap` with a pre-computed
   index mapping from varlist names to slot IDs. Document the
   choice and the reason.

2. **Scopes.** OMSI uses local variables (`L`), global variables
   (`G`), and predefined variables. Design how scopes are
   represented and accessed.

3. **Input binding.** Keyboard input writes to specific variables
   (`throttle`, `Brake`, `Steer`, etc.). The input system in
   `opts-bevy` should not directly set state — it should write to
   the variable table.

4. **Output binding.** Animation and sound systems read variables
   (`Wheel_Rotation_0_L`, `engine_n`, `door_0`, etc.) and apply
   them.

5. **Persistence.** The variable table persists across frames.
   Which variables reset each frame, which accumulate, and which
   are persistent, is defined by the scripts.

6. **Debugging.** A live variable table viewer is required — an
   overlay that shows the current value of any watched variable.

### Stage B2 — The Script VM

Design the `.osc` interpreter.

1. **Instruction set.** Per the spec. Confirm the spec documents
   the complete instruction set. If not, list the gaps.

2. **Execution model.** OMSI runs scripts every frame (or on a
   timer). Design the execution order:
   - Load all scripts.
   - Each frame: run each script's `frame` (or `(F)` entry).
   - On events: run the corresponding macro.

3. **Stack machine.** Scripts are stack-based bytecode. Design
   the value stack, the operand types, and the arithmetic ops.

4. **Error handling.** A broken script must not crash the sim.
   Log the error, freeze the script's state, continue running.

5. **Performance.** Some vehicles have thousands of instructions
   executing per frame. Design for acceptable performance:
   pre-parse into an instruction vector, use indices not strings,
   cache variable lookups.

6. **Hot reload.** When the user edits a `.osc` file, the script
   should reload without restarting. Design for this.

### Stage B3 — Physics Integration

How the variable table drives physics.

1. Which variables are physics inputs? (`throttle`, `Brake`,
   `Steer`, `engine_n`, `gear`, etc.)
2. Which variables are physics outputs? (scripts read these.)
3. How does the physics step interact with script execution?
   Recommended order per frame:
   - Input → variable table.
   - Script VM runs, updating variables.
   - Physics reads `throttle`, `Brake`, etc. from variables.
   - Physics updates vehicle state (position, velocity).
   - Physics writes back derived values (`Wheel_Rotation_0_L`,
     `engine_n` if driven by physics).
   - Animations and sounds read from variables.

4. **Deterministic time step.** Physics runs on a fixed timestep
   (e.g., 60 Hz). Script VM runs at the same rate.

### Stage B4 — Animation Pipeline

How `[newanim]` blocks become transforms.

1. **Parsing.** Confirm `model_cfg` parsing produces an
   `Animation` struct per block with: target mesh, origin
   transform, `anim_rot` / `anim_trans` variable and factor.

2. **Binding.** Each `[newanim]` binds a mesh to a variable. When
   the variable changes, the mesh's transform updates.

3. **Composition.** Multiple `[newanim]` blocks on the same mesh
   compose in order.

4. **Interpolation.** `[newanim]` has `delay` and `maxspeed`
   parameters. Design how these smooth the animation.

5. **Update order.** Apply animations in `PostUpdate` after the
   script VM has updated variables.

### Stage B5 — Sound Pipeline

How `[sound]` and `[loopsound]` become audio.

1. **Declaration parsing.** Confirm parsing produces a
   `SoundEntry` struct with: file, base volume, 3D position,
   viewpoint scope, volcurves, trigger, conditions.

2. **Triggered sounds.** When a trigger event fires (`[trigger]`),
   play the sound once.

3. **Looping sounds.** A `[loopsound]` plays continuously. Its
   volume is computed from the product of all volcurves. Its pitch
   is `pitch_var / pitch_ref`.

4. **Conditional playback.** `[conditionSingle]` blocks determine
   when a looping sound is active. Multiple conditions are ANDed.

5. **3D positioning.** Bevy spatial audio. Position relative to
   the vehicle.

6. **Viewpoint scoping.** Sounds with `[viewpoint] 5` are only
   audible from outside; `2` only from inside. The audio system
   respects this.

7. **Performance.** A vehicle may have hundreds of sound entries.
   Only trigger / loop those whose conditions are met. Pre-load
   audio assets.

### Stage B6 — File Loading Order

Design the load sequence for a vehicle:

1. Parse `.bus` — get config file paths.
2. Parse `model.cfg` — get `[newanim]` blocks.
3. Parse `paths.cfg` — get attachment points.
4. Parse `passengercabin.cfg` — get camera and passenger data.
5. Parse `sound.cfg` — get sound entries.
6. Load `varlist` files — declare variables.
7. Load `const` files — set constants.
8. Load `.osc` scripts — parse into executable form.
9. Initialize the variable table with defaults.
10. Run any initialization macros the vehicle declares.

### Stage B7 — Checkpoint

Report the design.

**Wait for user approval before Phase C.**

---

## 6. Phase C — Variable Table & Script VM

### Stage C1 — Variable Table

Implement the variable table in `opts-sim/src/variables/`:

```
variables/
├── mod.rs             # Public API
├── value.rs           # Value enum
├── table.rs           # VariableTable resource
├── scope.rs           # Local/global/predefined scopes
├── varlist.rs         # Load from varlist files
└── consts.rs          # Load from const files
```

Acceptance:

- Variables can be looked up by name in O(1).
- Varlist files declare variables with defaults.
- Const files override defaults with vehicle-specific values.
- The table is serializable for debugging.

### Stage C2 — Script Parser

Implement the `.osc` parser in `opts-core/src/formats/vehicles/osc/`:

- Parse the script into an AST / instruction list.
- Handle macros, labels, and calls.
- Handle all value types (int, float, string, list).
- Preserve source line numbers for error messages.

### Stage C3 — Script VM

Implement the interpreter in `opts-sim/src/scripts/`:

```
scripts/
├── mod.rs
├── vm.rs              # The interpreter
├── stack.rs           # Value stack
├── ops.rs             # Instruction implementations
├── context.rs         # Execution context (variable table, time)
├── error.rs           # Runtime errors
└── program.rs         # A loaded script
```

Acceptance:

- All instructions from the spec are implemented.
- Simple test scripts execute correctly (arithmetic, branches,
  loops).
- Complex test scripts (from the test vehicle) execute without
  errors.
- Errors are logged and don't crash the sim.
- A benchmark shows scripts run under 1 ms per frame for a typical
  vehicle.

### Stage C4 — Debug Overlay

Add a Bevy overlay that displays:

- Current value of any watched variable.
- The list of active scripts.
- Errors from the current frame.
- Execution time per script.

This is required for verification and for future debugging.

### Stage C5 — Checkpoint

Report the implementation. Include test results.

**Wait for user approval before Phase D.**

---

## 7. Phase D — Vehicle Physics from Scripts

### Stage D1 — Physics Model

Implement the vehicle physics in `opts-sim/src/physics/`:

```
physics/
├── mod.rs
├── vehicle.rs         # Vehicle body
├── engine.rs          # Engine model (RPM, torque curve)
├── transmission.rs    # Gear selection logic
├── brakes.rs          # Brake system
├── wheels.rs          # Wheel dynamics
├── axles.rs           # Per-axle suspension
└── forces.rs          # Force accumulation
```

The physics model is driven entirely by variables:

- `throttle` (0-1) → engine torque
- `Brake` (0-1) → brake force
- `engine_n` → current RPM (read by scripts, written by physics)
- `gear` → selected gear (written by transmission script, read by
  physics)
- `Wheel_Rotation_0_L/R` → wheel angular position (updated by
  physics)

### Stage D2 — Engine Model

The engine torque curve comes from the const file
(`engine_constfile*.txt`). Parse it. Use it to map RPM → torque.

The transmission ratio, axle ratio, and wheel radius come from the
`.bus` file. Use them to convert engine torque → wheel torque.

### Stage D3 — Transmission Logic

Transmission logic lives in the `.osc` scripts. The physics model
**reads** the current gear from the variable table and applies it.
The scripts decide when to shift.

Verify that:
- Automatic transmissions shift correctly based on
  `engine_throttle_injection` and `engine_n`.
- Manual transmissions respond to gear-selector variables.
- Neutral and reverse work.

### Stage D4 — Brakes

Brake force comes from the const file. The physics model:
- Reads `Brake` from variables.
- Applies brake force to all axles.
- Handles handbrake (`bremse_feststell`).
- Handles retarder (`antrieb_retarder`).

### Stage D5 — Wheels and Axles

Per `[newachse]` in the `.bus` file:
- Position on the vehicle.
- Spring constant, damper.
- Driven or not.

Per `[newanim]` in `model.cfg`:
- Wheel rotation variable (`Wheel_Rotation_0_L`).
- Suspension variable (`Axle_Suspension_0_L`).
- Steering variable (`Axle_Steering_0_L`).

### Stage D6 — Integration with Bevy

The physics model runs in `opts-sim`. `opts-bevy` reads the
resulting state and applies it:
- Vehicle position and rotation.
- Wheel angular positions.
- Suspension offsets.
- Steering angles.

### Stage D7 — Validation

Compare to OMSI on the same vehicle and same route:

| Metric | OMSI | OPTS | Diff |
| :--- | ---: | ---: | ---: |
| 0–50 km/h time | ? | ? | ? |
| Max speed | ? | ? | ? |
| Shift points (RPM) | ? | ? | ? |
| Braking distance from 50 km/h | ? | ? | ? |
| Idle RPM | ? | ? | ? |

Target: within 10% on all metrics.

### Stage D8 — Checkpoint

Report the implementation and validation.

**Wait for user approval before Phase E.**

---

## 8. Phase E — Animations

### Stage E1 — Animation Pipeline

Implement the animation pipeline in
`opts-bevy/src/animation/`:

```
animation/
├── mod.rs
├── binding.rs         # Mesh ↔ variable bindings
├── transform.rs       # Apply rotation/translation
├── smoothing.rs       # Delay and maxspeed
└── systems.rs         # Bevy systems
```

### Stage E2 — Wheel Animations

Per `[newanim]` on wheel meshes:

- `Wheel_Rotation_0_L` → rotation around the wheel's local axis.
- Factor: 57.295779513082320876798154814105 (radians to degrees).
- `Axle_Suspension_0_L` → vertical translation.
- `Axle_Steering_0_L` → rotation around Y axis.

Verify that wheels:
- Rotate at the correct speed.
- Steer left and right.
- Bounce with suspension.

### Stage E3 — Bellows Animations

Per `[newanim]` on the articulation bellows:

- `articulation_0_alpha` → yaw between segments.
- `articulation_0_beta` → pitch between segments.

Verify that bellows:
- Flex when the vehicle turns.
- Compress/extend with suspension motion.

For articulated vehicles, verify that the segments move in concert
and the bellows deform correctly.

### Stage E4 — Door Animations

Per `[newanim]` on door meshes:

- `door_0` → door position (0 = closed, 1 = open).
- `door_1`, `door_2`, ... for subsequent doors.

Verify that:
- Doors open and close smoothly.
- Double doors open symmetrically.
- Door sounds play at the right times.

### Stage E5 — Dashboard and Interior

Per `[newanim]` on cockpit meshes:

- `cockpit_tachowinkel` → tachometer needle angle.
- `cockpit_uhr_std/min/sek` → clock hands.
- `cp_odometer_*` → odometer digits.
- `cp_schluessel_rot` → ignition key rotation.
- Various switch positions.

### Stage E6 — Composition and Order

Verify that:
- Multiple `[newanim]` blocks on the same mesh compose correctly.
- Animation order matches OMSI's behavior.
- Smoothing (`delay`, `maxspeed`) works.

### Stage E7 — Checkpoint

Report the implementation.

**Wait for user approval before Phase F.**

---

## 9. Phase F — Sounds

### Stage F1 — Sound Loading

Implement sound loading in `opts-bevy/src/audio/`:

```
audio/
├── mod.rs
├── loader.rs          # Load .wav files
├── entry.rs           # Parsed sound entries
├── trigger.rs         # Trigger system
├── volcurve.rs        # Volume curve evaluation
├── routing.rs         # 3D positioning and viewpoint
└── systems.rs         # Bevy systems
```

### Stage F2 — Trigger System

Implement the trigger system:

- An event fires → play the sound once.
- Volume and pitch are assigned at trigger time.
- The trigger system subscribes to script events.

Verify that:
- Engine starter sound plays when `ev_engine_starter` fires.
- Door sounds play on `ev_doortriggeropen_*` and
  `ev_doortriggerclose_*`.
- Indicator sounds play on `ev_lights_blinker_on` / `_off`.

### Stage F3 — Looping Sounds

Implement looping sounds:

- Continuously playing while conditions are met.
- Volume = product of all volcurve outputs × base volume.
- Pitch = `pitch_var / pitch_ref`.

Verify that:
- Engine loop plays with correct pitch as RPM changes.
- Turbo whine plays at the right pitch.
- Air brake hiss plays when the compressor runs.

### Stage F4 — Conditions

Implement `[conditionSingle]` evaluation:

- Multiple conditions AND together.
- A looping sound is only active if all conditions are met.

### Stage F5 — 3D Positioning

Implement spatial audio:

- Each sound has a 3D position relative to the vehicle.
- Bevy spatial audio handles attenuation by distance.
- Viewpoint scoping: `[viewpoint] 5` for outside-only, `2` for
  inside-only.

Verify by:
- Walking the camera around the vehicle. Sound should attenuate
  with distance.
- Entering the cockpit. Outside-only sounds should attenuate
  dramatically.
- Opening the driver's window. Sound should change (if the script
  supports it).

### Stage F6 — Engine Sound Synthesis

The engine sound is the most important sound. Verify:

- At idle (`engine_n = ~500`), the engine sound plays at low pitch.
- Under load (`engine_throttle_injection > 0`), the volume
  increases.
- At high RPM, the pitch increases.
- When the engine is off, the sound stops.

### Stage F7 — Checkpoint

Report the implementation.

**Wait for user approval before Phase G.**

---

## 10. Phase G — Verification

### Stage G1 — End-to-End Test

Run the app with the test vehicle. Perform a manual test drive:

- [ ] Engine starts on key turn.
- [ ] Idle sound plays.
- [ ] Shift to drive.
- [ ] Accelerate. Vehicle moves.
- [ ] RPM increases. Engine sound pitch increases.
- [ ] Transmission shifts up. RPM drops. Sound changes.
- [ ] Brake. Vehicle slows. Brake sound plays.
- [ ] Turn. Steering wheel rotates in cockpit. Wheels steer.
- [ ] Open a door. Door animates. Sound plays.
- [ ] Open the driver's window. Window animates.
- [ ] Turn on indicator. Indicator clicks.
- [ ] Turn on wipers. Wipers sweep. Sound plays.
- [ ] Reverse. Vehicle backs up. Reverse sound plays.
- [ ] Park. Handbrake engages. Vehicle stays.

### Stage G2 — Articulated Vehicle Test

Load an articulated vehicle (e.g., `MAN_GN92`). Verify:

- [ ] Both segments render.
- [ ] Bellows deform when turning.
- [ ] Trailers respond to script variables from the shared script
      system.
- [ ] Passenger camera views from both segments work.

### Stage G3 — Comparison to OMSI

Drive the same route in OMSI and in OPTS with the same vehicle.
Compare:

- Acceleration curve (0–50, 0–80 km/h).
- Top speed.
- Shift points.
- Braking distances.
- Sound character (subjective but noticeable).
- Animation fidelity (wheels, doors, bellows).

Document differences and their causes.

### Stage G4 — Automated Tests

Add tests in `crates/opts-sim/tests/` and
`crates/opts-bevy/tests/`:

- Variable table: read/write, scope isolation.
- Script VM: arithmetic, branches, loops, function calls.
- Physics: torque curve, gear ratios, braking.
- Animation: transform composition, smoothing.
- Sound: volcurve evaluation, condition evaluation.

### Stage G5 — Final Report

Produce `docs/src/meta/vehicle-driveability-YYYY-MM-DD.md`:

- Which subsystems were implemented.
- Which vehicles were tested.
- Performance numbers (script VM, physics, audio).
- Fidelity comparison to OMSI.
- Known limitations.
- Remaining work.

---

## 11. Phase H — Merge

Phase H is the only phase in which `master` is touched.

### Stage H1 — Pre-Merge Verification

On `feat/vehicle-driveability`:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

All must pass. If any fails, **do not proceed**. Return to the
relevant phase, fix, and re-verify.

### Stage H2 — Merge Rehearsal

Simulate the merge in a scratch worktree before committing to it:

```bash
git worktree add /tmp/opts-merge-rehearsal master
cd /tmp/opts-merge-rehearsal
git merge feat/vehicle-driveability --no-commit --no-ff
```

Resolve any conflicts. Report what was resolved. If conflicts are
non-trivial, **STOP and report**. Do not blindly resolve.

When the rehearsal succeeds:

```bash
git merge --abort   # if needed
cd -
git worktree remove /tmp/opts-merge-rehearsal
```

### Stage H3 — Final Checkpoint

Report:

- Commit count on the branch
- Files changed
- LOC added / removed
- Test results
- Any conflicts that appeared during rehearsal
- Any deviations from the design

**Wait for explicit user approval before merging.**

### Stage H4 — Merge

Only after the user approves:

```bash
git checkout master
git merge feat/vehicle-driveability --no-ff -m "feat(vehicle): vehicle driveability

Full vehicle simulation: variable table, OMSI script VM, per-vehicle
physics, animation pipeline, and sound pipeline.

<One paragraph summary.>

Tested with: <vehicle name>
Fidelity within 10% of OMSI on: <metrics>

Branch: feat/vehicle-driveability
See docs/src/meta/vehicle-driveability-YYYY-MM-DD.md for the full report."
```

**Do not squash.** The merge commit preserves the branch topology.

### Stage H5 — Post-Merge Verification

On `master`:

```bash
cargo check --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

All must pass. Then run the app and confirm it behaves identically
to the branch.

### Stage H6 — Cleanup

Optional (ask the user):

```bash
git branch -d feat/vehicle-driveability
```

If the user wants the branch preserved for reference, leave it.

### Stage H7 — Failure Path

If Phase H cannot complete — merge conflicts are irreconcilable,
tests fail post-merge, or the user rejects the merge — the branch
is **abandoned**:

```bash
git checkout master
git branch -D feat/vehicle-driveability
```

Report what failed and why. `master` remains unchanged.

---

## 12. Reporting Cadence

1. Preconditions + test vehicle choice — user confirmation
2. Phase A (audit) — approval before Phase B
3. Phase B (design) — approval before Phase C
4. Phase C (variable table + script VM) — approval before Phase D
5. Phase D (physics) — approval before Phase E
6. Phase E (animations) — approval before Phase F
7. Phase F (sounds) — approval before Phase G
8. Phase G (verification) — final report

Format:

```
## P-1010 — Checkpoint N

**Phase:** <name>
**Branch:** feat/vehicle-driveability
**Commits on branch:** <count>
**Master status:** untouched | <violation warning>
**Deliverables:** <summary>
**Test results:** <summary>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

The "Branch" and "Commits on branch" lines confirm that work is
happening in the right place. The "Master status" line is a
self-check: it must always read "untouched" until Phase H.

---

## 13. Exit Criteria

- ✅ All work was performed on `feat/vehicle-driveability`.
- ✅ `master` was untouched until Phase H.
- ✅ Phase H was approved by the user before merging.
- ✅ Merge was `--no-ff` and preserved branch commits.
- ✅ Post-merge verification passed on `master`.
- ✅ Variable table is implemented and shared across all systems.
- ✅ `.osc` script VM executes the test vehicle's scripts.
- ✅ Physics reads from variables, writes back derived state.
- ✅ Transmission, engine, brakes behave per the vehicle's scripts
      and const files.
- ✅ Wheel, bellows, door, dashboard, switch animations work.
- ✅ Engine, gearbox, door, indicator sounds play with correct
      pitch, volume, and 3D position.
- ✅ Articulated vehicle (bellows) tested and working.
- ✅ Fidelity is within 10% of OMSI on measurable metrics.
- ✅ Debug overlay shows live variable values.
- ✅ Automated tests in `opts-sim` and `opts-bevy`.
- ✅ `docs/src/architecture/vehicle-driveability.md` exists.
- ✅ `docs/src/architecture/variable-table.md` exists.
- ✅ `docs/src/architecture/script-vm.md` exists.
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ Every file under 500 lines.
- ✅ Every public item documented.
- ✅ No `println!`.
- ✅ No `unsafe`.
- ✅ No OMSI asset committed.
- ✅ No OMSI binary inspected.

When finished, respond with the summary from Stage G5.

Then respond with the exact string:
"VEHICLE DRIVEABILITY COMPLETE — ready for user review"