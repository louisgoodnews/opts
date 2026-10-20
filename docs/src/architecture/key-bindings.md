# Key Bindings (M-0142)

Every key the player presses is an **action** with a default chord and a chord the player chose. The settings hold the choice; the input systems ask the registry; the hints print what the registry holds.

## Where things live

| What | Where |
| :--- | :--- |
| Built-in actions (id, group, default) | `opts_bevy::action_keys::ACTIONS` |
| The current chords | `opts_bevy::ActionKeys` (resource; `just_pressed(&keys, "help")`, `label("help")`) |
| Vehicle actions (`controls.vehicle.*`, raise a script trigger) | `opts_game::vehicle_keys::ACTIONS` |
| Settings rows, conflicts, reset | `opts_game::action_settings` (`controls.<group>.<id>`) |
| Hints | placeholders filled from `ActionKeys::label` (HUD hint line), chips of the Help table (`help::controls::ActionChip`) |

Chords are `Ctrl+`, `Shift+` and a physical key (`KeyChord`): `F7`, `Shift+F7`, `Period`. Matching is exact on Ctrl and Shift: Shift+F7 is not F7, so the mini map has three actions on F7.

## Rules

- **Conflicts**: a chord another action holds (vehicle or built-in) is refused with the name of that action; a key OPTS keeps (`OPTS_RESERVED_KEYS` minus the keys that are now default keys of an action) is refused; Ctrl combinations of reserved keys are free. Unbinding (`none`) is allowed.
- **Reset**: the row's *Reset* (one binding), `controls.reset.<group>` and `controls.reset.all` on the page, `settings.reset` and `settings.reset_keys {group?}` in the control protocol. A reset writes the defaults without conflict checks (the defaults of all actions are unique, a test holds that), so bindings that were swapped come back in any order.
- **Hints**: a message that names a key takes a placeholder (`{ $help }`); the text is built from `ActionKeys::label`, which prints the key as on the keyboard (`.` for Period, `Ctrl+H`). The Help table's chips update when the registry changes.
- **Persistence**: bindings are plain settings (`settings.db`, `settings.toml`), never part of a map profile.

## Adding an action

1. A row in `ACTIONS` (group, id, default text); the id is the name the input system asks for.
2. Messages `set-bk-<id>` and `set-bk-<id>-desc` in `messages_actions_en.ftl` and `_de.ftl`.
3. The input system takes `Option<Res<ActionKeys>>` and asks `opts_bevy::action_keys_of(&keys).just_pressed(&input, "<id>")`.
4. A row in the Help table (`help/controls.rs`, `rebindable(&["<id>"], ...)`); the test there checks that the source file asks for the id.

## Not yet moved to the registry

Throttle, brake, steering, the gear keys, the camera keys (1 to 5, 0, Shift+Space), the viewpoint editor (Tab), simulation pause (P), the vehicle menu (Alt), the tram keys (Z, X, C) and Esc: they need several keys per action (axes) and are the next slice of M-0142. They are still in `OPTS_RESERVED_KEYS`, so no vehicle action can take them.
