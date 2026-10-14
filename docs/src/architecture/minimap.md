# The Mini Map: Architecture and Speed-Based Zoom

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Draft - Phase A design (speed zoom) awaiting approval; sections 1-2 describe the merged M-0071 code |
| **Phase** | Phase 9 (M-0071), extension on `feat/minimap-speed-zoom` |
| **Code** | `crates/opts-game/src/hud/minimap/` (`config.rs`, `mod.rs`, `raster.rs`, `source.rs`, `tests.rs`) |
| **Related** | [HUD registry §4a](hud-registry.md#4a-the-mini-map-m-0071), minimap path recognition (separate research branch) |

## 1. Current state (M-0071, merged)

A top-down vector drawing of road paths around the player, redrawn into one texture at most every 100 ms (`UPDATE_INTERVAL`, `redraw` in `mod.rs`), budget 1 ms.

| Part | Today |
| :--- | :--- |
| Zoom model | **Discrete.** `hud.minimap_zoom` is one of `ZOOM_STEPS_M = [100, 200, 500, 1000]` metres (visible radius), default 200. `+`/`-` call `step_zoom`; `MiniMapConfig::read` rejects values that are not a step. `redraw` passes `radius_m: config.zoom_m` to `MapView` unchanged. |
| Centre | The `PlayerVehicle` transform; without a vehicle the first `Camera3d` (`redraw`, lines `scene.centre`). |
| Speed | **Not read by the map.** `HudValues.speed_kmh` exists (`hud/values.rs`: `c.forward_speed_km_h().abs()`, the player's road speed). |
| Branch state | `feat/hud-minimap` is merged into master (`git merge-base --is-ancestor` true) and the BOARD marks M-0071 done on 2026-10-13; no minimap agent is active, nothing to coordinate with. |

## 2. Requirement

Zoom in when slow or stationary (see the junction you are standing at), zoom out when fast (see what is coming), with a smooth transition in both directions.

## 3. Design

### 3.1 Speed to radius

The user's chosen step (`hud.minimap_zoom`) keeps its meaning as the **cruise radius** `R_far`: the radius shown at and above `V_FAST`. The near radius is derived, so no second setting is needed:

```text
R_near = max(R_far / 3, 40 m)          (100 -> 40, 200 -> 67, 500 -> 167, 1000 -> 333)
s      = smoothstep((v - V_SLOW) / (V_FAST - V_SLOW))        v = filtered speed
R_tgt  = exp( ln R_near + s * (ln R_far - ln R_near) )       interpolation in log space
```

Constants (in `config.rs`, one place): `V_SLOW = 5 km/h`, `V_FAST = 60 km/h`, `NEAR_RATIO = 1/3`, `NEAR_MIN_M = 40`.

Rationale: log space makes equal speed steps look like equal zoom steps (zoom is perceptually multiplicative); smoothstep has zero slope at both ends, so there is no change at constant speed near standstill or at cruise. 60 km/h is where urban bus speeds end; faster vehicles (rail) sit at `R_far`, as the user's setting says.

### 3.2 Smoothing

Two stages, both in `redraw` on the 10 Hz tick (`dt` = `Time<Real>` since the last tick, so the filter is frame-rate independent):

| Stage | Filter | Constant | Purpose |
| :--- | :--- | :--- | :--- |
| Speed | first-order low pass `v += (v_in - v) * (1 - exp(-dt/τ_v))` | `τ_v = 1.5 s` | removes the creep/jitter of low speeds; a stop takes about 3 s to read as stopped |
| Radius | first-order low pass on `ln R`, **asymmetric** | `τ_out = 0.7 s` (growing), `τ_in = 2.0 s` (shrinking) | zoom out quickly when accelerating (you need the view ahead), zoom in slowly when braking so a short stop does not pump the view |
| Dead band | the radius is not changed while `|ln R_tgt - ln R| < 0.02` | 2 % | no flicker at constant speed |

At 10 Hz, a 3x change spreads over about 10 redraws; the texture is redrawn at the new radius each time (the raster path is radius-agnostic), so there is no scaling of an old image.

### 3.3 Speed signal

| Case | Source |
| :--- | :--- |
| Player vehicle | `HudValues.speed_kmh` (head unit's road speed, absolute value). An articulated bus or a train moves as one consist: the trailing units follow the head kinematically ([articulation-physics.md](articulation-physics.md)) and have no speed of their own, so the **head unit** speed is the right and only one. |
| No vehicle (map-only session) | Speed of the free camera: `|centre_now - centre_prev| / dt` from consecutive redraw centres (already computed there). A jump above 100 m in one tick (teleport, viewpoint) is treated as a reset: the filter state is set to the new target, no animation. |
| Vehicle changes / session start | `OnEnter(InGame)` already resets `MiniMapState`; the filters reset with it, and the first redraw starts at the target (no zoom animation at load). |

### 3.4 Settings, keys, protocol

- New setting `hud.minimap_zoom_mode`: `speed` or `fixed`. **Recommended default: `speed`** (the feature is the point; `fixed` restores today's behaviour exactly). The existing settings page gets one more choice next to the zoom.
- `+`/`-` keep stepping `hud.minimap_zoom` (the cruise radius) in both modes. Ctrl+F7 toggles the mode (F7 and Shift+F7 are taken; Ctrl combinations on F7 are not bound today; to be checked against `OPTS_RESERVED_KEYS`).
- Caption: "200 m - Heading up" becomes `<current radius> m` rounded to 10 m, plus `Auto` while in speed mode (new l10n key `hud-minimap-auto`, German and English).
- `hud.minimap` / `hud.state.minimap` gain `zoom_mode`, `radius_m` (current, smoothed), `speed_kmh` (filtered) and `target_m`, so the test and the verification can read the state instead of inferring it from pixels.

### 3.5 Code shape (small and contained)

- `minimap/zoom.rs` (new, pure, no Bevy): `ZoomParams`, `target_radius(speed_kmh, r_far, &ZoomParams) -> f32`, `ZoomFilter { speed, ln_radius, initialised }` with `step(dt, speed_in, r_far) -> f32` and `reset()`. Unit tests here.
- `minimap/config.rs`: `SETTING_ZOOM_MODE`, `ZoomMode`, the constants, parse/read.
- `minimap/mod.rs`: `MiniMapState` gains the filter; `redraw` replaces `radius_m: config.zoom_m` by the filtered radius and, for no vehicle, derives the camera speed; `update_caption` shows the current radius; `hud/protocol.rs` and the settings page for the new fields.
- No change to `raster.rs`, `source.rs`, the traffic crate or any other crate.

## 4. Verification plan (Phase C)

1. Unit tests on `zoom.rs`: target at `V_SLOW`/`V_FAST` is `R_near`/`R_far`; monotone in speed; at constant speed the filter converges and stays within the dead band; step response is faster outwards than inwards; frame-rate independence (the same trajectory with `dt` of 0.05 s, 0.1 s and 0.25 s agrees within 3 %); a stop from 60 km/h reaches within 5 % of `R_near` after 8 s.
2. Live run through the control protocol: drive script at 0, about 25 and about 60 km/h, read `hud.minimap.radius_m` each second (monotone, no jitter at constant speed: the standard deviation of `radius_m` over 10 s at constant speed is below 1 %), screenshot at each speed.
3. Free-camera case: no vehicle, `camera.set_mode free`, move; check `speed_kmh` is the camera speed and the radius follows.
4. `./scripts/local-ci.sh`; the 1 ms redraw budget test must still pass (the filter adds a few floating-point operations).

## 5. Out of scope

Pan, rails, route line and path recognition (separate research task), a manual override timer, per-vehicle tuning. The constants are global; no per-vehicle special cases.

## 6. Open points for the decision

| # | Question | Recommendation |
| :--- | :--- | :--- |
| 1 | Default mode | `speed` (change of behaviour for existing users, one key switches back) |
| 2 | `V_FAST` | 60 km/h; configurable only in code in v1 |
| 3 | Is `R_far` = user's step acceptable, or should speed mode ignore the step and use fixed 100-500 m? | `R_far` = user's step (keeps `+`/`-` meaningful) |
| 4 | Key for the mode | Ctrl+F7 |
