# Bus Configurations

| Metadata | Value |
| :--- | :--- |
| **Milestone** | [M-0077](../roadmap/phase-10/M-0077.md) |
| **Status** | 🟡 Data model, store, control protocol and the wizard's *What* step implemented 2026-10-18; *Pause → Vehicle*, rename in the UI and session integration open |

A **bus configuration** is a vehicle, a repaint and the user's equipment options ([setvars](../formats/text/vehicles/setvars.md)) saved under a name and loaded again in one step. Code: `opts-game/src/bus_config.rs` (model, file, check), `control_configs.rs` (protocol).

## File

`bus_configurations.json` next to `drive_history.json` and `settings.db` (the user data directory):

```json
{
  "version": 1,
  "configurations": [
    {"id": "line-33", "name": "Line 33", "vehicle": "citybus", "repaint": "Blue",
     "setvars": [["display_ext", 5], ["vis_x", 0]], "notes": "", "favourite": false,
     "created": 1792300000, "last_used": 1792300000}
  ]
}
```

- `vehicle` is the catalogue id (the `.bus` file stem), not the display name, so a rename in the catalogue does not break a configuration. `repaint` is empty for the vehicle's default. `setvars` are only the user's changes, sorted by name; the repaint's own values are not stored.
- `id` is a slug of the first name, numbered when taken, and never changes; renaming changes `name` only. Names are unique ignoring case (umlauts included).
- Fields that are missing take their defaults, so a file from before a field existed loads. A file without `version` has the layout of version 1.
- A file with a **newer** `version` is reported and never overwritten. A **damaged** file is reported; the next save moves it aside as `bus_configurations.json.broken` before it writes a new one. Writes go through a temporary file and a rename.

## Check on load

A configuration is checked against the installation when it is listed or loaded (`BusConfiguration::check`), using the repaints and the setvar catalogue of its vehicle ([setvars](../formats/text/vehicles/setvars.md)):

| Problem | Code | Effect |
| :--- | :--- | :--- |
| The vehicle is not in the catalogue | `vehicle_missing` | The configuration cannot be loaded; the plan is not changed |
| The vehicle does not offer the repaint | `repaint_missing` | The default repaint is used |
| The vehicle has no such option | `setvar_unknown` | The value is not applied |
| The catalogue does not accept the value (kind, range) | `setvar_rejected` | The value is not applied |

Nothing in the check panics; every problem is reported with a code and a sentence.

## Control protocol

Since protocol 1.9.0 (see the [reference](../reference/control-protocol.md)): `config.list`, `config.save`, `config.load`, `config.delete`, `config.update` (rename, notes, favourite) and `config.duplicate`; since 1.18.0 also `config.spawn`.

- `config.save` takes the vehicle chosen in *Plan a drive* or, in a drive, the active session vehicle (its catalogue id, repaint and equipment options from the session registry). A taken name is refused unless `replace` is true.
- `config.load` writes the vehicle, the repaint and the accepted options into the plan and has the wizard rebuild its lists from it. In a running drive it answers `1007`; use `config.spawn` there.
- `config.spawn {id, entry?, entry_id?}` builds a configuration as a new session vehicle of the running drive: the same check as `config.load`, then the request of `session.spawn` (`bus_config_spawn::spawn_params`); the new vehicle becomes the active one. A missing vehicle or `.bus` file refuses it (`1008`, `data.problems`).

## Wizard

The *What* step has a *Configurations* section under the livery (`new_game_screen/configs.rs`): a name field (the vehicle and livery name when left alone) with *Save as configuration*, then the list, favourites first and then the most recently used. Each row loads on a click and has *Star*, *Duplicate* and *Delete* (the first press asks, the second deletes). *Save* with a name that exists replaces that configuration. A configuration whose vehicle is gone says so in its row and does not load; one whose livery or options are gone loads without them and a toast says how many parts were left out (the check against the vehicle's livery data runs once that has been read, `finish_load`). The list is kept in the `ConfigLibrary` resource, read at startup and written at once on every change (a few hundred bytes); the control protocol works on the same resource, so a script and the screen see one list. A file that cannot be used (damaged, newer version) is shown above the list.

## Alt menu

*Configurations…* on the main page of the vehicle menu (`in_game/vehicle_menu/configs_page.rs`) lists the saved configurations, favourites first and then the most recently used, each with its vehicle, repaint and number of options. *Spawn* builds it as a new vehicle at the session's start point and closes the menu; a configuration whose vehicle or `.bus` file is gone shows why instead of the button, one that lost a repaint or an option spawns without it and says so. *Rename* turns the name into a field (type, Backspace, Enter or *Save name*; Esc or *Cancel* leaves it; an empty or taken name is refused in the status line). *Delete* asks with a second press. *Save current vehicle* stores the active session vehicle under its name (numbered when taken). A repaint the catalogue does not list (the vehicle's own default as its `.bus` names it) is stored as the default, so the new entry does not read as having a missing repaint. `menu.do configs`, `spawnconfig <text>`, `saveconfig`, `renameconfig <text>` with `typename <text>` and `commitrename` drive it; Esc goes back to the main page.

## Session save

The session save (M-0069) already stores each session vehicle with its catalogue id, repaint and equipment options, so a spawned configuration is restored with the session; the saved configurations themselves are a user file next to the settings and are not part of a save.

## Open

A thumbnail is not done.
