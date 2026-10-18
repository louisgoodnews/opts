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

Since protocol 1.9.0 (see the [reference](../reference/control-protocol.md)): `config.list`, `config.save`, `config.load`, `config.delete`, `config.update` (rename, notes, favourite) and `config.duplicate`.

- `config.save` takes the vehicle chosen in *Plan a drive* or the player's vehicle in a drive. A taken name is refused unless `replace` is true.
- `config.load` writes the vehicle, the repaint and the accepted options into the plan and has the wizard rebuild its lists from it. In a running drive it answers `1007`: spawning a configuration into a session is a later part of the milestone.

## Wizard

The *What* step has a *Configurations* section under the livery (`new_game_screen/configs.rs`): a name field (the vehicle and livery name when left alone) with *Save as configuration*, then the list, favourites first and then the most recently used. Each row loads on a click and has *Star*, *Duplicate* and *Delete* (the first press asks, the second deletes). *Save* with a name that exists replaces that configuration. A configuration whose vehicle is gone says so in its row and does not load; one whose livery or options are gone loads without them and a toast says how many parts were left out (the check against the vehicle's livery data runs once that has been read, `finish_load`). The list is kept in the `ConfigLibrary` resource, read at startup and written at once on every change (a few hundred bytes); the control protocol works on the same resource, so a script and the screen see one list. A file that cannot be used (damaged, newer version) is shown above the list.

## Open

*Pause → Vehicle* list, renaming in the UI (`config.update` does it in the protocol), a thumbnail, spawning a configuration into a running session and the session save (M-0069) are not done.
