# Input Devices and Profiles

| Metadata | Value |
| :--- | :--- |
| **Status** | Menu navigation implemented and tested with synthetic input; **not tried with hardware** |
| **Spec** | [interaction and feel, section 1](../roadmap/phase-9/interaction-and-feel.md#1-devices-and-the-action-layer) |
| **Code** | `opts_gui::actions` (`key_action`, `button_action`, `gamepad_actions`), `opts_gui::interaction::{focus, navigation}` |

Every menu is operated through the **action layer**: devices become `UiAction`s, screens read actions, never devices. A new device or a rebinding changes one table.

## Profiles

| Profile | Devices | Buttons used by menus |
| :--- | :--- | :--- |
| **Keyboard and mouse** | primary | see the table in the spec; arrows move focus, Tab jumps between regions, Enter/Space confirm, Esc goes back |
| **Gamepad** (Xbox, PlayStation, generic, as Bevy's gamepad support names them) | secondary | D-pad and left stick: `Navigate`; South (A / cross): `Confirm`; East (B / circle): `Back`; North (Y / triangle): `Search`; Start: `Menu`; Select: `Help`; left and right shoulder: previous and next tab; right stick: scroll |
| **Steering wheel and pedals** | driving; menus through the wheel's buttons | a wheel that the operating system presents as a gamepad uses the gamepad profile through its D-pad and face buttons; **axes (wheel, pedals) are never used to navigate menus**; a wheel without a D-pad cannot be used for menus and needs the keyboard or mouse |
| **Touch** | not a target | target sizes keep it usable |

## What the pad can do, and how it is tested

- **Every control of the nine screens is reachable with the four directions alone.** From the current control the nearest one in the pressed direction takes focus; staying in the current region is preferred, but a clearly nearer control of another region (the rail from the content, the content from the rail) or the only one in that direction wins, because a pad has no Tab. Test: `a_gamepad_alone_reaches_every_control_of_every_screen` (`crates/opts-game/tests/a11y_matrix.rs`), which follows the directions breadth first from the first focus.
- **Confirm** activates the focused control exactly like Enter (library traversal test); **Back** closes dialogs, goes one level up in the shell and opens the pause overlay in a drive (`esc.rs`, `pause/mod.rs`).
- A focused control that is partly outside a scrolling container scrolls into view.
- The left stick is edge-triggered at 60 % deflection; there is no key repeat for the D-pad yet.

## Not done

- `Select` (`Help`) and `Start` (`Menu`) produce actions that no screen handles yet (finding A10 of the [accessibility audit](../roadmap/phase-9/accessibility-audit.md)).
- No test with real hardware; wheel button numbers differ per device and per driver. Testing a wheel: connect it, open Settings, move with its D-pad, confirm with its lower face button, report the device name and what happened.
- Rebinding of gamepad buttons is not in the Settings screen yet.
