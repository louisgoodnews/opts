# Engine Configuration — `keyboard.cfg`, `gamectrler.cfg`, `options.cfg`

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Text (keyword-based) |
| **File Extensions / Names** | `Inputs/keyboard.cfg`, `Inputs/keyboard_reset.cfg`, `Inputs/gamectrler.cfg`, `Inputs/*.kyb`, `options.cfg`, `option_presets/*.oop` |
| **Parser Crate** | `opts-core` (settings; low priority) |
| **Parser Module** | `opts_core::formats::engine_config` |
| **Encoding** | UTF-8 / ASCII (`keyboard.cfg`, `gamectrler.cfg`, `options.cfg`), CP1252/UTF-8 (`.kyb`) |
| **Status** | 🟡 Draft — structure verified; one field calibrated against the manual |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

These files hold the **user's** configuration: key bindings, game-controller mappings and options. They are not game content, and OPTS will define its own settings, but their grammar documents what OMSI exposes as bindable and configurable — in particular, `keyboard.cfg` binds keys to the **same trigger names** that scripts define. Evidence tags: **[C]** counted, **[H]** seen in files, 📖 shipped manual, ❓ inferred. Machine-specific values (a user's bindings) are described structurally only.

## 🔤 General Structure

### `Inputs/keyboard.cfg` (and `keyboard_reset.cfg`)

Keyword file, 656 lines. Two section markers, `[game]` and `[vehicles]`, followed by **130 `[entry]` blocks** (38 under `[game]`, 92 under `[vehicles]`) [C]. Each entry has **three lines**:

| Line | Content | Example |
| ---: | :--- | :--- |
| 1 | **action name** | `sim_pause`, `view_set_driver`, `ticket_give`, `bus_doorfront0`, `IBIS_setmode_route` |
| 2 | **key code** — an index into the key-name table in `ENG.kyb` / `DEU.kyb` | `25` = `P`, `44` = `Z`, `48` = `B`, `181` = `Num /` |
| 3 | **modifier mask**: `0` none, `2` Shift, `4` Ctrl, `6` Ctrl+Shift (`1` = Alt ❓) | `4` with `20`(T) = change |

**Calibration (📖 + [C]).** The key-code column matches the key-name tables line for line (`25 → P`, `44 → Z`, `48 → B`, `181 → Num /`; the table has 131 entries, indices up to 237), i.e. the codes are the **DirectInput keyboard scan codes** ❓ (identical numbering for the classic PC keyboard). The modifier values were tied down with the manual's shortcuts: `ticket_give` = T (modifier 0, "hand over a ticket"), `change_give` = T + modifier 4 (the manual: *Ctrl+T* hands over a ticket with change), `change_take` = T + modifier 2 (*Shift+T* collects the money), `IBIS_setmode_route` = Num / + modifier 4 (*Ctrl+Num /* = Line/Course). So **2 = Shift, 4 = Ctrl**, and 6 = both; the remaining bit (1) is presumably Alt ❓. Modifier values seen: 0 ×69, 4 ×28, 1 ×16, 6 ×8, 2 ×7, 3 ×1, 5 ×1.

The **`[vehicles]` action names are script trigger names**: `bus_doorfront0`, `parking_brake_toggle`, `IBIS_setmode_route`, `kw_scheinwerfer_toggle` … occur as `{trigger:name}` in vehicle scripts ([script sets](vehicles/script-sets.md)). `[game]` actions (`view_set_*`, `sim_pause`, `screenshot`, `quicksave`, `debug_*`, `exit`) are engine commands. The manual confirms that any event reachable by mouse or shortcut can be bound 📖.

`keyboard_reset.cfg` is the shipped default set (645 lines, same grammar), used to reset bindings.

### `Inputs/*.kyb`

Key-name tables (`<index><TAB><name>`, 131 lines, index up to 237) for the interface languages — see [`.kyb`](../kyb.md). The index is the scan code used by `keyboard.cfg`.

### `Inputs/gamectrler.cfg`

Keyword file, 189 lines, one group per known controller (4 in this install). Blocks per group [C]:

| Keyword | Lines | Content |
| :--- | ---: | :--- |
| `[ctrl]` | 2 | **device name** (`SideWinder Joystick`, `Logitech G25 Racing Wheel USB`) and an index |
| `[axis]` | 16 | eight pairs of numbers; `-1 0` marks an unassigned slot, other pairs hold the device axis and an option (the simulator's five logical axes are steering/force feedback, throttle, brake, clutch, combined throttle/brake — [predefined variables](vehicles/predefined-variables.md#controller-axes-gamectrl_axestxt)); exact pair meaning ❓ |
| `[buttons]` | 1 + 2·N | a count and then **pairs of (action name, button index)**; the actions are trigger names such as `automatic_1`, `automatic_D`, `automatic_N` (19 pairs for the G25, 0 for the joystick) |
| `[FFScale]` | 2 | two force-feedback scale factors (`1.000 1.000`) |

### `options.cfg` and `option_presets/*.oop`

`options.cfg` (152 lines) is the **current** option set; `option_presets/*.oop` (75 files) are named presets. They share one grammar: a keyword per setting — `[last_map]`, `[last_driver]`, `[currWeather_ICAO]`, `[performance_*]`, `[sound_*]`, `[AI*]`, `[language]`, … — where a bare keyword is a boolean flag and valued keywords take one or more lines. The full keyword list is in [misc types](misc-types.md#oop); `[last_map]` and `[last_driver]` hold paths (`maps\Dusseldorf\global.cfg`, `Drivers\OMSI-Fan.odr`).

## 🧪 Examples

```
[entry]
change_give          ← action (a script trigger name)
20                   ← key code: T
4                    ← modifier: Ctrl
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-EC01 | Confirm modifier bit 1 = Alt and the scan-code numbering (the table has gaps at keys without names). | 🟢 Low |
| Q-EC02 | Meaning of each pair in `gamectrler.cfg` `[axis]` (device axis, inversion, dead zone?). | 🟢 Low |
| Q-EC03 | Do `[game]` entries also appear as `[vehicles]` triggers when a vehicle defines the same name? | 🟢 Low |

## 📝 Provenance

- **Files read (2026-10-04):** `Inputs/keyboard.cfg` (all 130 entries tallied), `keyboard_reset.cfg`, `gamectrler.cfg`, `ENG.kyb`/`DEU.kyb`, `options.cfg`.
- **Calibration:** modifier values compared with the shortcuts in the shipped manual (📖, paraphrased); key codes compared with the `.kyb` tables.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [`.kyb`](../kyb.md), [misc types (`.oop`)](misc-types.md#oop), [predefined variables](vehicles/predefined-variables.md), [script sets](vehicles/script-sets.md), [environment](environment.md)
