# Camera Perspective System

> Status: **keys 1–5 select ego, cockpit, external, passenger and free roam; 0 returns to the previous mode; Shift+Space and the HUD hint are implemented** (`opts_bevy::camera`). Articulated passenger cameras span tractor and trailer segments.

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

## Articulated vehicles

Trailers named by `[couple_back]` are spawned as child segments (offset = tractor `[coupling_back]` − trailer `[coupling_front]`, chain depth ≤ 4). Their `[add_camera_pax]` presets are appended to the 4 list after the tractor's presets and are evaluated in the frame of their own segment (`BusVehicle::pax_camera`). Segments are rigid for now: no articulation angle is simulated.

> **Key change (F1 reporting decision):** F1 now belongs to the report modal (`reporting-feature.md`). Camera modes moved from F2–F6 to the number keys 1–5 and "previous mode" from F1 to 0. The gear selector's digit aliases (1/2/3) were removed to avoid the clash; gears are D/N/R (D was E while A/D steered; steering is now on the arrow keys). The debug overlay stays on F10.
