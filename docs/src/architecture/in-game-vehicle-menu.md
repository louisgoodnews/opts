# In-Game Vehicle Menu (Left Alt)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented (2026-10-12) |
| **Code** | `crates/opts-game/src/in_game/vehicle_menu.rs`, `omsi_world/respawn.rs` |

## Behaviour

* **Left Alt** (tap) opens and closes the menu while a session runs (the loading must be complete and no report modal open). While it is open the cursor is free and the driving, camera and cockpit inputs are gated through `opts_bevy::reporting::MenuModalActive` (the same run condition as the F1 report modal, `not_modal_active`). Escape steps back one page (configuration → select → main → closed) and does **not** leave the session; the gate stays up one frame longer than the menu so the same Escape press cannot reach the "back to the main menu" shortcut.
* **Main page** (also *Pause → Vehicle*): the **session vehicles** ([session vehicles](session-vehicles.md)), one row each: name · repaint · spawn entry point, the active one marked and filled; per row *Respawn* (back to its spawn point) and *Despawn* (asks first through the shell's confirmation dialog; removes the vehicle with its trailers); a click on the name **switches** to it. Then *Spawn vehicle…* and *Close*. **F8** (Shift+F8) switches without the menu.
* **Select Vehicle** (modal, three columns): (1) the vehicles of the catalogue, scrolled to the current vehicle; (2) its name, data, description, the repaints of its `Colorscheme` folders and *Configure vehicle…* (a second modal toggling the `[setvar]` equipment options the repaints declare; *Reset* restores the repaint's values); (3) the **entry points of the map that no AI vehicle stands on** (within 12 m). Entries share names on many maps; duplicates are numbered. *Confirm* is enabled once a vehicle and an entry are chosen.
* **Confirm** adds a vehicle to the session (it no longer replaces the active one): the scrambled meshes decode in the background, then it is built at the entry point with the chosen repaint and equipment options, becomes the active vehicle (the previous one is parked with its parking brake on) and the camera switches to the external view. A vehicle spawned on a far entry point appears before its surroundings: the tiles around it stream in while the camera follows.

## Entry points

`global.cfg` `[entrypoints]` records name a placed `entrypoint_*.sco` object by id code but not its tile. A worker thread (`EntryIndex`) reads every tile once per session and resolves each record to the object's absolute position (tile origin + object position, the record's elevation) and heading; records on spline attachments are resolved on demand when chosen. Until the index is ready all entries are listed (blocking cannot be checked).

## Automation

The control protocol has no absolute mouse position, so the menu can be driven by `menu.do {command}` (queued, executed next frame): `open`, `close`, `spawn`, `vehicle <text>`, `repaint <text>`, `entry <text>`, `config`, `toggle <variable>`, `step <variable> [-1|1]` (a choice to its previous or next value, a number by one), `done`, `confirm`, `despawn` (the active vehicle, no question), `switch <text>`, `respawn`, `back` (texts match names case-insensitively by substring). `menu.state` returns the page, choices and the free entry points; the session vehicles have their own commands `session.*`. The rows have the ids `vehicle.session.<id>.switch|respawn|despawn`. See [control protocol](control-protocol.md).

## Search and the entry index (2026-10-12)

* The vehicle list has a **search field**: type to filter (every word must occur in name, id, make, model or repaint), Backspace deletes; `menu.do search <text>` sets it. The counter shows `n of 947`. While the menu is open, `P` no longer pauses.
* The entry index starts when the session is up (not when the menu opens), reads the nearest tiles first and **publishes entries as it finds them**; the *blocked* filter applies to every entry resolved so far, and the list says how many are still being located. Entries on spline attachments cannot be checked for blocking (they are resolved when chosen).

## Change map (M-0108)

*Change map…* on the main page lists the installed maps (the current one marked). A click on a map opens *Change to <map>*, the one page that asks: the **start point** (the map's first, or the one picked), the **vehicle** (the active one is built again with its repaint and equipment options), the **time** (date, time of day and weather of the session are kept, to the minute) and the **session vehicles that are not carried over** (listed by name). *Change map* then writes the choices into the session setup and sets the state to the drive once more: the old session is torn down and the new one loads through the normal loading screen, exactly like *Load game* (`save::restore::begin_restore`); no second loading path exists. A drive started with `--map-path`, `--vehicle-path` or `--livery` drops those arguments at that moment, otherwise it would load the same map again.

If the active vehicle is not in the catalogue (it cannot be built on the other map), the page says so and offers *Go on without a vehicle* (free camera); a session without a vehicle stays without one. The rules are a pure function (`in_game/change_map.rs`, `plan`) with unit tests; `session.change_map {map, entry?, vehicle?, dry_run?}` (protocol 1.16.0) runs the same code (`dry_run` only answers what would happen). `menu.do changemap`, `map <text>` or `map <text> / <entry>`, and `confirm` drive the pages. Esc steps back (confirm page → list → main page).

Measured live (release build, Berlin-Spandau and Grundorf, the BRT C2 with the *Beige* repaint, six switches in a row, `scene` after each): `entities_live` 34,328 on Berlin-Spandau and 5,115 on Grundorf every time (no entity is left behind), meshes and materials equal too; the image count grows by 91 per switch. The same growth appears without a map change: every vehicle spawn adds about 24 images and 29 meshes that despawn does not release (`session.despawn` then `session.spawn` three times: images 358, 379, 400, 421; meshes 1640, 1669, 1698, 1727, entities back to 5,115 each time) - a content cache that keeps the vehicle's assets; it is listed on the BOARD, not changed here.

## Not done

* Rail vehicles spawn on the nearest track within 12 m of the entry point (`rail-vehicles.md` §8); an entry far from any track keeps the vehicle a road vehicle with an ERROR.
* A new repaint's textures for coupled parts follow the same repaint-name rules as a session start.

## Theme

The menus follow the persisted *dark/light* setting. Several OPTS instances (a game, a test run, another worktree) share one `settings.db`; each used to write **all** settings on exit, so an instance that had loaded an older value undid the other's choice (the menu colours seemed to flip between runs). `GameSettings::save` now writes only the values that differ from what the process loaded or saved last.
