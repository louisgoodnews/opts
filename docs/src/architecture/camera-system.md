# Camera Perspective System

> Status: **keys 1–5 select ego, cockpit, external, passenger and free roam; 0 returns to the previous mode; Shift+Space and the HUD hint are implemented; Tab edits the driver's viewpoint (M-0084)** (`opts_bevy::camera`). Articulated passenger cameras span tractor and trailer segments.

Code: `crates/opts-bevy/src/camera/`. Session wiring: `crates/opts-game/src/in_game/`.

## Modes

| Key | Mode | Target | Sub-cameras (mouse wheel) | Status |
| :--- | :--- | :--- | :--- | :--- |
| 0 | previous | the last active mode | — | ✅ |
| 1 | Ego | player human, head anchor | one; mouse look relative to the head (yaw ±120°, pitch ±80°) | ✅ available when a human is loaded; WARN on first press otherwise |
| 2 | Cockpit | user vehicle | `[add_camera_driver]` list, default `[set_camera_std]`; wheel cycles, wraps; right-drag looks | ✅ |
| 3 | External | user vehicle | left-drag orbit, middle-drag pan (`zoom × 0.005` m/px, moves the orbit centre), wheel zoom 0.5–50 m; centre = `[set_camera_outside_center]` | ✅ (vehicle target only; human target with 1) |
| 4 | Passenger | user vehicle | `[add_camera_pax]`; wheel cycles, wraps; right-drag looks | ✅ for single vehicles; articulated segment flattening waits for trailer spawning |
| 5 | Free roam | none | one | ✅ |

`ModelConfig`/`BusManifest` already expose the camera data (`BusManifest::driver_cameras`, `pax_cameras`, `std_camera_index`, `outside_camera_center` — see `BusVehicle`); conversion to Bevy space uses the existing `camera_preset_to_bevy_transform`. No camera transform is hard-coded. The one exception today is the interim chase distance/height, which `opts-game` derives from the vehicle's measured body size.

## Types

`CameraMode`, `CameraAvailability` (which modes can be entered, refreshed each frame) and `CameraState` (mode, `previous_mode`, `sub_camera_index`, free-roam speed/yaw/pitch/spawn, external zoom/yaw/pitch/`pan_offset`, mouse sensitivity). `CameraState::switch_to` records `previous_mode` and clears transient per-mode state; free-roam speed persists for the whole run.

## Systems (all in `Update`, chained)

`update_camera_availability` → `handle_camera_key_input` (keys 0–5) → `handle_camera_reset_input` (Shift+Space) → `update_free_roam_camera` → `sync_vehicle_rig_mode` → `manage_free_roam_cursor`. The vehicle rig (`vehicle_camera_system`) runs only in Cockpit/External; vehicle driving keys are ignored in free roam; Shift+Space no longer toggles the parking brake.

## Free roam (5)

- Mouse look always active; the cursor is locked and hidden while in 5.
- W/S forward/back along the look direction, A/D strafe, E/Q up/down (world Y). Movement is camera-relative.
- Shift + wheel multiplies speed by 1.25 per notch (÷1.25 down). Minimum 5 m/s, no maximum, persists across mode switches.
- Entering 5 records the camera pose; Shift+Space returns to that position, resets yaw/pitch to world-forward and speed to 5 m/s.
- If no vehicle could be loaded, the session starts in free roam over the spawn point so the world can still be inspected. `--camera free` starts there deliberately.

## Reset (Shift+Space)

Fires on the key edge in every mode, never changes the mode or `previous_mode`, and always logs at `DEBUG`. Per-mode defaults: 5 as above; 3 yaw 0, pitch −15°, zoom 8 m, pan 0; 2/4 sub-camera index 0 (or `[set_camera_std]`).

## Design decisions (confirm before 1–4)

1. **3 wheel:** zoom when one target exists, cycle when vehicle and human both exist (context-sensitive, as recommended).
2. **3 pan:** translate the *orbit centre* (`pan_offset`), so later orbits revolve around the panned point. Pan rate = `zoom × 0.005` m/px. Middle mouse is not bound anywhere else.
3. **2/4 wrap** at the ends; 3's target list does not.
4. **5 reset position:** the pose at the most recent entry into 5.
5. **Articulated vehicles:** flatten passenger cameras by segment then declaration order; each camera is composed with its segment's world transform from the `ChainAssembly` in `opts-sim`.

## HUD

`[0] Prev  [1] Ego  [2] Cockpit  [3] External  [4] Pax  [5] Free   [Shift+Space] Reset`. The active mode is wrapped in `*…*`, unavailable modes in `(…)`; the free-roam entry appends the speed. `[MMB] Pan` joins the line when the external view gets orbit/pan.

## Cursor handling

The cursor is captured (locked and hidden) only while a 3D camera exists **and** the mode is free roam or ego. Free roam is the *default* state before a session starts, so the lock must never apply in the menus; an earlier version captured the cursor at start-up and made the menus unusable. Holding **Left Alt** releases the cursor (and mouse look) so HUD buttons stay reachable. Debug overlay: F10.

## Ego perspective (1)

The camera follows the `HumanHead` anchor of the player's character (`opts_bevy::human`): a child of the head joint at `(0, height − 0.11 m − neck height, 0)`, so it moves with the vehicle and any head pose. The character is spawned seated at the vehicle's `[drivpos]` (hips on the contact point, thighs horizontal) with a skinned mesh bound to the 13 canonical bones; see [humans.md](../formats/text/humans.md). Shift+Space resets the look angles.

## Per-vehicle views (2/3/4)

Cockpit and passenger cameras are **per-vehicle data**: `BusManifest::driver_cameras`, `pax_cameras`, `std_camera_index` and `outside_camera_center` (from the `.bus` file) are carried on the `BusVehicle` component and converted with `camera_preset_to_bevy_transform`; the camera pose is `vehicle world transform × preset`. The preset's horizontal FOV becomes the camera's vertical FOV for the window aspect. The HUD shows the index (`[2] Cockpit [4/11]`). The legacy `VehicleCameraRig` no longer drives the camera when the camera plugin is present.

## Viewpoint editor (driver's view, key 2) — M-0084

Code: `opts_bevy::camera::viewpoint` (model, state, keys), `opts-game` `hud/viewpoint.rs` (panel) and `viewpoints.rs` (store, protocol).

**How the view is defined today.** The driver's view is **key 2 (Cockpit)**; key 4 is the passenger view. Its cameras are the vehicle's `[add_camera_driver]` presets (position `x y z`, `dist`, yaw, pitch, **horizontal FOV**); `[set_camera_std]` picks the one shown first. The label reads `Driver's view x/n` (`x` = sub-camera index + 1, `n` = number of `[add_camera_driver]` presets) for 1.5 s after a change. The preset's position is not what is drawn from when the player character exists: the eye position is the character's head (seated at `[drivpos]`, shifted so its eyes lie at the preset). The horizontal FOV becomes the vertical one for the window aspect.

**What the editor changes.** A `Viewpoint` is an **offset from that eye** plus an optional FOV, so the default (all zero, no FOV) *is* the vehicle-defined view and nothing about the vehicle is copied:

| Value | Axis | Limit | Step |
| :--- | :--- | :--- | :--- |
| X | right (+) / left | ±0.60 m | 1 cm |
| Y | up (+) / down (height) | ±0.60 m | 1 cm |
| Z | forward (+) / back | ±0.60 m | 1 cm |
| FOV | horizontal, degrees | 30 – 120° | 1° |

The camera applies `vehicle rotation × (x, y, −z)` to the eye and the FOV in `update_cockpit_camera`, so every change is live; look-around, sub-camera cycling and Shift+Space are untouched.

**Keys (only in view 2; Tab does nothing elsewhere).** Tab opens the editor; while it is open: Up/Down pick the row, Left/Right adjust (hold repeats, Shift = 10 steps), Home resets the draft to the vehicle's view, Enter or Tab saves and closes, **Esc cancels** (and does not open the pause menu: `EscGate.viewpoint_open`). The simulation is not paused. While the editor is open it owns the keyboard: driving keys and the script key bindings stand still (`not_editing_viewpoint`, `vehicle_accepts_input`), the vehicle coasts. Leaving the view, or switching the sub camera, cancels an open editor. Tab is reserved against OMSI key bindings (`OPTS_RESERVED_KEYS`).

**Panel.** Right edge, below the camera label, painted from the instrument tokens (day and night), spawned only while the editor is open, no focusable element: buttons are `PointerOnly`. One row per value with name, − / + buttons, a track (click or drag sets the value) and the number in cm or degrees; a "Not saved yet" line, the key hint, and the buttons *Vehicle default*, *Cancel*, *Save*. The selected row has a `›` marker as well as a brighter name. Strings: `hud-viewpoint-*` in the HUD message tables (en, de). Saving raises an information alert; a failed write raises a warning.

**Storage.** `viewpoints.json` in the user data directory beside `drive_history.json`:

```json
{ "version": 1,
  "vehicles": { "man_sd77": { "cameras": { "0": { "x": 0.03, "y": -0.05, "z": 0.0, "fov_deg": 80.0 } } } } }
```

The key is the vehicle's catalogue identity (the `.bus` file stem, lower case, stored on the spawned vehicle as `ViewpointKey`), so repaints share it; the camera index is the driver preset. Loading happens when the vehicle spawns, when the sub camera changes and when the view is entered (the file is re-read), never while the editor is open; a vehicle without an entry shows its own view. Reading is tolerant: a missing file is empty, an unusable one is moved to `viewpoints.json.bad`, a newer `version` is ignored, an invalid entry (bad key or camera index, non-finite or out-of-range value) is dropped with a warning and its neighbours stay. Saving merges into the file's current content and writes through a temporary file. A viewpoint equal to the vehicle's own removes its entry (*Vehicle default* then *Save*).

**Control protocol.** `camera.viewpoint.get`, `.set {x?, y?, z?, fov_deg?}`, `.save`, `.reset` (see the [reference](../reference/control-protocol.md)); out-of-range values answer `-32602` rather than being clamped.

**Not done yet:** applying one viewpoint to all vehicles of a model with a per-vehicle override, export/import for sharing, a default-FOV setting, and the passenger and ego views (the structure is per camera index, the editor is wired to view 2 only).

## Articulated vehicles

Trailers named by `[couple_back]` are spawned as child segments (offset = tractor `[coupling_back]` − trailer `[coupling_front]`, chain depth ≤ 4). Their `[add_camera_pax]` presets are appended to the 4 list after the tractor's presets and are evaluated in the frame of their own segment (`BusVehicle::pax_camera`). Segments are rigid for now: no articulation angle is simulated.

> **Key change (F1 reporting decision):** F1 now belongs to the report modal (`reporting-feature.md`). Camera modes moved from F2–F6 to the number keys 1–5 and "previous mode" from F1 to 0. The gear selector's digit aliases (1/2/3) were removed to avoid the clash; gears are D/N/R (D was E while A/D steered; steering is now on the arrow keys). The debug overlay stays on F10.


## The `MainCamera` marker

`opts_bevy::MainCamera` marks the one camera that shows the player's view (spawned with the in-game
`Camera3d` in `vehicle_spawner.rs`). Camera input and modes, the sky dome, tile streaming, traffic, pedestrian
and rail centres, the minimap, the scenery LOD/light culling, the look presets, `Hdr` (`graphics.hdr`) and the
control protocol all query `With<MainCamera>`. Never use `With<Camera3d>` for "the player's view": auxiliary
cameras (the reflection prototype's `MirrorCamera`, benchmark cameras) are `Camera3d` too, and a second match
moves the wrong camera or makes a `single()` query fail. Code that really means any 3D camera says so in a comment.
