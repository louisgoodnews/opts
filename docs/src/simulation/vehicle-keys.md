# Vehicle Key Events and Bindings

| Metadata | Value |
| :--- | :--- |
| **Status** | Implemented 2026-10-18: key to script trigger path, rebindable actions in the settings, `vehicle.key` / `vehicle.keys` |
| **Code** | `opts-bevy` `vehicle_triggers.rs` (key to trigger), `opts-game` `vehicle_keys.rs` (actions, settings, conflict check), `control_drive.rs` (protocol) |
| **Evidence** | `Inputs/keyboard.cfg`, `keyboard_reset.cfg` of the installation, `{trigger:...}` handlers of the shipped `.osc` scripts (Studio Polygon, NF6D, HHA DT) |

## How OMSI does it

1. **Keys are not hard-wired to functions.** `Inputs/keyboard.cfg` lists `[entry]` blocks of *action name*, *DirectInput scan code* and *modifier bits* (2 = Shift, 4 = Ctrl, 1 = held key); see [engine config](../formats/text/engine-config.md). The `[vehicles]` section names **script triggers**: the action name is the trigger that the player's vehicle receives.
2. **The vehicle's scripts handle the trigger.** An `.osc` script has `{trigger:kw_scheinwerfer_toggle}` ... `{end}` blocks. A *held* key (bit 1, or the engine start and battery switch, which their scripts treat like buttons) also raises `<name>_off` on release (`{trigger:kw_m_enginestart_off}`); the scripts use it to reset anti-repeat flags and to release the starter.
3. **Cockpit hot-spots** use the same names through `[mouseevent]` in the model (`vehicle_triggers.rs`).
4. The keys are the user's: any entry can be changed in OMSI's key dialog. `keyboard_reset.cfg` is the default set.

## What OPTS had before this change

* `keyboard.cfg` was read and every `[vehicles]` binding raised its trigger on the player's vehicle, **except** bindings on keys that OPTS uses itself (A, E, Q, S, W, D, N, R, digits, ...). That dropped the battery switch (E), the microphone (Q) and all interior-light keys (6 to 9); the rest worked only for someone who knew OMSI's keys. Nothing in the game said what the keys were, nothing was rebindable, and a test could only use the raw `input.key`.
* `drive.trigger` raised a trigger by name over the protocol.
* The scripts' start-up sequence (battery, engine start) existed only behind `--script-drive`.

## What is added

* **Rebindable actions.** 24 actions (table below) have a setting each, `controls.vehicle.<action>`, of kind *Binding*. The value is `none` or `[Ctrl+][Shift+]<key>`, with the key by physical position (`L`, `7`, `F5`, `Numpad7`, `NumpadDivide`), so it is the same on QWERTZ and AZERTY. The defaults are OMSI's. The other `keyboard.cfg` vehicle bindings (IBIS, destination blinds, cash desk, gear letters) stay as the installation has them; a rebinding that uses their chord wins.
* **Conflict check.** Storing a key is refused (page and `settings.set` alike) when another action has it, or when OPTS itself uses it (driving, gears, cameras, overlays). A `Ctrl+` combination of a used key is allowed. The check runs in `SettingsRegistry::set`, so every route gives the same answer.
* **Settings page.** Controls shows one row per action: the current key, *Change key* (press the new key; Esc cancels; the refusal reason is shown above the rows), *Reset*.
* **Free keys.** A, E, Q and 6 to 9 are no longer reserved (steering is on the arrows; free camera flying is gated, and the vehicle triggers are off in free roam).
* **Protocol.** `vehicle.key` and `vehicle.keys` ([reference](../reference/control-protocol.md)). `vehicle.key action=lights` raises the trigger of the action whatever its key is; `vehicle.key key=L` presses a chord and raises what is bound to it, which tests the binding. The real keyboard path is also reachable with `input.key`.

## Default keys (OMSI)

| Action | Setting | Default | Trigger | Held (`_off` on release) |
| :--- | :--- | :--- | :--- | :--- |
| Battery switch | `controls.vehicle.battery` | E | `cp_batterietrennschalter_toggle` | yes |
| Start engine | `engine_start` | M | `kw_m_enginestart` | yes (hold until it runs) |
| Headlights (switch steps) | `lights` | L | `kw_scheinwerfer_toggle` | no |
| Parking lights | `parking_light` | Shift+L | `kw_standlicht_toggle` | no |
| High beam | `high_beam` | F | `kw_fernlicht_toggle` | no |
| Rear fog light | `fog_rear` | Ctrl+F | `taster_nebelschluss` | no |
| Indicator left | `indicator_left` | Num 7 | `blinker_left_set` | yes |
| Indicator right | `indicator_right` | Num 9 | `blinker_right_set` | yes |
| Indicators off | `indicator_off` | Num . | `blinker_off` | no |
| Hazard lights | `hazard` | B | `blinker_warn_toggle` | no |
| Horn | `horn` | H | `horn` | yes |
| Wipers up | `wiper` | none (OMSI: W, the throttle here) | `kw_wipermode_up` | no |
| Door 1 | `door_1` | Num / | `bus_doorfront0` | no |
| Door 2 | `door_2` | Num * | `bus_doorfront1` | no |
| Door 3 | `door_3` | Num 0 (OMSI: Num -, kept by the map zoom) | `bus_dooraft` | no |
| Door 4 | `door_4` | Shift+Num / | `bus_doorfront5` | no |
| Driver's light | `interior_driver` | 6 | `cp_fahrerlicht_toggle` | no |
| Interior, lower right | `interior_lower_right` | 7 | `cp_licht_untenrechts_toggle` | no |
| Interior, upper deck | `interior_upper` | 8 | `cp_licht_oberdeck_toggle` | no |
| Interior, lower deck | `interior_lower` | 9 | `cp_licht_unterdeck_toggle` | no |
| Microphone | `microphone` | Q | `cp_microphone` | yes |
| Hand over ticket | `ticket` | T | `ticket_give` | no |
| Ticket with change | `ticket_change` | Ctrl+T | `change_give` | no |
| Collect money | `ticket_take` | Shift+T | `change_take` | no |

Not rebindable here and driven by OPTS itself: throttle, brake, steering, gears (W, S, arrows, D/N/R), parking brake (`.`, which also raises `parking_brake_toggle`), cameras (digits, Tab), menus. A vehicle that does not define a trigger simply ignores the key (`handled: false` in the protocol answer).

## Open

* Digits 6 to 9 now raise the interior-light triggers although OMSI's own camera keys are not on them; OPTS's cameras use 0 to 5.
* Shift+W and Ctrl+W (wiper interval, washer) are dropped like W; the wiper action is unbound by default.
* Gamepad and wheel bindings are not available yet ([not available yet](../reference/not-available-yet.md)).
