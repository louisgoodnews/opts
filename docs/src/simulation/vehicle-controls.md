# Vehicle-Type-Specific Controls: Analysis and Roadmap

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Road scheme implemented 2026-10-12 (arrows steer, A/D free, D/N/R gears, `.` parking brake); rail scheme (lever, direction) not yet |
| **Code today** | `opts-bevy` `vehicle_sim.rs` `vehicle_input_system` (one scheme for all vehicles) |

## Today (after 2026-10-12)

Road vehicles: W/↑ throttle, S/↓ brake, ←/→ steering, `.` parking brake, D/N/R gear. Rail players still use this scheme until C2. Before the change it was:

One scheme for every player vehicle: W/↑ throttle, S/↓ service brake, A/D or ←/→ steering, Space parking brake, E/N/R gear (Drive/Neutral/Reverse). Rail players use the same keys, so a tram gets a parking brake it does not have and no directional lever.

## Target (requested)

| Control | Bus (road) | Tram (rail) |
| :--- | :--- | :--- |
| Accelerate | W | Throttle lever: Q forward step, A back to 0, Y backward/brake step |
| Brake | S | Y beyond 0 (brake notches) |
| Steering | A / D | none (rails) |
| Gear / direction | D, N, R (Drive, Neutral, Reverse) | Directional lever on W (forward) / S (backward), neutral between |
| Parking brake | `.` | none (the tram's holding brake is applied by the throttle lever at full brake) |

**Conflicts to resolve.** Bus: `D` is both "steer right" and "Drive" in the request, and `A`/`D` steer today. Options: (a) gear D/N/R on the keys only while Shift is held or via the numpad, (b) keep steering on A/D and put the gears on E/N/R as now (the current state), (c) steer with the arrow keys exclusively and free A/D. Recommendation: (c) for the bus when the user wants D/N/R literally, with WASD still working where it does not collide (W/S pedals, arrows for steering), made configurable in the settings. Tram: `A` and `Y` are free there (no steering), `S`/`W` are lever, so there is no conflict inside the tram scheme. Also check the reserved keys of OMSI's `keyboard.cfg` (the vehicle triggers `automatic_D/N/R`, `kw_m_enginestart` map to the same letters in OMSI) so a later script-driven mode uses one table.

## Design

1. **`ControlScheme` per vehicle class**, chosen at spawn from the vehicle: `Road` (`.bus` without `[rail]`) or `Rail` (`[rail]` present, as `VehicleComposition` already knows). Stored as a component on the player vehicle; the input system matches on it.
2. **Rail model.** `RailInputs { direction: Fwd|Neutral|Back, lever: f32 in -1..1 }`. Lever > 0 = traction notch (0..1), < 0 = brake notch. Q steps the lever up, Y steps it down, A resets to 0 (all with a rate and a notch step so it feels like a lever, not a pedal). Direction lever W/S moves one detent per press and is changeable only at standstill (< 1 km/h) with the lever at 0. Tractive force = lever × max, direction picks the sign; brake notch applies the service brake, at the end stop also the holding brake. The vehicle's own `[control_cable_*]` state bus drives it later (see [rail vehicles](rail-vehicles.md) §8), the keys only set the lever and direction variables.
3. **Road model.** Keep `VehicleInputs`; add the parking brake on `.`; gear on D/N/R according to the decision above.
4. **Script mapping.** Road: gear keys raise `automatic_D/N/R`, parking brake `bremse_feststell_*` triggers, ignition keys later. Rail: direction and lever write the variables the tram's scripts read (to be found per vehicle from `[control_cable_*]` and the cockpit script; OMSI's `keyboard.cfg` entries for `fahrschalter`-type triggers).
5. **HUD and hints.** The vehicle menu and the keyboard hint panel show the keys of the active scheme (generated from a table in one place); the HUD shows direction, lever notch and brake state for rail, gear, speed and parking-brake lamp for road.
6. **Settings.** Key table in `GameSettings` (saved as a diff), defaults as above, rebinding in the controls page ([Controls](../SUMMARY.md) is still empty).
7. **Remote control.** `input.key` of the CLI stays raw keys; add `vehicle.control <name> <value>` (direction, lever, gear, parking brake) so tests do not depend on layouts (German `Y`/`Z` swap!).

## Layout note

Bevy's `KeyCode` is the physical key. On the user's QWERTZ layout the key labelled `Y` is `KeyCode::KeyZ`, and `Z` is `KeyCode::KeyY`; the Q/A/Y column is the bottom-left block either way. Bind by physical position (Q, A and the key below A) and show the label of the active layout in the hints; the existing `keyboard_layout.rs` already knows the QWERTZ/AZERTY label swaps.

## Roadmap

| Step | Content | Size |
| :--- | :--- | :--- |
| C1 | `ControlScheme` component, input system split road/rail, no behaviour change for road | S |
| C2 | Rail lever and direction model + physics hookup, no parking brake for rail | M |
| C3 | Road: `.` parking brake, gear keys per the decision, conflict resolved in settings | S |
| C4 | Hint panel and HUD per scheme | S |
| C5 | Script mapping (triggers) once script drive runs | M (key events to triggers done 2026-10-18, see [Vehicle Key Events and Bindings](vehicle-keys.md)) |
| C6 | Rebinding page, `vehicle.control` CLI command | M (rebinding of the vehicle system keys and `vehicle.key` done, see [Vehicle Key Events and Bindings](vehicle-keys.md); the driving keys are not rebindable yet) |
