# In-Game Vehicle Menu (Left Alt)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented (2026-10-12) |
| **Code** | `crates/opts-game/src/in_game/vehicle_menu.rs`, `omsi_world/respawn.rs` |

## Behaviour

* **Left Alt** (tap) opens and closes the menu while a session runs (the loading must be complete and no report modal open). While it is open the cursor is free and the driving, camera and cockpit inputs are gated through `opts_bevy::reporting::MenuModalActive` (the same run condition as the F1 report modal, `not_modal_active`). Escape steps back one page (configuration → select → main → closed) and does **not** leave the session; the gate stays up one frame longer than the menu so the same Escape press cannot reach the "back to the main menu" shortcut.
* **Main page:** *Despawn vehicle* (removes the player's vehicle with all coupled parts), *Spawn vehicle…*, *Close*.
* **Select Vehicle** (modal, three columns): (1) the vehicles of the catalogue, scrolled to the current vehicle; (2) its name, data, description, the repaints of its `Colorscheme` folders and *Configure vehicle…* (a second modal toggling the `[setvar]` equipment options the repaints declare; *Reset* restores the repaint's values); (3) the **entry points of the map that no AI vehicle stands on** (within 12 m). Entries share names on many maps; duplicates are numbered. *Confirm* is enabled once a vehicle and an entry are chosen.
* **Confirm** replaces the player's vehicle: the old one is removed, the scrambled meshes of the new one decode in the background, then it is built at the entry point with the chosen repaint, its equipment options (set on the script engine after the repaint's own), and the camera switches to the external view. A vehicle spawned on a far entry point appears before its surroundings: the tiles around it stream in while the camera follows.

## Entry points

`global.cfg` `[entrypoints]` records name a placed `entrypoint_*.sco` object by id code but not its tile. A worker thread (`EntryIndex`) reads every tile once per session and resolves each record to the object's absolute position (tile origin + object position, the record's elevation) and heading; records on spline attachments are resolved on demand when chosen. Until the index is ready all entries are listed (blocking cannot be checked).

## Automation

The control protocol has no absolute mouse position, so the menu can be driven by `menu.do {command}` (queued, executed next frame): `open`, `close`, `spawn`, `vehicle <text>`, `repaint <text>`, `entry <text>`, `config`, `toggle <variable>`, `done`, `confirm`, `despawn`, `back` (texts match names case-insensitively by substring). `menu.state` returns the page, choices and the free entry points. See [control protocol](control-protocol.md).

## Not done

* No search field in the vehicle list (947 entries on the reference install).
* Rail vehicles spawn on the nearest track within 12 m of the entry point (`rail-vehicles.md` §8); an entry far from any track keeps the vehicle a road vehicle with an ERROR.
* A new repaint's textures for coupled parts follow the same repaint-name rules as a session start.
