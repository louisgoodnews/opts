# 🎥 Camera Perspective System

You are implementing the camera perspective system for OPTS. The
system is driven by F-buttons and provides six distinct camera
modes, each with context-sensitive sub-cameras cycled via mouse
wheel. One mode (free roam) is a free-flying camera with
user-controllable speed.

This system is user-facing and highly tactile. Every mode must feel
right, respond immediately, and degrade gracefully when a mode is
unavailable. Sloppy camera work is the fastest way to make a
simulator feel broken.

This is a MULTI-PHASE task. Design first, implement second, verify
third. Checkpoints between each.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Target crate
`crates/opts-bevy/src/` — where the camera system lives

### Output targets
- `crates/opts-bevy/src/camera/` — the camera module tree
- `crates/opts-game/src/` — CLI/HUD wiring for camera hints
- `docs/src/architecture/camera-system.md` — the design doc
- `docs/crates/opts-bevy.md` — updated with camera API

### Related context (read first)
- `docs/src/formats/text/vehicles/model_cfg.md` — documents
  `[add_camera_driver]`, `[add_camera_pax]`,
  `[add_camera_reflexion]`, `[set_camera_std]`,
  `[set_camera_outside_center]`, `[view_schedule]`,
  `[view_ticketselling]`
- `docs/src/formats/text/vehicles/passengercabin_cfg.md` — for
  passenger-position context
- `docs/src/formats/text/paths_cfg.md` — for pathpoint references
- `docs/src/architecture/coordinates.md` — OMSI ↔ Bevy conversion
- `docs/src/architecture/main-menu.md` — for the menu that stays
  active when no vehicle is current
- The ego-perspective investigation doc (if it exists at
  `docs/src/architecture/ego-perspective.md`)
- `docs/src/contributing/standards.md`

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not hardcode camera transforms. Every camera position and
  rotation comes from a parsed `model_cfg` or `passengercabin_cfg`
  entry, or from the free-roam camera's user input.
- Do not break the existing HUD camera hint (`[F1/F2] Cam`).
  Extend it.
- Do not use `println!` for camera diagnostics. Use `tracing` at
  `DEBUG` level.
- Do not add new format parsers. Use the existing ones.
- Do not animate cameras into position with springs or lerps that
  interfere with user input. Transitions must be near-instant or
  user-interruptible.

### ✅ Permitted

- Read every file in the repository.
- Add Bevy resources, components, and systems to `opts-bevy`.
- Add camera input handling.
- Add a HUD hint that lists camera keybinds.
- Add a settings field for mouse sensitivity (used for free-roam
  look).
- Add a debug overlay (toggleable) that shows the active camera
  mode and sub-camera index.

---

## 3. Preconditions

Verify and report before starting:

1. `opts-bevy` exists with a Bevy plugin entry point.
2. A vehicle can be spawned into the scene (from the previous task).
3. `model_cfg.md` documents the camera keywords listed above.
4. `passengercabin_cfg.md` exists.
5. `coordinates.md` exists.
6. The application runs (`cargo run -p opts-game`).

If a precondition fails, STOP and report. Do not attempt to work
around missing camera keyword parsing — if `model_cfg` doesn't
parse `[add_camera_driver]` yet, that must be fixed first.

---

## 4. Camera Mode Specification

The following modes are required. Each mode has a **mode key**, a
**target** (what the camera is attached to), and a **sub-camera
list** (what the mouse wheel cycles through).

### F2 — Ego Perspective

- **Target:** the player's human character (if loaded).
- **Sub-cameras:** one — the head bone attachment point.
- **Fallback:** if no human is loaded, F2 does nothing and logs a
  `DEBUG` message. Do not silently switch to another mode.
- **Source:** the human's head bone from the `.hum` file (see the
  ego-perspective investigation).

> If the human system is not yet implemented, mark F2 as
> "unavailable" in the HUD and log a `WARN` on first press. Do not
> build a placeholder human. The mode is either real or absent.

### F3 — Cockpit

- **Target:** the current user vehicle.
- **Sub-cameras:** all `[add_camera_driver]` entries in the
  vehicle's `model_cfg`, in declaration order.
- **Default sub-camera:** the index from `[set_camera_std]` (if
  present), otherwise 0.
- **Special entries:** `[view_schedule]` marks a sub-camera as the
  timetable view. `[view_ticketselling]` marks the ticket-selling
  view. These are accessible via mouse wheel like any other.
- **Fallback:** if there is no current user vehicle, F3 logs a
  `DEBUG` message and does nothing.

### F4 — External

- **Target:** the current user vehicle, or the player human if no
  vehicle is current.
- **Sub-cameras:** if both a vehicle and a human exist, mouse wheel
  cycles between vehicle-external and human-external. If only one
  exists, there is a single sub-camera.
- **Orbit center:** for vehicle, use `[set_camera_outside_center]`
  if present, otherwise the vehicle origin. For human, use the
  head bone.
- **Orbit behavior:** left mouse drag orbits around the center.
- **Pan behavior:** **middle mouse button held + mouse movement
  pans the camera.** Panning translates the orbit center in the
  camera's local right/up plane. The pan rate scales with the
  current zoom level so that panning feels consistent at both close
  and far distances (e.g., `pan_rate = zoom * 0.005` metres per
  pixel of mouse movement).
- **Zoom behavior:** mouse wheel scroll (no modifier) zooms in/out
  within a bounded range (0.5 m to 50 m).
- **Sub-camera cycling:** when both vehicle and human targets
  exist, mouse wheel scroll cycles between them. When only one
  exists, mouse wheel scroll zooms.

> **Design decision to confirm in Part A:** whether middle mouse
> button conflicts with any existing bindings. If a conflict
> exists, remap pan to a different input and document the reason.

> **Design decision to confirm in Part A:** whether panning should
> translate the orbit center (so subsequent orbits revolve around
> the panned point) or translate the camera (so the orbit center
> stays fixed). Recommendation: **translate the orbit center**,
> because it feels more natural for inspection and matches the
> convention in 3D modeling tools. Document the alternative if
> chosen.

### F5 — Passenger Views

- **Target:** the current user vehicle.
- **Sub-cameras:** all `[add_camera_pax]` entries in the vehicle's
  `model_cfg`, in declaration order.
- **Articulated vehicles:** for a multi-segment vehicle, F5 must
  present passenger cameras from **all** segments, ordered by
  segment, then by declaration order within each segment.
- **Cycle behavior:** mouse wheel cycles through the flattened
  list. The HUD shows `F5 [2/7]` style indicator.
- **Fallback:** if no vehicle is current, F5 logs `DEBUG` and does
  nothing.

### F6 — Free Roam

- **Target:** none. Detached camera.
- **Sub-cameras:** one.
- **Movement:** WASD for horizontal movement relative to camera
  direction. Optional Q/E for vertical (or space/ctrl). Movement
  is camera-relative, not world-axis-aligned.
- **Look:** mouse look, either free (always active) or via
  right-drag. Recommend: always active in F6, no drag needed,
  because F6 is a debugging/inspection mode.
- **Speed control:**
  - Base speed: **5 m/s** minimum.
  - `Shift + Mouse Wheel Up`: increase speed by a multiplier (see
    below).
  - `Shift + Mouse Wheel Down`: decrease speed.
  - **No maximum speed cap.**
  - **Minimum speed is 5 m/s.** If the user tries to decrease
    below 5 m/s, clamp to 5 m/s.
- **Speed multiplier:** each wheel notch multiplies the current
  speed by a fixed factor (recommendation: **1.25×** for increase,
  **0.8×** for decrease, keeping the two reciprocal). This gives a
  smooth geometric ramp that feels natural at both walking and
  flying scales.
- **Speed persistence:** the speed set in F6 persists across
  camera mode switches. If the user goes F6 → F3 → F6, the speed
  is retained. Reset only on application restart.
- **Speed HUD:** while in F6, the HUD shows current speed (e.g.,
  `F6  22.5 m/s`).

### F1 — Previous Camera

- **Target:** the previously active non-F1 camera mode.
- **Purpose:** quick toggle. OMSI uses F1 for this pattern.
- **Behavior:** press F1 once to return to the previous mode; press
  F1 twice to toggle back. Behaves like Alt+Tab for cameras.

### Mouse Wheel and Mouse Button — Global Behavior

| Context | Wheel Scroll | Shift + Wheel | Middle Button + Move |
| :--- | :--- | :--- | :--- |
| F2 (ego) | no-op | no-op | no-op |
| F3 (cockpit) | cycle driver cameras | no-op | no-op |
| F4 (external) | zoom (single target) or cycle (dual target) | no-op | **pan** |
| F5 (pax) | cycle pax cameras | no-op | no-op |
| F6 (free roam) | no-op | adjust speed | no-op |

> **Design decision to confirm in Part A:** whether F4 wheel
> should zoom or cycle when both a vehicle and a human exist.
> Recommendation: **cycle** when both exist (user can switch
> targets quickly), **zoom** when only one exists (no target to
> switch to). This means the behavior is context-sensitive, which
> may feel inconsistent. An alternative is to always zoom, and use
> Shift + Wheel to cycle targets. Document the choice.

### Shift + Space — Reset Current View

In **every** camera mode (F2 through F6), pressing **Shift + Space**
resets the current view to its default state.

The definition of "default" is per-mode:

| Mode | Reset behavior |
| :--- | :--- |
| **F2 (Ego)** | Reset head bone attachment offset to (0, 0, 0). Reset look direction to head bone's forward vector. |
| **F3 (Cockpit)** | Reset to the `[set_camera_std]` index if present, otherwise index 0. Reset any per-view offsets (view pitch, view yaw adjustments) to zero. |
| **F4 (External)** | Reset orbit yaw to 0, pitch to the default elevation (recommend `-15°`), zoom to 8 m, and pan offset to zero. Reset target to vehicle if both targets exist. |
| **F5 (Passenger)** | Reset sub-camera index to 0. Reset any per-view offsets to zero. |
| **F6 (Free Roam)** | Reset yaw and pitch to the world-forward orientation. Reset speed to **5 m/s**. Reset position to the last spawn point (see below). |

**F6 position reset:** Free-roam has no natural "home" position.
The recommendation is to reset to the position the camera occupied
when the user **most recently entered** F6 mode. This makes the
reset act like an "undo" for exploration. If a more conventional
behavior is preferred, reset to the world origin — but document
the choice.

**When the reset fires:**

- The reset is immediate (no transition).
- The reset does not change the active camera mode.
- The reset does not affect the `previous_mode` field (so F1 after
  a reset still returns to whatever mode the user came from).

**When the reset does not fire:**

- If the current mode's default state is already active, the reset
  still fires and logs a `DEBUG` event. This makes the behavior
  predictable — the key always does the same thing.
- If no camera mode is active (during `Loading` state), the key
  press is ignored.

> **Design decision to confirm in Part A:** whether F4 wheel
> should zoom or cycle. Also: whether F3/F5 should wrap around at
> the ends or stop. Recommendation: **wrap** for F3 and F5, since
> they're small lists. Do not wrap F4's sub-camera count, since it
> has at most 2.

### HUD Camera Hint

The existing HUD hint (`[F1/F2] Cam`) must be extended to show all
modes:

```
[F1] Prev  [F2] Ego  [F3] Cockpit  [F4] External  [F5] Pax  [F6] Free
```

Unavailable modes render dimmed (e.g., F2 without a human, F3/F5
without a vehicle). The currently active mode is highlighted.

When the active mode has sub-cameras, append an index:

```
[F5] Pax [2/7]
```

The HUD hint must include the middle-mouse pan hint when F4 is
active, and the Shift + Space reset hint when any camera mode is
active:

```
[F1] Prev  [F2] Ego  [F3] Cockpit  [F4] External  [F5] Pax  [F6] Free
[MMB] Pan  [Shift+Space] Reset
```

The `[MMB] Pan` segment is only shown in F4. The
`[Shift+Space] Reset` segment is shown whenever a camera mode is
active.

---

## 5. Part A — Design

Produce `docs/src/architecture/camera-system.md` before writing any
code.

### Stage A1 — Data Structures

Define the types:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CameraMode {
    Ego,        // F2
    Cockpit,    // F3
    External,   // F4
    Passenger,  // F5
    FreeRoam,   // F6
}

#[derive(Resource)]
pub struct CameraState {
    pub mode: CameraMode,
    pub previous_mode: CameraMode,
    pub sub_camera_index: usize,
    pub free_roam_speed: f32,       // m/s, default 5.0
    pub external_zoom: f32,          // metres, default 8.0
    pub external_yaw: f32,
    pub external_pitch: f32,
}
```

The `CameraState` struct must additionally track:

```rust
pub struct CameraState {
    // ... existing fields
    pub pan_offset: Vec3,           // metres, offset from orbit center
    pub free_roam_spawn: Vec3,      // position when F6 was last entered
    pub free_roam_spawn_yaw: f32,   // orientation when F6 was last entered
    pub free_roam_spawn_pitch: f32,
}
```

`pan_offset` is reset to `Vec3::ZERO` on:
- Mode change away from F4.
- Shift + Space reset in F4.
- Target switch (vehicle ↔ human).

`free_roam_spawn` and its yaw/pitch are written each time the
user **enters** F6 mode. They are read by the Shift + Space reset
in F6. They are not modified while F6 is active.

Define the per-mode sub-camera sources:

- **Ego:** `HeadBoneOf(player_human)`
- **Cockpit:** `Vec<DriverCamera>` — populated on vehicle spawn
- **External:** `Vec<ExternalTarget>` — vehicle, human (whichever
  exist)
- **Passenger:** `Vec<PassengerCamera>` — flattened across segments
- **FreeRoam:** a single `Transform` tracked by the camera itself

Define how these lists get populated:

- `DriverCamera` and `PassengerCamera` come from `model_cfg`
  parsing. State how the parser exposes them (whether existing
  `ModelConfig` has a `cameras` field, and if not, what needs to be
  added).
- Articulated vehicles: how the segment list is flattened.
- Segment transform composition: how a passenger camera in the
  trailer is converted into world space when the tractor is at a
  given transform.

### Stage A2 — Systems

Define the Bevy systems:

| System | Runs in | Purpose |
| :--- | :--- | :--- |
| `handle_camera_key_input` | `Update` | F1–F6 switching |
| `handle_camera_reset_input` | `Update` | Shift + Space reset |
| `handle_camera_wheel_input` | `Update` | Sub-camera cycling, zoom, speed |
| `handle_camera_pan_input` | `Update` | Middle mouse pan (F4) |
| `update_ego_camera` | `PostUpdate` | Attach to human head bone |
| `update_cockpit_camera` | `PostUpdate` | Attach to vehicle driver camera |
| `update_external_camera` | `PostUpdate` | Orbit + pan around target |
| `update_passenger_camera` | `PostUpdate` | Attach to pax camera in segment |
| `update_free_roam_camera` | `Update` | WASD + look |
| `update_camera_hint` | `PostUpdate` | HUD text |

Specify run conditions — e.g., `update_free_roam_camera` only runs
when `CameraState.mode == FreeRoam`.

### Stage A3 — Input Bindings

| Key | Action |
| :--- | :--- |
| `F1` | Switch to previous camera mode |
| `F2` | Ego perspective |
| `F3` | Cockpit |
| `F4` | External |
| `F5` | Passenger |
| `F6` | Free roam |
| `W` / `A` / `S` / `D` | Free-roam movement |
| `Q` / `E` | Free-roam vertical (if used) |
| `Shift` (held) | Modifier for wheel speed in F6 |
| `Shift + Space` | **Reset current view to default** |
| `Mouse Wheel` (scroll) | Per-mode (see table) |
| `Left Mouse Drag` | Orbit (F4) / look (F6) |
| `Middle Mouse Button + Move` | **Pan (F4)** |

### Stage A4 — Fallback Matrix

Specify what happens when each mode is unavailable:

| Mode | Required | If unavailable |
| :--- | :--- | :--- |
| F2 Ego | A player human | Stay in current mode, log `DEBUG`, dim HUD entry |
| F3 Cockpit | A user vehicle | Stay in current mode, log `DEBUG`, dim HUD entry |
| F4 External | Any target | Never unavailable — falls back to free-roam if needed |
| F5 Passenger | A user vehicle with pax cameras | Stay in current mode, log `DEBUG`, dim HUD entry |
| F6 Free Roam | Nothing | Never unavailable |

### Stage A5 — Checkpoint

Report the design.

**Wait for approval before Part B.**

---

## 6. Part B — Implementation

### Stage B1 — Module Tree

```
crates/opts-bevy/src/camera/
├── mod.rs               # Public API + plugin registration
├── plugin.rs            # CameraPlugin
├── state.rs             # CameraState, CameraMode
├── input.rs             # Key and wheel handlers
├── mode/
│   ├── mod.rs
│   ├── ego.rs
│   ├── cockpit.rs
│   ├── external.rs
│   ├── passenger.rs
│   └── free_roam.rs
├── sub_cameras.rs       # Sub-camera list construction
├── transform.rs         # OMSI → Bevy transform helpers
└── hud.rs               # Camera hint rendering
```

### Stage B2 — Model Config Camera Extraction

Verify (and extend if needed) that the `model_cfg` parser exposes:

- `driver_cameras: Vec<DriverCamera>`
- `passenger_cameras: Vec<PassengerCamera>`
- `default_driver_index: Option<usize>` (from `[set_camera_std]`)
- `outside_center: Option<Vec3>` (from
  `[set_camera_outside_center]`)

If any of these are missing, add them to the parser with the
appropriate unit tests. Do not stub.

Each camera entry must preserve:
- Its parameter values (`x, y, z, dist, fov, yaw, pitch` per the
  `model_cfg` spec)
- Whether it's marked `[view_schedule]` or `[view_ticketselling]`

### Stage B3 — Camera State Initialization

On application start:

- `CameraState.mode = FreeRoam` (safe default — no vehicle yet)
- `CameraState.sub_camera_index = 0`
- `CameraState.free_roam_speed = 5.0`
- `CameraState.external_zoom = 8.0`

When a vehicle is spawned and becomes the current user vehicle:

- Populate cockpit and passenger camera lists.
- If currently in `External` mode, keep it.
- Do **not** auto-switch to cockpit. The user decides.

### Stage B4 — Key Handling

Implement `handle_camera_key_input`:

- Watch for `KeyCode::F1` through `F6`.
- On press, check availability per the fallback matrix.
- If available: `previous_mode = mode; mode = new_mode; sub_camera_index = 0`
  (or `default_driver_index` for cockpit, if `[set_camera_std]` is
  present).
- If unavailable: log `DEBUG`, no state change.
- Do not process the same key press on multiple frames.

### Stage B5 — Wheel Handling

### Stage B5a — Middle Mouse Pan Handling

Implement `handle_camera_pan_input`:

- Only active when `CameraState.mode == CameraMode::External`.
- Detect `MouseButton::Middle` press and release.
- While held, read accumulated mouse movement deltas from the
  frame's `MouseMotion` events.
- Compute pan offset:
  ```
  let pan_speed = self.external_zoom * 0.005;
  let right = camera_forward.cross(Vec3::Y).normalize();
  let up = right.cross(camera_forward).normalize();
  let pan_delta = right * (-mouse_dx * pan_speed)
                + up * (mouse_dy * pan_speed);
  self.pan_offset += pan_delta;
  ```
- The pan offset is added to the orbit center before computing the
  camera position in `update_external_camera`.

Panning does not modify yaw, pitch, or zoom.

### Stage B5b — Reset Handling

Implement `handle_camera_reset_input`:

- Detect `Shift + Space` (both keys held at the same time in the
  same frame).
- Fire on key press edge, not continuous while held.
- Delegate to a per-mode reset function:
  ```rust
  fn reset_current_view(state: &mut CameraState, mode_specific: &mut ...) {
      match state.mode {
          CameraMode::Ego => reset_ego(...),
          CameraMode::Cockpit => reset_cockpit(state, ...),
          CameraMode::External => reset_external(state, ...),
          CameraMode::Passenger => reset_passenger(state, ...),
          CameraMode::FreeRoam => reset_free_roam(state, ...),
      }
  }
  ```
- Each reset function applies the per-mode defaults from the
  Shift + Space specification table.
- Log the reset via `tracing::debug!` with the mode and the
  previous state.

### Stage B6 — Cockpit Camera Update

`update_cockpit_camera`:

1. Get the vehicle root transform.
2. Get the active `DriverCamera` entry from the vehicle's
   `ModelConfig`.
3. Compute the camera transform:
   - Start with the driver camera's local `(x, y, z, dist)`.
   - Apply `yaw`, `pitch` as rotation offsets.
   - Set `fov` on the projection.
   - Compose with the vehicle root transform.
4. Apply to the `Camera3d` component.

The camera's `dist` is the eye distance from the pivot. Handle the
sign convention documented in `model_cfg.md`.

### Stage B7 — Passenger Camera Update

`update_passenger_camera`:

1. Determine the current segment from `sub_camera_index`
   (articulated vehicles flatten segment + local index).
2. Get the segment's world transform (composed from the chain).
3. Get the local `PassengerCamera` entry.
4. Compute the camera transform as in B6.

For articulated vehicles, segment transforms come from the
`ChainAssembly` resource (from `opts-sim`).

### Stage B8 — External Camera Update

`update_external_camera`:

1. Determine the target (vehicle or human).
2. Determine the orbit center:
   - Vehicle: `outside_center` from model_cfg, or vehicle origin.
   - Human: head bone world position.
3. Compute camera position: `center + (yaw, pitch) * zoom`.
4. Camera looks at center.
5. Mouse drag updates `yaw`, `pitch`.
6. Wheel updates `zoom` (clamped 0.5 – 50 m).

### Stage B9 — Free Roam Camera Update

`update_free_roam_camera`:

1. Read WASD input.
2. Compute direction vector from camera yaw/pitch.
3. Move camera by `direction * speed * delta_time`.
4. Mouse look updates yaw/pitch (always active).
5. Wheel with Shift adjusts speed:
   - Up: `speed *= 1.25`
   - Down: `speed /= 1.25`
   - Clamp minimum to `5.0`
   - No maximum clamp.
6. Update HUD speed display.

Speed change should be smooth when done rapidly. If the user
spins the wheel, the multiplier compounds. This is intentional.

### Stage B10 — Ego Camera

If the human system exists:

`update_ego_camera`:

1. Get the player human entity.
2. Get its head bone world transform.
3. Set camera transform to head bone with a small forward offset.
4. Handle first-person view (hide the human mesh's head? or full
   body? Confirm in Part A).

If the human system does not exist:

- F2 must be marked unavailable in the HUD.
- `update_ego_camera` returns early.

Do not build a placeholder human.

### Stage B11 — HUD

Update the camera hint component:

- Shows all six modes.
- Highlights the active one.
- Dims unavailable ones.
- Appends `[index/total]` when a mode has sub-cameras.
- In free roam, appends the current speed.

### Stage B12 — Verify

Run:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p opts-bevy --all-features
```

All must pass.

### Stage B13 — Checkpoint

Report the implementation and runtime verification.

**Wait for approval before Part C.**

---

## 7. Part C — Verification

### Stage C1 — Manual Verification

Run the app with a real map and a real vehicle. Verify:

- [ ] F4 works with no vehicle (human external, if a human exists;
      otherwise logs a warning)
- [ ] F4 middle-mouse drag pans the camera
- [ ] F4 pan rate scales with zoom (close = slow, far = fast)
- [ ] F4 pan does not change yaw, pitch, or zoom
- [ ] F4 left-drag orbit still works after panning
- [ ] Shift + Space resets F2 to default
- [ ] Shift + Space resets F3 to default sub-camera index
- [ ] Shift + Space resets F4 orbit, zoom, pan, and target
- [ ] Shift + Space resets F5 to sub-camera index 0
- [ ] Shift + Space resets F6 to spawn position, world-forward, and 5 m/s
- [ ] Shift + Space does not change the active mode
- [ ] Shift + Space does not change `previous_mode`
- [ ] Shift + Space from an already-default state still logs the reset
- [ ] F6 works at application start (before any vehicle exists)
- [ ] F6 WASD moves the camera relative to facing
- [ ] F6 wheel-with-shift changes speed
- [ ] F6 speed never drops below 5 m/s
- [ ] F6 speed has no upper bound
- [ ] F6 speed persists across mode switches
- [ ] F3 selects the vehicle cockpit
- [ ] F3 wheel cycles through driver cameras
- [ ] F3 wheel wraps at the end
- [ ] F5 selects a passenger camera
- [ ] F5 wheel cycles through passenger cameras
- [ ] F5 wheel wraps at the end
- [ ] For an articulated bus, F5 presents cameras from **all**
      segments
- [ ] F4 wheel cycles between vehicle and human external (if both
      exist)
- [ ] F4 wheel zooms when only one external target exists
- [ ] F4 mouse drag orbits
- [ ] F1 returns to the previous mode
- [ ] F1 pressed twice returns to the original mode
- [ ] HUD shows all six modes
- [ ] HUD dims unavailable modes
- [ ] HUD shows sub-camera index for F3 and F5
- [ ] HUD shows speed in F6
- [ ] No camera jitters, snaps, or lerps that fight user input
- [ ] Mode switches are instant

### Stage C2 — Log Audit

Run with `--log-level debug`. Confirm the log shows:

- Camera mode changes
- Camera sub-index changes
- Free-roam speed changes
- Warnings when an unavailable mode is pressed

### Stage C3 — Automated Tests

Add integration tests in `crates/opts-bevy/tests/camera.rs`:

- Test `CameraMode` transitions via simulated key events.
- Test sub-camera index cycling logic (pure function).
- Test free-roam speed clamping at 5 m/s minimum.
- Test that free-roam speed has no upper bound.
- Test that F1 toggles between modes correctly.
- Test the fallback matrix with simulated unavailable modes.

### Stage C4 — Final Report

Produce `docs/src/meta/camera-system-YYYY-MM-DD.md`:

- Which modes were implemented
- Which modes are stubbed (should be none except possibly F2 if
  humans aren't loaded yet)
- Screenshots of each mode (or the debug overlay)
- Any deviations from the design
- Any remaining issues

---

## 8. Reporting Cadence

1. Part A (design) — approval before Part B
2. Part B (implementation) — approval before Part C
3. Part C (verification) — end of task

Format:

```
## Camera System — Checkpoint N

**Phase:** <name>
**Modes implemented:** <list>
**Modes unavailable:** <list + reason>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 9. Exit Criteria

- ✅ `docs/src/architecture/camera-system.md` exists.
- ✅ All six modes implemented (except F2 if no human system —
  documented as unavailable).
- ✅ F1 previous-mode toggle works.
- ✅ Mouse wheel behavior per the specification table.
- ✅ Free-roam speed: min 5 m/s, no max, persists across modes.
- ✅ F5 presents cameras from all articulated segments.
- ✅ HUD shows all modes, dims unavailable, shows sub-index and
  speed.
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ Integration tests in `crates/opts-bevy/tests/camera.rs`.
- ✅ No hardcoded transforms — all from `model_cfg` parsing.
- ✅ Every file under 500 lines.
- ✅ Every public item documented.
- ✅ No `println!`.
- ✅ F4 supports middle-mouse pan.
- ✅ Pan rate scales with zoom level.
- ✅ Shift + Space resets every camera mode to its default.
- ✅ F6 reset restores spawn position, world-forward orientation,
      and 5 m/s speed.
- ✅ Reset does not change the active mode or `previous_mode`.
- ✅ HUD shows `[MMB] Pan` in F4 and `[Shift+Space] Reset` in all
      modes.

When finished, respond with the summary from Stage C4.

Then respond with the exact string:
"CAMERA SYSTEM COMPLETE — ready for user review"