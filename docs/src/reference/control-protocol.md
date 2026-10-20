<!-- Generated from the command table of opts-cli (crates/opts-cli/src/commands.rs) by the test `reference_page_is_up_to_date`; regenerate with `UPDATE_CONTROL_DOC=1 cargo test -p opts-cli reference`. -->

# Control Protocol Reference

Protocol version **1.19.0**. Every command below is declared once in the command table; the same table answers `help <command>` and `commands` at runtime, drives the `opts-cli` argument parser and generates this page. Transport, message format, gating and worked sessions are in [the architecture page](../architecture/control-protocol.md).

*Debug* commands deliberately skip the input layer and its gates (they act on the world directly); every other command goes through the same messages, keys and systems a person would use, so a dialog that swallows a key press also refuses the command (`1009`).

## Errors

Every error carries `data.name` (the symbolic code) and `data.retryable`.

| Code | Name | Retry | Meaning |
| ---: | :--- | :--- | :--- |
| -32700 | `PARSE_ERROR` | no | the line is not valid JSON |
| -32600 | `INVALID_REQUEST` | no | not a JSON-RPC request object |
| -32601 | `METHOD_NOT_FOUND` | no | unknown method (see `commands`) |
| -32602 | `INVALID_PARAMS` | no | missing, mistyped or out-of-range parameter |
| -32603 | `INTERNAL_ERROR` | no | handler panicked or failed unexpectedly |
| 1001 | `NOT_READY` | later | the app is not in a state in which the command makes sense (no session, wrong screen, element hidden or disabled) |
| 1002 | `FORBIDDEN_PATH` | no | path outside the allowed read/write roots |
| 1003 | `UNAUTHORIZED` | no | token missing or wrong |
| 1004 | `NOT_FOUND` | no | entity, element, asset or file does not exist |
| 1005 | `TIMEOUT` | later | the app (or a `ui.wait_for` condition) did not answer in time |
| 1006 | `BUSY` | later | rate limit exceeded or command queue full |
| 1007 | `UNSUPPORTED` | no | specified but not implemented in this build |
| 1008 | `FAILED` | no | the operation ran and failed (`data` carries the cause) |
| 1009 | `GATED` | later | the input layer refused the command because a dialog or another gate blocks it, exactly as it would swallow a key press |

## Group `meta`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| `ping` | - | 1.0 | Check that the server answers |
| `version` | - | 1.0 | App and protocol version |
| [`help`](#help) | `command?` (string) | 1.0 | Describe one command (or list all) |
| `capabilities` | - | 1.0 | List all commands by group |
| `auth` | `token` (string) | 1.0 | Authenticate with the server token |
| `shutdown` | `force?` (boolean) | 1.0 | Exit the application |
| [`commands`](#commands) | `group?` (string) | 1.1 | Every command with its argument schema, result and documentation |
| [`batch`](#batch) | `commands` (json)<br>`stop_on_error?` (boolean) | 1.0 | Run several commands in order in one round trip |

### `help`

Describe one command (or list all).

**Result:** `{command, group, summary, since, args, result, doc, debug, implemented}`; without `command` a list of `{method, summary}`

### `commands`

Every command with its argument schema, result and documentation.

**Result:** `{protocol_version, count, commands:[{method, group, summary, since, args, result, doc, debug, implemented}]}`

The machine-readable form of the generated reference. `args` entries are `{name, type, required, position, aliases, values?}`; `debug` marks commands that skip the input layer on purpose.

### `batch`

Run several commands in order in one round trip.

**Result:** `{results:[{result}|{error}]}` in command order

At most 100 commands, no nesting. A command that waits (`ui.wait_for`, `scene.load_map`, `ui.drag`) finishes before the next one starts, so a batch is a script: act, wait, observe.

## Group `query`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| `state` | - | 1.0 | App state, camera mode, fps, worst frame time of the last 10 s (frame_ms_max), frame |
| [`scene`](#scene) | - | 1.0 | Loaded map, tiles, entity and asset counts |
| [`scene.tiles`](#scenetiles) | - | 1.0 | Resident tiles, tile loading mode and radius |
| `camera.get` | - | 1.0 | Camera mode and transform |
| `sim.get` | - | 1.0 | Pause state, simulation time, time scale, session clock, weather and lighting quality |
| `log.tail` | `n?` (integer) | 1.0 | Last N log lines |
| `log.filter` | `target?` (string)<br>`level?` (string)<br>`contains?` (string)<br>`n?` (integer) | 1.0 | Filtered log lines |
| `entity.list` | `filter?` (string) | 1.0 | List entities |
| `entity.get` | `id` (integer) | 1.0 | Components of one entity |
| `resource.list` | - | 1.0 | List resource types |
| `resource.get` | `name` (string) | 1.0 | Value of one resource |
| `asset.list` | `kind?` (string) | 1.0 | List loaded assets |
| `asset.get` | `path` (string) | 1.0 | Metadata of one asset |
| `hud.state` | - | 1.0 | What the HUD shows: widgets, texts, alert, speed, gear |
| [`vehicle.get`](#vehicleget) | - | 1.0 | Position, heading and speed of the player vehicle (absolute OMSI coordinates) |
| `vehicle.script_vars` | `contains?` (string) | 1.0 | Script variables of the player vehicle (substring filter) |
| [`session.vehicles`](#sessionvehicles) | - | 1.2 | The session's user vehicles: id, name, repaint, spawn entry point, lifecycle, active marker |
| [`session.saves`](#sessionsaves) | - | 1.3 | The list of saves, the registered save sections and the state of a restore |
| [`audio.state`](#audiostate) | - | 1.19 | The sound engine: volumes, voice budget, listener and the loudest emitters |
| `menu.state` | - | 1.0 | Page, choices and free entry points of the in-game vehicle menu |

### `scene`

Loaded map, tiles, entity and asset counts.

**Result:** `{map, start_tile, tiles_loaded, tiles_missing, entity_count, entities_live, meshes, materials, images, asset_count, stats}`

`entity_count` is the number of entity slots the world has allocated (it only grows); `entities_live` counts the entities that exist now (since 1.16), which is the number to compare before and after leaving and entering a session; `meshes`, `materials` and `images` are the asset counts that add up to `asset_count`.

### `scene.tiles`

Resident tiles, tile loading mode and radius.

**Result:** `{mode, radius, automatic, setting_radius, recommended_radius, load_all, map_radius, start_tile, listed, listed_missing, resident_count, resident, queued, building, stream_budget_ms}`

What the streamer keeps resident (M-0080). `mode` is `all` (every tile, never unloaded; `radius` is `null`) or `radius` (the tiles within `radius` of the tile the camera stands on: 1 is 3 x 3). `setting_radius` is the stored `gameplay.tile_radius` (`0`: automatic, then `radius` is `recommended_radius`, derived from the hardware profile and limited to `map_radius`, the radius that covers the map from the start tile); `load_all` is `gameplay.load_all_tiles`. `resident` lists the tile keys `[x, y]` (a tile whose build is still in progress is in it and named in `building`; it fills in over a few frames within `stream_budget_ms` per frame, M-0089), `queued` counts tiles waiting to be built, `listed` the tiles `global.cfg` names and `listed_missing` those among them without a file. Set the mode with `settings.set`. `1001` outside a drive.

### `vehicle.get`

Position, heading and speed of the player vehicle (absolute OMSI coordinates).

**Result:** `{x, y, z, heading_deg, speed_kmh}`

### `session.vehicles`

The session's user vehicles: id, name, repaint, spawn entry point, lifecycle, active marker.

**Result:** `{active, count, vehicles:[{id, name, catalogue_id, repaint, setvars, spawn_entry_name, spawn_point, lifecycle (spawning|alive|despawned|failed), despawn_reason, error, active, trailers}], place_label}`

The registry of the vehicles the user spawned (the wizard's choice, the pause menu, `session.spawn`); parked and AI vehicles are not listed. Despawned vehicles stay in the list as history. `place_label` is what the HUD shows for the active vehicle (the entry point it was spawned at) or `null`.

### `session.saves`

The list of saves, the registered save sections and the state of a restore.

**Result:** `{dir, running, saves:[{id, name, map, vehicles, play_time_s, saved_unix, sections, unknown_sections, from_backup, issues}|{id, error}], sections:[{id, version, about}], restore:{state, id?, warnings?}}`

Most recent first. A file that cannot be read is listed with its `error` (`kind`: `corrupt`, `newer_format`, ...). `issues` name missing or changed content (`MissingMap`, `MissingVehicle`, `Changed`).

### `audio.state`

The sound engine: volumes, voice budget, listener and the loudest emitters.

**Result:** `{enabled, volumes:{master, effects, ambience, engine, interface}, voices:{budget, emitters, audible, with_voice, virtualized, playing}, listener, clips_cached, clips:{ready, loading, failed, bytes}, scenery:{voices, playing, one_shots, cold_misses, unsupported_conditions, files_not_found}, traffic:{voices, playing}, top:[{entity, bus, gain, distance, voice}]}`

`gain` is the level an emitter computes: its own gain × the distance fade × its bus' volume × the master volume (volumes are percent, squared into a gain). `voices.playing` counts voices whose sink exists, that is the sound card accepted them: on a machine without an output it stays 0 while `with_voice` rises. `listener` is the main camera's position (Bevy metres) or `null`. `clips` counts the sound files the worker threads decoded, are decoding or could not decode, with the bytes held; `scenery` reports the sound definitions of placed objects (M-0139): all of them, the continuous ones playing now, the one-shots started by script triggers, triggers that found their file still decoding, definitions waiting for a `[conditionSingle]` (not supported yet) and files not found; `traffic` the continuous sound voices of AI vehicles (engine, rolling noise) and how many have an emitter now (M-0140).

## Group `ui`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| `ui.tree` | `visible_only?` (boolean) | 1.0 | The UI as a tree: id, role, label, state, rect, focusable |
| [`ui.click`](#uiclick) | `id` (string)<br>`count?` (integer) | 1.0 | Activate an element by id (a text field gets focus) |
| `ui.focus` | `id` (string) | 1.0 | Move keyboard focus to an element |
| `ui.action` | `action` (string)<br>`arg?` (string) | 1.0 | Send a UI action (navigate right, confirm, back, ...) |
| `ui.state` | `id` (string) | 1.0 | State of one element by id |
| [`ui.screen`](#uiscreen) | - | 1.1 | Where the app is: state, screen, open dialogs, toasts, focus, scrollables |
| [`ui.scrollables`](#uiscrollables) | `visible_only?` (boolean) | 1.1 | List the scrollable views with offset, content size and position |
| [`ui.scroll`](#uiscroll) | `target?` (string)<br>`by?` (number)<br>`dx?` (number)<br>`pages?` (number)<br>`to?` (string = top \| bottom \| left \| right)<br>`item?` (string) | 1.1 | Scroll a view by an amount, by pages, to the top or bottom, or to an item |
| [`ui.text`](#uitext) | `text` (string)<br>`id?` (string)<br>`replace?` (boolean)<br>`submit?` (boolean) | 1.1 | Type text into a text field (optionally by id; replaces the content by default) |
| [`ui.select`](#uiselect) | `id` (string)<br>`option?` (string)<br>`index?` (integer) | 1.1 | Choose an option of a combo box, a tab, a radio or a list row |
| [`ui.toggle`](#uitoggle) | `id` (string)<br>`value?` (boolean) | 1.1 | Set a switch, check box or radio to on or off (or flip it) |
| [`ui.hover`](#uihover) | `id` (string) | 1.1 | Move the pointer over an element (tooltips, hover states) |
| [`ui.key`](#uikey) | `key` (string)<br>`ctrl?` (boolean)<br>`shift?` (boolean)<br>`alt?` (boolean)<br>`duration_ms?` (integer) | 1.1 | Press a key with modifiers (Ctrl, Shift, Alt) through the input layer |
| [`ui.drag`](#uidrag) | `from` (string)<br>`to?` (string)<br>`dx?` (number)<br>`dy?` (number)<br>`steps?` (integer) | 1.1 | Drag from one element to another element or by an offset |
| [`ui.wait_for`](#uiwait_for) | `until` (string = exists \| visible \| gone \| enabled \| disabled \| focused \| checked \| text \| screen \| dialog \| no_dialog \| app_state \| hud_ready \| speed_above \| speed_below \| distance_above \| paused \| running \| sim_time_above \| items_above)<br>`id?` (string)<br>`value?` (string)<br>`number?` (number)<br>`timeout_ms?` (integer) | 1.1 | Wait until a condition holds (element, screen, dialog, app state, speed, ...) |
| [`ui.navigate`](#uinavigate) | `screen` (string) | 1.0 | Open a shell destination or deep link (home, drive, settings, help, screen:settings/graphics) |

### `ui.click`

Activate an element by id (a text field gets focus).

`count` activates the element that many times (stepper buttons like `settings/<category>/<key>/up`); each activation is a separate message, as separate clicks would be.

### `ui.screen`

Where the app is: state, screen, open dialogs, toasts, focus, scrollables.

**Result:** `{app_state, screen, loading?, modal_open, dialogs:[{id, tag?, label, visible, controls:[{id, label, role, enabled, focused}]}], toasts:[{id, label}], focus, back_stack, scrollables}`

One call that answers "what is on screen and what can I do now". `dialogs` lists every open modal (confirmations, the pause overlay, settings) with the ids of its controls, so a script can answer a confirmation without reading the whole tree; `loading` (stage and progress) is present while a drive loads, which is when the screen has nothing to click.

### `ui.scrollables`

List the scrollable views with offset, content size and position.

**Result:** `{count, scrollables:[{id, owner, rect, offset, viewport, content, max, at_top, at_bottom, items:[id...]}]}`

`id` is what `ui.scroll target` takes: the element's own id, else `<nearest id>/scroll` (suffix `-2`, `-3` for duplicates). `items` are the ids of the elements inside the view in layout order, so `ui.scroll item` can reveal any of them.

### `ui.scroll`

Scroll a view by an amount, by pages, to the top or bottom, or to an item.

**Result:** the scrollable after the move (as in `ui.scrollables`) plus `moved`

Without `target` the view under focus is used, else the first visible one; while a dialog is open only views inside it can be scrolled (`1009` otherwise). Exactly one of `by` (logical pixels, positive scrolls down), `dx`, `pages` (viewport heights), `to` or `item` is needed. `item` reveals the element with that id (the same effect as focusing it); `1004` when the item is not inside the view.

### `ui.text`

Type text into a text field (optionally by id; replaces the content by default).

**Result:** `{ok, id, value}` - the field's content once the text has arrived

Focuses the field `id` (like a click), clears it when `replace` is true (the default) and types the text as keyboard events, so validation, search-as-you-type and the settings search react exactly as for a person. `submit` presses Enter afterwards. Without `id` the focused field receives the text. `1001` when the field is disabled, hidden or behind a dialog.

### `ui.select`

Choose an option of a combo box, a tab, a radio or a list row.

**Result:** `{ok, id, selected}`

For a combo box `option` is the visible text (or `index` the position): the drop-down is opened and the entry activated like clicks. For a tab, radio or list row no option is needed. Disabled targets answer `1001` with the reason.

### `ui.toggle`

Set a switch, check box or radio to on or off (or flip it).

**Result:** `{ok, id, checked, changed}`

Only activates the element when its state differs from `value`; without `value` it flips. Settings with a revert prompt open the prompt as for a click.

### `ui.hover`

Move the pointer over an element (tooltips, hover states).

**Result:** `{ok, id, pos}` with the pointer position in logical pixels

Places the cursor at the centre of the element's rectangle and reports it to the window like a mouse move, so picking, hover states and tooltips behave as for a person. `1001` for an element without a rectangle (not laid out or hidden).

### `ui.key`

Press a key with modifiers (Ctrl, Shift, Alt) through the input layer.

**Result:** `{ok, key, modifiers}`

Presses the modifiers, then the key, and releases everything after `duration_ms` (default 50). The events are ordinary keyboard events, so shortcuts (`Ctrl+F`, `Ctrl+Tab`) and gating are the same as for a keyboard.

### `ui.drag`

Drag from one element to another element or by an offset.

**Result:** `{ok, from, to?, from_pos, to_pos}` once the button is released

Presses the left button on the centre of `from`, moves the cursor in `steps` frames (default 8) to the centre of `to` or by `(dx, dy)` logical pixels and releases. Answers after the release.

### `ui.wait_for`

Wait until a condition holds (element, screen, dialog, app state, speed, ...).

**Result:** `{ok, until, waited_ms}` on success; error `1005` with `data.last` (the last observed value) when `timeout_ms` (default 10000, at most 120000) passes first

Polled once per frame on the main thread. `exists`, `visible`, `gone`, `enabled`, `disabled`, `focused` and `checked` need `id`; `text` needs `id` and `value` (the field or label contains it); `screen` takes the shell id in `value` (`settings`), `dialog` a dialog tag or id in `value` (`no_dialog` when none is open); `app_state` takes `MainMenu`, `Loading`, `InGame`, ...; `speed_above`/`speed_below` take km/h in `number`, `distance_above` the metres driven since the session started (see `drive.state`), `sim_time_above` seconds of simulation time, `items_above` the number of entries a scrollable `id` lists (a list fills in after the catalogue scan). Use it instead of sleeping.

### `ui.navigate`

Open a shell destination or deep link (home, drive, settings, help, screen:settings/graphics).

Sends the same `Navigate` message as a rail click; the shell may refuse a destination that is not available yet. While a dialog is open (the first-run question, a confirmation) the rail is behind it for a person, so the command answers `1009`.

## Group `input`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| `input.key` | `key` (string)<br>`duration_ms?` (integer) | 1.0 | Press and release a key |
| `input.key_hold` | `key` (string) | 1.0 | Press a key and keep it down |
| `input.key_release` | `key` (string) | 1.0 | Release a key |
| `input.text` | `text` (string) | 1.0 | Type text into the focused field |
| `input.mouse.move` | `dx` (number)<br>`dy` (number) | 1.0 | Move the mouse relatively |
| `input.mouse.click` | `button` (string)<br>`x?` (number)<br>`y?` (number) | 1.0 | Click a mouse button |
| `input.mouse.scroll` | `delta` (number) | 1.0 | Scroll the wheel |

## Group `sim`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| [`sim.pause`](#simpause) | - | 1.0 | Pause the simulation |
| `sim.resume` | - | 1.0 | Resume the simulation |
| `sim.step` | `frames` (integer) | 1.0 | Advance N frames while paused |
| `sim.time_scale` | `factor` (number) | 1.0 | Set the simulation speed |
| [`sim.set_time`](#simset_time) | `seconds` (number)<br>`date?` (string)<br>`weather?` (string)<br>`rate?` (number) | 1.0 | Set the session clock (and date, weather, clock rate) |

### `sim.pause`

Pause the simulation.

Pauses the simulation clock (`Time<Virtual>`) directly, not through the pause overlay; the overlay is reached with `ui.action menu` or `ui.key Escape`.

### `sim.set_time`

Set the session clock (and date, weather, clock rate).

**Result:** `{clock, seconds_of_day, date, map_date, weather, rate, revision}` after the change

Sets the session environment like the pause menu's *Environment* page: `seconds` since midnight (0 ≤ s < 86400), optionally `date` (`YYYY-MM-DD`; a new date is applied like *Apply* and may rebuild the map, Chrono), `weather` (an installed preset's id such as `Sturm` or `#CAVOK`, its slug, its localised name or a word of it; the old ids `Clear Summer Day`, `Rainy Afternoon`, … work too and stand for the closest installed preset; `sim.get` lists the available ids as `weather_presets`) and `rate` (clock seconds per simulated second, 0 < rate ≤ 3600). Answers two frames later with the environment; `1001` outside a drive. `sim.get` reports the clock as `clock`, `seconds_of_day`, `date`, `map_date`, `weather` and `clock_rate`, and the weather as `weather_canonical`, `weather_presets` and `weather_conditions`.

## Group `scene`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| [`scene.load_map`](#sceneload_map) *(debug)* | `path` (string)<br>`entry?` (string) | 1.0 | Start a session on a map |
| [`scene.unload`](#sceneunload) *(debug)* | - | 1.0 | Return to the main menu |
| `camera.set_mode` | `mode` (string)<br>`sub?` (integer) | 1.0 | Switch camera mode |
| `camera.teleport` *(debug)* | `x` (number)<br>`y` (number)<br>`z` (number)<br>`yaw?` (number)<br>`pitch?` (number) | 1.0 | Move the camera (OMSI coordinates) |
| `render.look` | `look?` (string) | 1.0 | Get or set the colour look (vivid, natural, bevy) |
| [`render.stats`](#renderstats) | `reset?` (boolean)<br>`parts?` (boolean) | 1.0 | Frame times, draw counts per render phase, memory |
| [`vehicle.set`](#vehicleset) *(debug)* | `path` (string) | 1.0 | Replace the player vehicle |
| `entity.spawn` | `asset` (string)<br>`transform?` (json) | 1.0 | Spawn an asset |
| `entity.despawn` | `id` (integer) | 1.0 | Despawn an entity |
| `entity.set_component` | `id` (integer)<br>`type` (string)<br>`value` (json) | 1.0 | Set a component value |
| [`camera.reset`](#camerareset) | - | 1.2 | Reset the current camera view to its default, like Shift+Space |
| [`camera.look`](#cameralook) | `yaw?` (number)<br>`pitch?` (number)<br>`dx?` (number)<br>`dy?` (number) | 1.1 | Turn the camera: absolute yaw/pitch (free roam) or relative look (cockpit, external) |
| [`camera.move`](#cameramove) | `forward?` (number)<br>`right?` (number)<br>`up?` (number)<br>`duration_ms?` (integer) | 1.1 | Move the free camera forward/right/up for a duration, like the movement keys |
| [`camera.viewpoint.get`](#cameraviewpointget) | - | 1.2 | The driver's-view viewpoint: offsets, field of view, stored value, editor state |
| [`camera.viewpoint.set`](#cameraviewpointset) | `x?` (number)<br>`y?` (number)<br>`z?` (number)<br>`fov_deg?` (number) | 1.2 | Change the driver's-view viewpoint live (opens the editor; not stored until saved) |
| [`camera.viewpoint.save`](#cameraviewpointsave) | `scope?` (string = vehicle \| model) | 1.2 | Store the shown viewpoint for this vehicle (or all of its model) and camera and close the editor |
| [`camera.viewpoint.reset`](#cameraviewpointreset) | - | 1.2 | Back to the vehicle's own viewpoint and remove the stored one |
| [`hud.minimap`](#hudminimap) | `zoom?` (integer)<br>`zoom_mode?` (string = speed \| fixed)<br>`orientation?` (string = north \| heading)<br>`enabled?` (boolean) | 1.4 | Read or change the HUD mini map: zoom (radius in metres), zoom mode, orientation, on/off |
| [`hud.fullmap`](#hudfullmap) | `open?` (boolean)<br>`span?` (number)<br>`x?` (number)<br>`y?` (number)<br>`follow?` (boolean)<br>`fit?` (boolean) | 1.15 | Read or drive the full-size map: open or close, zoom span, centre, follow |
| [`camera.door`](#cameradoor) | `door?` (integer) | 1.17 | Read the door camera or choose the door it shows |
| [`session.spawn`](#sessionspawn) | `vehicle` (string)<br>`repaint?` (string)<br>`setvars?` (json)<br>`entry?` (string)<br>`entry_id?` (integer) | 1.2 | Spawn a vehicle at an entry point of the map; it becomes the active vehicle when it exists |
| [`session.change_map`](#sessionchange_map) | `map` (string)<br>`entry?` (string)<br>`vehicle?` (string = carry \| none)<br>`dry_run?` (boolean) | 1.16 | Change the map of the running drive (the Alt menu's Change map): the vehicle and the time are carried over |
| [`session.switch`](#sessionswitch) | `id?` (integer)<br>`step?` (integer) | 1.2 | Make a session vehicle the active one (cockpit camera, HUD profile, input and scripts follow) |
| [`session.despawn`](#sessiondespawn) | `id?` (integer) | 1.2 | Remove a session vehicle with its trailers (default: the active one) |
| [`session.respawn`](#sessionrespawn) | `id?` (integer) | 1.2 | Put a session vehicle back at its spawn point, at rest (default: the active one) |
| [`formation.validate`](#formationvalidate) | `saved?` (string)<br>`zug?` (string)<br>`cars?` (json)<br>`name?` (string)<br>`reverse?` (boolean) | 1.6 | Check a train formation against the installation (every problem is listed) |
| [`formation.spawn`](#formationspawn) | `saved?` (string)<br>`zug?` (string)<br>`cars?` (json)<br>`name?` (string)<br>`reverse?` (boolean)<br>`entry?` (string)<br>`entry_id?` (integer) | 1.6 | Spawn a train formation at an entry point; its first car becomes the active vehicle |
| [`formation.save`](#formationsave) | `saved?` (string)<br>`zug?` (string)<br>`cars?` (json)<br>`name?` (string)<br>`reverse?` (boolean)<br>`id?` (string)<br>`overwrite?` (boolean) | 1.6 | Save a train formation in the user's formation folder |
| [`formation.list`](#formationlist) | - | 1.6 | List the saved train formations, newest first |
| [`formation.delete`](#formationdelete) | `id` (string) | 1.6 | Delete a saved train formation |
| [`session.save`](#sessionsave) | `name` (string)<br>`overwrite?` (boolean) | 1.3 | Save the running drive under a name |
| [`session.load`](#sessionload) | `name` (string) | 1.3 | Restore a save (leaves the running drive); poll `session.saves` for the progress |
| [`session.delete_save`](#sessiondelete_save) | `name` (string) | 1.3 | Delete a save and its backup (no confirmation) |
| [`session.rename_save`](#sessionrename_save) | `name` (string)<br>`new_name` (string) | 1.3 | Change the name of a save (the file keeps its id) |
| [`menu.do`](#menudo) | `command` (string) | 1.0 | Drive the in-game vehicle menu (open, spawn, vehicle &lt;text&gt;, repaint &lt;text&gt;, done, ...) |
| [`vehicle.setvars`](#vehiclesetvars) | `values?` (json)<br>`allow_unlisted?` (boolean) | 1.8 | List or change the equipment options (setvars) of the vehicle being planned or driven |
| [`config.list`](#configlist) | - | 1.9 | List the saved bus configurations (vehicle, repaint, equipment options) with their check against the installation |
| [`config.save`](#configsave) | `name` (string)<br>`notes?` (string)<br>`replace?` (boolean) | 1.9 | Save the vehicle, repaint and equipment options of the plan (or of the driven vehicle) under a name |
| [`config.load`](#configload) | `id` (string) | 1.9 | Write a saved configuration into the plan (vehicle, repaint, equipment options) |
| [`config.spawn`](#configspawn) | `id` (string)<br>`entry?` (string)<br>`entry_id?` (integer) | 1.18 | Build a saved configuration as a new vehicle of the running drive |
| [`config.delete`](#configdelete) | `id` (string) | 1.9 | Delete a saved configuration |
| [`config.update`](#configupdate) | `id` (string)<br>`name?` (string)<br>`notes?` (string)<br>`favourite?` (boolean) | 1.9 | Rename a saved configuration, change its notes or its favourite mark |
| [`config.duplicate`](#configduplicate) | `id` (string)<br>`name` (string) | 1.9 | Copy a saved configuration under a new name |
| [`audio.tone`](#audiotone) | `bus?` (string = effects \| ambient \| vehicles \| ui)<br>`loop?` (boolean)<br>`at?` (json) | 1.19 | Play a sine tone (a test signal for the sound output) |
| [`audio.play`](#audioplay) | `file` (string)<br>`bus?` (string = effects \| ambient \| vehicles \| ui)<br>`loop?` (boolean)<br>`at?` (json) | 1.19 | Play a sound file of the installation through the engine |
| [`audio.stop`](#audiostop) | - | 1.19 | Stop and remove the sounds `audio.tone` and `audio.play` made |

### `scene.load_map`

Start a session on a map.

A debug shortcut around the plan-a-drive wizard: it starts a session directly. The player-facing route is the wizard (`ui.*`). Answers when the HUD is up. A load that failed answers at once with `1008 FAILED` (the problems in the message). A load with problems answers while the loading screen waits for *Continue* (`waiting_for_continue: true`, the problems listed): click `loading.continue` (`ui.click`) to start the drive.

### `scene.unload`

Return to the main menu.

A debug shortcut: leaves the drive without the confirmation dialog (`pause.leave` in the pause overlay is the player route).

### `render.stats`

Frame times, draw counts per render phase, memory.

`frame_times` covers the frames since the last reset (average, median, 99th percentile, 1 % low rate); `phases` the latest frame's batch sets, bins and entities of Bevy's render phases summed over all views (one `multidraw_sets` entry is one multi-draw call, a batchable bin one instanced draw, every unbatchable entity one draw); `parts` (opt-in, scans the world) the visible mesh entities with their distinct meshes, materials and pairs; `gpu_diagnostics_ms` needs `OPTS_RENDER_DIAG=1`. `reset` clears the frame-time window after reading.

### `vehicle.set`

Replace the player vehicle.

Restarts the session with the vehicle and answers like `scene.load_map` (including `1008` for a failed load and `waiting_for_continue` for a load with problems).

### `camera.reset`

Reset the current camera view to its default, like Shift+Space.

**Result:** `{mode, sub_camera}`

Same code as the key. Free roam returns to the pose recorded on entry (speed 5 m/s); external to yaw 0, pitch -15 degrees, zoom 8 m, no pan; ego looks forward; the driver's view (2) and the passenger view (4) keep their sub camera and look straight ahead, and view 2 returns to the stored viewpoint (an open editor is cancelled). Never changes the mode. Answers `1009` while a dialog or the vehicle menu is open.

### `camera.look`

Turn the camera: absolute yaw/pitch (free roam) or relative look (cockpit, external).

**Result:** `{mode, yaw_deg, pitch_deg}`

`dx`/`dy` are mouse-motion pixels delivered like a right-drag; `yaw`/`pitch` (degrees, compass yaw) set the free-roam camera directly and answer `1001` in other modes.

### `camera.move`

Move the free camera forward/right/up for a duration, like the movement keys.

**Result:** `{ok, mode, duration_ms}`

Switches nothing: answers `1001` unless the camera is in free roam (use `camera.set_mode free`). Positive values move forward, right and up; the movement keys are held for `duration_ms` (default 500) and released, so speed follows `free_roam_speed`.

### `camera.viewpoint.get`

The driver's-view viewpoint: offsets, field of view, stored value, editor state.

**Result:** `{editing, dirty, row, vehicle, camera, cameras, viewpoint:{x,y,z,fov_deg}, stored:{x,y,z,fov_deg}, fov_deg, default_fov_deg, limits, file}`

Offsets are metres from the vehicle's own `[add_camera_driver]` eye: `x` right, `y` up, `z` forward (limit +-0.6). `fov_deg` in the viewpoint is `null` while the vehicle's own field of view is used; the top-level `fov_deg` is the one shown. `vehicle` is the catalogue identity (the `.bus` file stem, lower case) the viewpoint is stored under, `camera` the driver camera index.

### `camera.viewpoint.set`

Change the driver's-view viewpoint live (opens the editor; not stored until saved).

**Result:** the `camera.viewpoint.get` result

Only the given values change. Out-of-range values answer `-32602` (offsets +-0.6 m, `fov_deg` 30..120) instead of being clamped. Answers `1001` unless the driver's view (`camera.set_mode cockpit`) is active and `1009` while a dialog or menu is open. Equivalent to the editor panel: the camera shows the change at once; `camera.viewpoint.save` stores it, leaving the view or changing the sub camera cancels it.

### `camera.viewpoint.save`

Store the shown viewpoint for this vehicle (or all of its model) and camera and close the editor.

**Result:** the `camera.viewpoint.get` result

Writes `viewpoints.json` (next to `drive_history.json`) before it answers. `scope` `model` (default `vehicle`) applies the viewpoint to every vehicle of the same model (the folder of its `.bus` file) and drops this vehicle's own override; other vehicles' own entries keep winning. A viewpoint equal to the vehicle's own removes the stored entry (or keeps an explicit default when the model has one). Answers `1001` unless the driver's view is active.

### `camera.viewpoint.reset`

Back to the vehicle's own viewpoint and remove the stored one.

**Result:** the `camera.viewpoint.get` result

Resets offsets and field of view, stores the default (which removes the entry) and closes the editor. Answers `1001` unless the driver's view is active.

### `hud.minimap`

Read or change the HUD mini map: zoom (radius in metres), zoom mode, orientation, on/off.

**Result:** `{enabled, shown, zoom_m, zoom_mode, radius_m, target_m, speed_kmh, orientation, zoom_steps_m, texture_px, side_px, compact, redraws, last_us, worst_us, budget_us, road_chords, markers, centre, up_heading_deg}`

Without parameters it reads. `zoom` is the visible radius, one of 100, 200, 500, 1000 (m); `zoom_mode` is `speed` (the radius follows the speed: `zoom` is then the largest radius, `radius_m` the radius drawn now, `target_m` what the speed asks for, `speed_kmh` the filtered speed) or `fixed`; `orientation` is `north` (north up) or `heading` (the player's heading up); `enabled` shows or hides the map. The three are the settings `hud.minimap_zoom`, `hud.minimap_orientation` and `hud.minimap` (changed and saved the same way; the keys F7, Shift+F7, `+` and `-` do the same). The map is centred on the player's vehicle, or on the free camera when there is no vehicle. `last_us` and `worst_us` are the time of the last and the slowest redraw (budget `budget_us`, at most 10 redraws per second); `hud.state` carries the same object as `minimap`. An unknown zoom or orientation answers `-32602`.

### `hud.fullmap`

Read or drive the full-size map: open or close, zoom span, centre, follow.

**Result:** `{open, follow, span_m, centre, px_per_m, screen, texture_px, redraws, last_us, worst_us, road_chords, markers, min_span_m, max_span_m, overview}`

The full-size map of the session (key F6). Without parameters it reads. `open` opens or closes it (the change happens in the next frame; opening is refused silently while a dialog, the pause overlay or the vehicle menu holds the input, or before the session has loaded). `span` is the number of metres across the shorter side of the screen, limited to `min_span_m`..`max_span_m` (50 m to 20 km); `x` and `y` (east and north in OMSI metres) centre the map and stop following the vehicle; `follow` follows the active vehicle (or the camera) again; `fit` shows the whole map (every lane read so far, with a margin; the End key does the same). `centre` is the world point in the middle of the screen, `px_per_m` the scale in logical screen pixels, `screen` the logical window size. `last_us` and `worst_us` are the time of the last and the slowest redraw (a redraw follows a change of the view at once, a moving vehicle at most ten times a second, a still map twice a second). Mouse drag, wheel and Home work through `input.mouse_*` and `input.key` like for a person. `overview` is the reader of the roads of the whole map (`{started, complete, percent, tiles_done, tiles_total}`): the first opening starts a background thread that reads every tile of the map, and the map draws these roads for all tiles that are not resident while it fills in. `1001` outside a drive.

### `camera.door`

Read the door camera or choose the door it shows.

**Result:** `{enabled, doors:[{number, kind, zone, side, pathpoint, position}], manual, auto, shown, renders, picture}`

The door camera of M-0109: a picture-in-picture view from a door of the player's vehicle at the top right of the HUD. It exists only while the setting `camera.door_overlay` is on. `doors` are the pathpoints that the vehicle's passenger cabin names with `[entry]` and `[exit]` (`position` in the vehicle's Bevy frame); `manual` is the door chosen with F4 or with `door` (a number from the list, `0` for none), `auto` the door whose leaf animates, `shown` the door in the picture (`manual` wins), `renders` the pictures rendered so far (30 per second while shown). `-32602` for a door number outside the list.

### `session.spawn`

Spawn a vehicle at an entry point of the map; it becomes the active vehicle when it exists.

**Result:** `{id, queued:true}`; poll `session.vehicles` until `lifecycle` is `alive` (or `failed`, with `error`)

`vehicle` is an absolute `.bus` path (below an allowed read root) or a catalogue id or name. `repaint` defaults to the vehicle's own; `setvars` is an object `{variable: 0|1}` set on the scripts after the repaint's. `entry` is an entry point name (exact, else the only one containing it), `entry_id` its record number; without either the session's start entry is used. The previous active vehicle is parked (parking brake on).

### `session.change_map`

Change the map of the running drive (the Alt menu's Change map): the vehicle and the time are carried over.

**Result:** `{map, map_id, entry_point, entry_defaulted, vehicle, repaint, date, time, weather, dropped, started}`

Leaves the drive and enters it again on another installed map through the loading path of *Load game*; poll `state` until `loading.ready` is true. `map` is a folder name, a friendly name or a part of one that names one map (else `-32602` with the candidates). `entry` is an entry point of the target map (exact, else a part that names one; default: the map's first). The active vehicle is built again at it with its repaint and equipment (`vehicle: carry`, the default; `none` goes on without a vehicle); the session date, time of day and weather are kept; the other session vehicles are dropped and listed in `dropped`. A vehicle that is not in the catalogue answers `1004` (ask for `vehicle: none` or choose another vehicle first). `dry_run` only answers what would happen (`started: false`). `1001` outside a loaded drive.

### `session.switch`

Make a session vehicle the active one (cockpit camera, HUD profile, input and scripts follow).

**Result:** `{ok, active}`

`id` names the vehicle; `step` (1 next, -1 previous) cycles through the living ones like the F8 key. The vehicle that was active keeps its state with the parking brake on. Answers `1004` for an unknown id and `1001` when the vehicle is not in the world.

### `session.despawn`

Remove a session vehicle with its trailers (default: the active one).

**Result:** `{ok, active, removed}`: the vehicle that is active afterwards (`null`: none, free camera) and the entities removed

No confirmation (the confirmation dialog belongs to the pause menu). Despawning the active vehicle makes the most recently spawned other vehicle active, or none.

### `session.respawn`

Put a session vehicle back at its spawn point, at rest (default: the active one).

**Result:** `{ok, id}`

Shows the HUD place label again. Answers `1001` when the vehicle is not in the world.

### `formation.validate`

Check a train formation against the installation (every problem is listed).

**Result:** `{ok, name, cars, issues:[{key, car, message}]}`; `car` is the 0-based car or null

The formation is a saved one (`saved`, its id from `formation.list`), a `.zug` of the installation (`zug`, a path relative to the OMSI root such as `Trains\X10_112_IC-8.zug`) or `cars`, an array of vehicle files or `{vehicle, orientation, powered}` objects (`orientation` `forward` or `backward`; `powered` true or false says whether the car pulls, else the first car and every multi-file car pull and single-file cars behind the head are hauled). `reverse` runs the train the other way round (order reversed, every car turned). Rules: at least two cars; every file exists and is a rail vehicle (`[type]` 2 with `[boogies]`); the first car has a driver camera and is not turned; each car has the coupling point towards a neighbour; no vehicle group names; at most 64 vehicle files in total counting the parts of multi-file cars; a multi-file car (a multiple unit) can be turned, mirrored as a whole. Needs a running session (the installation index).

### `formation.spawn`

Spawn a train formation at an entry point; its first car becomes the active vehicle.

**Result:** `{id, queued:true, cars}`; poll `session.vehicles` until `lifecycle` is `alive` (or `failed`, with `error`)

As `formation.validate` for the formation, then like `session.spawn`: the first car is the vehicle, the other cars are coupled behind it on the track the entry point lies on (within 12 m, else the train stays a road vehicle). A formation with problems answers `-32602` with the problems in the message. The player drives the first car; the other cars are passive. The mass of the whole train acts on the dynamics (resistance and gradient on all cars, brakes on all, traction on the cars that pull).

### `formation.save`

Save a train formation in the user's formation folder.

**Result:** `{ok, id, name, cars}`

The formation is described as for `formation.validate`; `name` renames it. It is written as `<user data>/formations/<id>.toml`, the id made from the name and never one that exists (`name`, `name-2`, ...). `id` with `overwrite: true` replaces that file instead. An empty formation is refused. A formation with open problems can be saved (it can be fixed later).

### `formation.list`

List the saved train formations, newest first.

**Result:** `{dir, formations:[{id, name, cars, modified} | {id, error, modified}]}`; `modified` is Unix seconds

A file that cannot be read is listed with its `error` and kept.

### `formation.delete`

Delete a saved train formation.

**Result:** `{ok, id}`

Answers `1004` for an unknown id and `-32602` for an id that is not a formation id.

### `session.save`

Save the running drive under a name.

**Result:** `{ok, id, name, overwritten}`

Writes a session save (see the session-save page): every registered section is captured and the file is written atomically. Answers `1001` without a loaded drive, `1008` with `data.kind = "exists"` when a save of that name exists and `overwrite` is not set. The save is the same one the pause overlay's *Save game* writes.

### `session.load`

Restore a save (leaves the running drive); poll `session.saves` for the progress.

**Result:** `{ok, started, id, name, unknown_sections, missing_sections, warnings}`

`name` is a save's name or id. The restore goes through the normal loading screen and works from Home and from inside a drive. It is asynchronous: `session.saves` reports `restore.state` (`loading`, `done` with `warnings`, `failed`). Answers `1004` for an unknown save and `1008` for a damaged or newer file (`data.kind` `corrupt`, `newer_format`) or when the map or every vehicle is missing (`data.kind = "missing_content"`, `data.issues`).

### `session.delete_save`

Delete a save and its backup (no confirmation).

**Result:** `{ok, id}`

### `session.rename_save`

Change the name of a save (the file keeps its id).

**Result:** `{ok, id, name}`

### `menu.do`

Drive the in-game vehicle menu (open, spawn, vehicle &lt;text&gt;, repaint &lt;text&gt;, done, ...).

Queues one command of the vehicle menu (see the in-game vehicle menu page); read the result with `menu.state`.

### `vehicle.setvars`

List or change the equipment options (setvars) of the vehicle being planned or driven.

**Result:** without `values`: `{context, repaint, family, count, options: [{name, label, description, kind, min, max, values, value, default, group, declared_in, evidence, range_unverified}], not_offered}`; with `values`: `{context, count, changed: [{name, previous, value}], live}`

The options of the vehicle in a drive (its player vehicle) or, outside a drive, of the vehicle chosen in *Plan a drive* once its repaints are discovered; `1001` when there is none. Each option has a `kind`: `switch` (0 or 1), `choice` (one of `values`), `number` (a whole number from `min` to `max`) or `fraction` (decimal, listed but not settable). `default` is the value of the chosen repaint; `evidence` tells whether repaints set it, the vehicle declares it and the vendor documents it; `range_unverified` marks a documented range that is narrower than what repaints set. Only variables the vehicle declares are offered (`not_offered` names the others). `values` is an object of option name to value: the whole batch is validated and applied all or none (`-32602` with `data.errors: [{key, code, message}]`). In the wizard a change is a choice of the plan, like the equipment dialog; in a drive it is set on the vehicle's scripts at once (`live: true`: a script that reads the variable only at start shows no change) and kept for a respawn. A name the vehicle does not offer is refused unless `allow_unlisted` is true.

### `config.list`

List the saved bus configurations (vehicle, repaint, equipment options) with their check against the installation.

**Result:** `{count, configurations: [{id, name, vehicle, vehicle_name, repaint, setvars, notes, favourite, created, last_used, usable, problems: [{code, message}]}]}`

The configurations are kept in `bus_configurations.json` beside the drive history. Every one is checked when listed: `usable` is false when its vehicle is not installed (any more); `problems` also names a repaint the vehicle no longer offers (`repaint_missing`, the default is used) and equipment values the vehicle's catalogue no longer accepts (`setvar_unknown`, `setvar_rejected`: not applied). `1008` when the file cannot be read (damaged, or from a newer version).

### `config.save`

Save the vehicle, repaint and equipment options of the plan (or of the driven vehicle) under a name.

**Result:** `{id, replaced, configuration}`

Takes the vehicle chosen in *Plan a drive* or, in a drive, the player's vehicle (it must be in the vehicle catalogue). An existing name (ignoring case) is refused with `-32602` unless `replace` is true; a replaced configuration keeps its id, creation time and favourite mark. `1001` when there is no vehicle to save.

### `config.load`

Write a saved configuration into the plan (vehicle, repaint, equipment options).

**Result:** `{id, name, context, vehicle, repaint, setvars, problems}`

`id` is the id or the name. The configuration is checked first: a missing vehicle refuses it (`1008`, `data.problems`), a missing repaint falls back to the default and values the catalogue no longer accepts are left out; both are listed in `problems`. In a running drive nothing is changed (`1007`): configurations are loaded in *Plan a drive*. `1004` for an unknown configuration.

### `config.spawn`

Build a saved configuration as a new vehicle of the running drive.

**Result:** `{id, queued:true, configuration, name, problems}`; poll `session.vehicles` until `lifecycle` is `alive` (or `failed`)

As `session.spawn` with the vehicle, repaint and options of the configuration (`id` is its id or name), after the same check as `config.load`: a missing vehicle or `.bus` file refuses it (`1008`, `data.problems`), a missing repaint falls back to the default, values the catalogue no longer accepts are left out and listed in `problems`. The new vehicle becomes the active one and the previous one is parked; `entry`/`entry_id` as in `session.spawn`. `1001` outside a loaded drive, `1004` for an unknown configuration.

### `config.delete`

Delete a saved configuration.

**Result:** `{ok, id}`

### `config.update`

Rename a saved configuration, change its notes or its favourite mark.

**Result:** `{configuration}`

At least one of `name`, `notes`, `favourite`. A name that is empty or taken by another configuration is refused with `-32602`.

### `config.duplicate`

Copy a saved configuration under a new name.

**Result:** `{id, configuration}`

### `audio.tone`

Play a sine tone (a test signal for the sound output).

**Result:** `{entity, bus, loop}`

`frequency` 20 to 20000 Hz (440), `seconds` 0.05 to 30 (1). Without `at` the tone plays everywhere; with `at: [x, y, z]` (Bevy metres) it is a source that fades from `reference` (5 m) to `max` (120 m) and is panned. The sound is kept after it ended, so `audio.state` can show it; `audio.stop` removes it. `1001` before the engine runs.

### `audio.play`

Play a sound file of the installation through the engine.

**Result:** `{entity, bus, loop}`

`file` is a `.wav` below an allowed read root (`1002` outside, `1004` missing). It is decoded once and shared (`audio.state.clips_cached`). A file that is not RIFF/WAVE or has an unusual sample format answers `1008` with the reason (five MPEG files in the corpus are named `.wav`). `at`, `reference`, `max` as for `audio.tone`.

### `audio.stop`

Stop and remove the sounds `audio.tone` and `audio.play` made.

**Result:** `{stopped}`

## Group `settings`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| `settings.list` | `category?` (string) | 1.0 | List the settings with their values (optionally one category) |
| `settings.get` | `key` (string) | 1.0 | Value, default and options of one setting |
| [`settings.set`](#settingsset) | `key` (string)<br>`value` (string)<br>`confirm?` (boolean) | 1.0 | Change a setting (validated like the settings page, saved) |
| [`settings.reset`](#settingsreset) | `key` (string)<br>`confirm?` (boolean) | 1.0 | Restore the default of a setting |
| [`settings.confirm`](#settingsconfirm) | - | 1.7 | Keep the risky setting change that waits and save it |
| [`settings.revert`](#settingsrevert) | - | 1.7 | Undo the risky setting change that waits |
| [`settings.set_many`](#settingsset_many) | `values` (json) | 1.7 | Change several settings at once, all or none |
| [`settings.diff`](#settingsdiff) | - | 1.7 | The settings that differ from their defaults |
| [`settings.export`](#settingsexport) | `all?` (boolean) | 1.7 | A profile of the changed settings, to import later |
| [`settings.import`](#settingsimport) | `settings` (json)<br>`reset_missing?` (boolean) | 1.7 | Apply a settings profile, all or none |
| [`hardware.profile`](#hardwareprofile) | `redetect?` (boolean) | 1.2 | The detected hardware profile and the recommended defaults (local, no identifiers) |

### `settings.set`

Change a setting (validated like the settings page, saved).

**Result:** the setting as `settings.get` shows it, plus `previous`, `applied` (it changed), `needs_restart` and, while it waits, `pending_confirm` and `confirm_within_s`

Applies and saves at once. A setting that the page protects with a 15 s prompt (`apply.revert_after_s` in `settings.list`: window mode, resolution) can instead be tried with `confirm: false`: it is applied but not saved and waits for `settings.confirm` (keep and save) or `settings.revert` (restore); after its time it is reverted by itself, like the page's prompt. Only one such change waits at a time (`1006 BUSY` otherwise); `confirm: false` on any other setting changes nothing about how it is applied. `needs_restart` is true when the change only takes effect after a restart.

### `settings.reset`

Restore the default of a setting.

Like `settings.set` with the default value, including `confirm`.

### `settings.confirm`

Keep the risky setting change that waits and save it.

**Result:** `{key, value, confirmed: true}`

Finishes a `settings.set` or `settings.reset` with `confirm: false`. `1001 NOT_READY` when nothing waits.

### `settings.revert`

Undo the risky setting change that waits.

**Result:** `{key, value, reverted: true}`

Restores the value from before a `settings.set` or `settings.reset` with `confirm: false`. `1001 NOT_READY` when nothing waits.

### `settings.set_many`

Change several settings at once, all or none.

**Result:** `{count, changed: [{key, previous, value, needs_restart}], unchanged, needs_restart: [keys]}`

`values` is an object of setting key to value. The whole batch is validated first (unknown or unavailable keys, bad values, key bindings that clash inside the batch); if anything is refused nothing is applied and the answer is `-32602` with `data.errors: [{key, code, message}]` listing every refusal. Settings with a revert prompt are applied at once, without the `confirm` wait. Saved once at the end; at most 1000 entries.

### `settings.diff`

The settings that differ from their defaults.

**Result:** `{count, differences: [{key, value, default}]}`

Visible settings only (internal state such as the last map is left out).

### `settings.export`

A profile of the changed settings, to import later.

**Result:** `{format: "opts-settings", version: 1, count, settings: {key: value}}`

Without `all` only the settings that differ from the defaults; with `all: true` every visible setting. Hand the result (its `settings` object) to `settings.import`.

### `settings.import`

Apply a settings profile, all or none.

**Result:** as `settings.set_many`

`settings` is the object `settings.export` returned (`values` is accepted as a synonym). Validated and applied like `settings.set_many`. With `reset_missing: true` the settings that differ from the defaults and are not in the profile go back to their defaults, so the result is exactly the profile.

### `hardware.profile`

The detected hardware profile and the recommended defaults (local, no identifiers).

**Result:** { probing, stored, allow_in_reports, profile, recommendations }

`profile` is the versioned `opts-hardware` document (every fact is `{known: v}` or `{unknown: reason}`; `null` until the first probe finished). `redetect: true` starts a new probe in the background; poll again. Nothing is stored by this call. See the hardware probe charter.

## Group `l10n`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| [`l10n.languages`](#l10nlanguages) | - | 1.1 | List the languages (built in and installed packs) with completeness, authors and problems |
| [`l10n.set`](#l10nset) | `code` (string) | 1.1 | Switch the interface language (saved like a setting) |
| [`l10n.reload`](#l10nreload) | - | 1.1 | Scan the language folder again (new or edited packs) |
| [`l10n.report`](#l10nreport) | `code?` (string) | 1.1 | Completeness and every finding of one language |

### `l10n.languages`

List the languages (built in and installed packs) with completeness, authors and problems.

**Result:** `{current, packs_dir, languages:[{code, name, native_name, direction, builtin, from_pack, authors, version, total, translated, copied, missing, percent, errors, warnings, outdated, missing_fonts, selectable, current, dir}], problems:[{folder, issues}]}`

`translated` counts messages that are really translated, `copied` those that are still the English text, `missing` those that show English. `selectable` is false for right-to-left packs. `problems` lists pack folders that cannot be used (no valid `pack.json`). Packs are files in `<data dir>/lang/<code>/`, see the localisation packs page.

### `l10n.set`

Switch the interface language (saved like a setting).

**Result:** `{current, previous}`

`1004` for a language that is not installed, `1007` for a right-to-left pack (not supported yet). Stores `interface.language` and updates every text at once.

### `l10n.reload`

Scan the language folder again (new or edited packs).

**Result:** the `l10n.languages` result plus `{usable, broken, errors}`

The same as the *Reload languages* button. A pack that cannot be used or a file with errors never fails the call: read `problems` and `l10n.report`. If the active language is gone the game shows English.

### `l10n.report`

Completeness and every finding of one language.

**Result:** `{language, missing_keys, copied_keys, issues:[{severity, code, file, line, key, message}], issue_lines}`

Without `code` the current language. `issue_lines` are the same findings as `file:line: error[code]: message` text, as `opts-tools l10n check` prints them.

## Group `reload`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| `reload.asset` | `path` (string) | 1.0 | Force a reload of one file |
| `reload.scene` | - | 1.0 | Reload every resident tile and the vehicle |
| `reload.config` | `path` (string) | 1.0 | Reload one configuration file |
| `watch.add` | `path` (string) | 1.0 | Watch another path |
| `watch.remove` | `path` (string) | 1.0 | Stop watching a path |
| `watch.list` | - | 1.0 | List watched paths |

## Group `debug`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| `debug.wireframe` | `enabled` (boolean) | 1.0 | Toggle wireframe rendering |
| `debug.normals` | `enabled` (boolean) | 1.0 | Toggle normal visualisation |
| `debug.stats` | - | 1.0 | Render and loader statistics |
| `hud.alert` *(debug)* | `severity?` (string = info \| warning \| critical)<br>`clear?` (boolean) | 1.0 | Raise or clear a test alert on the HUD |
| `hud.toast` *(debug)* | `message` (string)<br>`details?` (string) | 1.0 | Show a test toast |

## Group `output`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| [`screenshot`](#screenshot) | `path?` (string)<br>`name?` (string)<br>`format?` (string = png \| jpeg)<br>`quality?` (integer)<br>`include_metadata?` (boolean) | 1.0 | Save a screenshot of the application window, with metadata |
| `report` | `text` (string) | 1.0 | File a report (screenshot + text) |
| [`snapshot`](#snapshot) | `path?` (string)<br>`screenshot?` (boolean)<br>`warnings?` (integer) | 1.1 | One call that returns everything an agent needs to judge the state, optionally with a screenshot |

### `screenshot`

Save a screenshot of the application window, with metadata.

**Result:** `{path, width, height, format, bytes, sim_time, frame, metadata_path?, metadata?}`

Captures the window's own render target: never the desktop, other windows or the cursor, in windowed and fullscreen mode alike. Files go below the captures directory (`<user data dir>/opts/captures`). `name` is a relative file name or subpath there (preferred); `path` is an explicit path that must lie inside the captures directory; both together are `-32602`, neither generates `{datetime}_{session_id}_{counter}.png`. `format` is `png` (default) or `jpeg` (with `quality` 1-100, default 90). With `include_metadata` (default true) a sidecar `<file>.json` is written and the metadata is returned: simulation time (`Time<Virtual>`, frozen while paused), render frame, fixed tick, map, camera mode, vehicle, window mode and size, versions. `1001` without a renderer or while the window is minimized, `1002` for a name or path outside the captures directory, `1005` when the frame does not arrive in 5 s. Extended in 1.5; the 1.0 call `{path}` still works for paths inside the captures directory.

### `snapshot`

One call that returns everything an agent needs to judge the state, optionally with a screenshot.

**Result:** `{state, screen, drive?, hud?, camera, sim, warnings:[log lines], screenshot?}`

`path` writes the JSON to a file below the reports directory (or an allowed directory). `screenshot` also saves a PNG (the loading screen included) and returns its path. `warnings` is the number of recent `warn`/`error` log lines (default 20).

## Group `capture`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| [`capture.burst`](#captureburst) | `count` (integer)<br>`interval_ms` (integer)<br>`format?` (string = png \| jpeg)<br>`quality?` (integer)<br>`dir?` (string)<br>`metadata?` (boolean) | 1.5 | Capture N frames at a fixed interval as a PNG sequence |
| [`capture.record.start`](#capturerecordstart) | `name?` (string)<br>`format?` (string = av1 \| png_sequence \| h264)<br>`fps?` (integer)<br>`duration_ms?` (integer)<br>`max_size_mb?` (integer)<br>`quality?` (integer)<br>`scale?` (number) | 1.5 | Start a bounded recording of the application window |
| [`capture.record.stop`](#capturerecordstop) | `recording_id` (string) | 1.5 | Stop a recording and return its result |
| [`capture.record.status`](#capturerecordstatus) | `recording_id` (string) | 1.5 | Progress of a recording |
| [`capture.prune`](#captureprune) | `older_than_days` (integer)<br>`dry_run?` (boolean) | 1.5 | Delete old captures written by this subsystem (dry run unless told otherwise) |

### `capture.burst`

Capture N frames at a fixed interval as a PNG sequence.

**Result:** `{dir, paths:[..], frames:[{index, path, sim_time, frame, wall_ms}], dropped, manifest}`

1-300 frames, `interval_ms` 16-60000, and `count` x `interval_ms` at most 90 000 ms. Frames are written to `dir` (a relative name below the captures directory; default `{datetime}_{session_id}_{counter}_burst`) as `0001.png`, … with a `manifest.json` carrying the shared metadata and one entry per frame. Replies when the last frame is written; a frame the renderer does not deliver within 2 s counts in `dropped`. Only one capture job (burst or recording) runs at a time (`1006`); `1001` without a renderer or while minimized, `1002` for a `dir` outside the captures directory.

### `capture.record.start`

Start a bounded recording of the application window.

**Result:** `{recording_id, path, format, fps, effective_limits:{duration_ms, max_size_mb, min_free_disk_mb}}`

Returns at once; the recording runs in the background and is off unless started. `format` is `av1` (default; AV1 in an `.ivf` file, encoded by rav1e) or `png_sequence` (a directory of PNGs, ends early at the size cap); `h264` needs an optional build feature. `fps` 1-60 (default 30), `duration_ms` default 60 000 (the `opts-cli` default is 30 000) up to 30 min, `max_size_mb` default 512 up to 2048, `quality` 1-100, `scale` 0.25-1, rounded to a whole reduction (1, 1/2, 1/3, 1/4) and **0.5 by default**: the encoder is a pure-Rust AV1 build without assembly and measures about 9 fps at 1920x1013, 23 fps at half and 29 fps at a quarter size; frames it cannot keep up with are dropped (`dropped`, and `timing_ms` in `status` shows where the time goes). It also stops below 1 GiB free disk space, when the frame rate stays under 30 for 5 s, or when the renderer stops delivering frames. `name` is a relative stem below the captures directory. A sidecar `<file>.json` lists every frame with its simulation time. `1006` while another job runs, `1008` when the disk is nearly full, `1001` without a renderer.

### `capture.record.stop`

Stop a recording and return its result.

**Result:** `{recording_id, state, path, format, duration_ms, frame_count, dropped, bytes, stopped_by}`

`stopped_by` is `request`, `duration`, `size`, `disk`, `degraded`, `encoder_error` or `window`. Waits (at most 10 s) until the file is closed; stopping a finished recording returns its stored result. `1004` for an unknown id.

### `capture.record.status`

Progress of a recording.

**Result:** `{recording_id, state, frame_count, elapsed_ms, dropped, bytes, fps_measured, degraded, stopped_by?}`

`state` is `recording`, `finishing`, `finished` or `failed`; a finished recording returns its final result (the last 8 are kept). Never waits.

### `capture.prune`

Delete old captures written by this subsystem (dry run unless told otherwise).

**Result:** `{dry_run, items:[{path, bytes}], bytes}`

Only files with a metadata sidecar and burst or recording directories with a `manifest.json` are candidates, judged by their modification time; other files in the captures directory are never touched. `dry_run` defaults to true.

## Group `drive`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| [`drive.controls`](#drivecontrols) | - | 1.1 | List every vehicle control by name with its kind, range and current value |
| [`drive.set`](#driveset) | `control` (string)<br>`value` (json)<br>`hold_ms?` (integer) | 1.1 | Set a vehicle control by name, optionally held for a duration |
| `drive.release` | `control?` (string) | 1.1 | Release held controls (one by name, or all) |
| [`drive.trigger`](#drivetrigger) | `name` (string)<br>`hold_ms?` (integer) | 1.1 | Raise a vehicle script trigger (doors, lights, indicators, horn, ...) |
| [`vehicle.key`](#vehiclekey) | `action?` (string)<br>`key?` (string)<br>`hold_ms?` (integer) | 1.1 | Press a vehicle key (engine start, lights, indicators, doors, ...) by action or by key |
| [`vehicle.keys`](#vehiclekeys) | - | 1.1 | List the vehicle key actions with their keys, defaults and triggers |
| [`drive.state`](#drivestate) | - | 1.1 | Speed, position, heading, gear, pedals, parking brake, distance, camera and HUD state |
| [`vehicle.teleport`](#vehicleteleport) *(debug)* | `x` (number)<br>`y` (number)<br>`heading_deg?` (number) | 1.1 | Place the player vehicle at an absolute OMSI position and heading, standing still |
| [`vehicle.reset`](#vehiclereset) *(debug)* | - | 1.1 | Put the player vehicle back where the session placed it, standing still |
| [`vehicle.dynamics`](#vehicledynamics) | - | 1.10 | What the player's vehicle states about its dynamics (mass, centre of gravity, inertia, rolling resistance, axles) with the source of each value, and the live axle loads |
| [`profiles.list`](#profileslist) | - | 1.14 | The map profiles of the planned drive's map: the profile that applies, the saved ones, the presets, the load profile and the advice |
| [`profiles.advice`](#profilesadvice) | - | 1.14 | The load profile of the planned drive's map and the recommendations that follow from it |
| [`profiles.apply`](#profilesapply) | `profile` (string)<br>`make_default?` (boolean) | 1.14 | Use a profile for the next drive of the planned map (Apply Selected) |
| [`profiles.save`](#profilessave) | `name` (string)<br>`chrono?` (boolean) | 1.14 | Create a profile from the current values of the map-eligible settings (Create New from Current) |
| [`profiles.delete`](#profilesdelete) | `chrono?` (boolean) | 1.14 | Delete the map's profile (the Chrono entry's with `chrono: true`) |
| [`profiles.status`](#profilesstatus) | - | 1.14 | Which profile the running drive uses |
| [`bus.routes`](#busroutes) | `limit?` (integer) | 1.12 | List the lines and courses of the depot (.hof) of the player's vehicle |
| [`bus.get_route`](#busget_route) | - | 1.12 | The route the player's vehicle has selected, and whether its IBIS is still being keyed |
| [`bus.set_route`](#busset_route) | `route_code?` (integer)<br>`line?` (integer)<br>`course?` (integer)<br>`clear?` (boolean) | 1.12 | Select a route for the player's vehicle the way a driver does: the IBIS keys, then the destination to the displays |
| [`bus.timetables`](#bustimetables) | `line?` (string)<br>`tour?` (integer)<br>`limit?` (integer)<br>`all?` (boolean) | 1.13 | List the map's timetables: line files, their tours and the runs of a tour |
| [`bus.set_timetable`](#busset_timetable) | `line?` (string)<br>`tour?` (integer)<br>`run?` (integer)<br>`stops?` (integer)<br>`clear?` (boolean) | 1.13 | Follow a run of the map's timetable: its stops and times answer the vehicle's timetable callbacks, the depot's matching route is keyed on the IBIS |
| [`bus.set_stop`](#busset_stop) | `index` (integer) | 1.13 | Set the stop of the route or timetable the vehicle is at (0-based) |

### `drive.controls`

List every vehicle control by name with its kind, range and current value.

**Result:** `{vehicle, controls:[{name, kind, range?, value, bound?}], triggers:[{name, key, held}]}`

`throttle`, `brake`, `steer`, `gear`, `parking_brake` are built in; `triggers` are the script triggers the vehicle's keyboard bindings can raise (doors, lights, indicators, horn, ... named as in the vehicle's scripts), usable with `drive.trigger` and `drive.set`.

### `drive.set`

Set a vehicle control by name, optionally held for a duration.

**Result:** `{ok, control, value, hold_ms?}`

`throttle`/`brake` take 0..1, `steer` -1 (left)..1 (right), `gear` `drive|neutral|reverse`, `parking_brake` true/false/`toggle`; any other name is raised as a script trigger (`horn`, `bus_doorfront0`, ...). Analog values stay set for `hold_ms` of **simulation time** (while the sim is paused the time does not run) or, without it, until set again or released with `drive.release`. The values reach the vehicle through the same system as the keyboard, so they are ignored while a dialog is open or the vehicle does not take input (`1009`).

### `drive.trigger`

Raise a vehicle script trigger (doors, lights, indicators, horn, ...).

**Result:** `{ok, name, handled, held_ms?}`

Raises `<name>`; for a binding that is held (like the horn) `<name>_off` follows after `hold_ms` of simulation time (default 200). `handled` says whether a script of the vehicle reacted. An unbound name is raised as is. Gated like a key press.

### `vehicle.key`

Press a vehicle key (engine start, lights, indicators, doors, ...) by action or by key.

**Result:** `{ok, raised:[{ok, name, action, raised, handled, held_ms?}]}`

Raises the script trigger that the key press raises, whatever the player has bound. `action` is the short name of a rebindable action (`battery`, `engine_start`, `lights`, `high_beam`, `indicator_left`, `hazard`, `door_1`, `interior_upper`, ... all listed by `vehicle.keys`) or a bound trigger name; `key` is a key chord (`L`, `Ctrl+Numpad1`) and raises every trigger bound to it, so a test can also check the binding itself. Held actions (battery, engine start, indicators, horn) get their `_off` after `hold_ms` of simulation time (default 200); give a longer `hold_ms` to hold the engine start until the engine runs. `handled` says whether a script of the vehicle reacted. Gated like a key press (`1001` without a player vehicle, `1009` while a dialog is open).

### `vehicle.keys`

List the vehicle key actions with their keys, defaults and triggers.

**Result:** `{actions:[{action, setting, trigger, key, default, held}], other:[{trigger, key, held}]}`

`actions` are the rebindable ones (setting `controls.vehicle.<action>`, change with `settings.set`; a key that another action or OPTS itself uses is refused); `other` are the remaining `keyboard.cfg` vehicle bindings (IBIS, destination blinds, cash desk).

### `drive.state`

Speed, position, heading, gear, pedals, parking brake, distance, camera and HUD state.

**Result:** `{speed_kmh, signed_speed_kmh, x, y, z, heading_deg, gear, throttle, brake, steer, parking_brake, rpm?, distance_m, camera, paused, time_scale, sim_time, alert?, route, stops}`

`distance_m` is the path length driven since the session (or the last `vehicle.reset`) started. `route` and `stops` are `null`: the timetable is not implemented yet (`not available yet`); `alert` is the HUD alert on screen, if any.

### `vehicle.teleport`

Place the player vehicle at an absolute OMSI position and heading, standing still.

A debug command: it moves the vehicle directly (speed zero, controls neutral) without any gate. Coordinates are absolute OMSI metres as in `vehicle.get`.

### `vehicle.reset`

Put the player vehicle back where the session placed it, standing still.

A debug command: restores the start position, zeroes speed and the controls and the distance counter.

### `vehicle.dynamics`

What the player's vehicle states about its dynamics (mass, centre of gravity, inertia, rolling resistance, axles) with the source of each value, and the live axle loads.

**Result:** `{spec: {mass_kg, cg_height_m, yaw_inertia_kg_m2, pitch_inertia_kg_m2, roll_inertia_kg_m2, rolling_coefficient}, axles: [{index, longitudinal_m, track_width_m, wheel_diameter_m, spring_n_per_m, damping_n_s_per_m, max_force_n, driven, steered, static_load_n, load_n: [left, right], deflection_m, deflection_speed_m_s, compression_m: [left, right], brake_force_n, brake_limited}], state: {speed_m_s, head_mass_kg, consist_mass_kg, units, wheelbase_m}, body: {heave_m, pitch_rad, roll_rad}, brakes: {pressure, apply_tau_s, force_n}}`; every `spec` entry is `{value, source}` with `source` one of `file`, `estimate`, `default`; since 1.11 the answer also has `pitch_inertia_kg_m2`, `roll_inertia_kg_m2`, `compression_m`, `brake_force_n`, `brake_limited`, `body` and `brakes`

`1001` without a player vehicle. `file` values are read from the vehicle's `.bus` (`[mass]`, `[schwerpunkt]`, `[momentofintertia]` or lines 2 to 4 of `[mass]`, `[rollwiderstand]`); `estimate` is the yaw inertia of a box with the bounding box's width and length; `default` is the generic transit bus, used because the file states nothing usable. The rolling coefficient is the file's rolling resistance (a force) over the stated weight. Axle values are those the physics uses (the `.bus` axle block, with bus defaults for what is missing); `static_load_n` follows from the axle positions, the mass and the centre of gravity.

### `profiles.list`

The map profiles of the planned drive's map: the profile that applies, the saved ones, the presets, the load profile and the advice.

**Result:** `{map, chrono, chrono_entries, resolved: {source, profile, dropped}, picked, saved: [{chrono, name, created, based_on, settings}], presets: [...], load: {tiles, objects, splines, ai_pool_vehicles, area_km2, estimated, counting, avg_frame_time_ms, p95_frame_time_ms}, advice: [{id, severity, preset, suggested}]}`

`1001` without a map in the plan. `chrono` is the newest Chrono entry active on the plan's date (`default` when none is); `resolved.source` is `chrono`, `map` or `global` (the fallback chain: this Chrono entry, the map's default, your global settings). The load profile is counted in the background the first time the map is asked for: `counting` is true until the full count is in, `estimated` while only a sample is. `avg_frame_time_ms` comes from the last session of this key on this computer.

### `profiles.advice`

The load profile of the planned drive's map and the recommendations that follow from it.

**Result:** `{load, advice}` as in `profiles.list`

`advice[].id` is one of `geometry-above-typical`, `geometry-high`, `light-map`, `many-splines`, `high-ai-load`, `ran-slowly`, `ran-smoothly`, `not-played-yet`; `suggested` lists settings to change as `{key, value}` and `preset` names the preset that carries the suggestion.

### `profiles.apply`

Use a profile for the next drive of the planned map (Apply Selected).

**Result:** the answer of `profiles.list`

`profile` is `none` (your global settings), `preset:<id>` (`performance`, `balanced`, `quality`) or the name of a saved profile of the map. The profile is applied when the drive starts and the global settings return when it ends; the settings file never holds profile values. With `make_default` the profile is also written as the profile of this key. `-32602` for a profile that does not exist, `1001` without a map.

### `profiles.save`

Create a profile from the current values of the map-eligible settings (Create New from Current).

**Result:** the answer of `profiles.list`

Saved as the map's default profile, or with `chrono: true` as the profile of the newest active Chrono entry. Settings nothing is stored for are saved with their default. An existing profile of the same key is replaced.

### `profiles.delete`

Delete the map's profile (the Chrono entry's with `chrono: true`).

**Result:** the answer of `profiles.list` plus `removed` (false when there was none)

Presets cannot be deleted.

### `profiles.status`

Which profile the running drive uses.

**Result:** `{active, map, chrono, profile, source, keys, overlaid}`

`keys` are the settings the profile overrides, `overlaid` the keys the settings currently hold as session values (the global values return when the drive ends).

### `bus.routes`

List the lines and courses of the depot (.hof) of the player's vehicle.

**Result:** `{depot, count, routes: [{route_code, line, course, line_string, route_string, terminus_code, terminus}], selected}`; `selected` is the route code or `null`

`1001` without a player vehicle. `route_code` is `line * 100 + course` as keyed on the IBIS (`13001`: line 130, course 01); `terminus` is the first string of the destination's row in the depot. The depot is the `.hof` the game chose for the vehicle and the map (the vehicle's folder first); a vehicle without a depot lists nothing.

### `bus.get_route`

The route the player's vehicle has selected, and whether its IBIS is still being keyed.

**Result:** `{depot, selected: {route_code, line, course, line_string, route_string, terminus_code, terminus} or null, stop_index, typing}`

`1001` without a player vehicle. `typing` is true while the keys of a selection are still being pressed (one key per physics step).

### `bus.set_route`

Select a route for the player's vehicle the way a driver does: the IBIS keys, then the destination to the displays.

**Result:** `{selected: {...} or null, keypad, typing}`; without a keypad also `note`

Give `route_code`, or `line` and `course`, or `clear` (the driver cancels the route). The selection changes the timetable callbacks at once and the vehicle's own scripts take it: the keys of the MAN-type IBIS (`ibis_setmode_linie_kurs`, the digits, `ibis_eingabe`, ...) are pressed one per physics step, and the target (`AI_Target_Index`, `SetLineTo`, the trigger `ai_scheduled_settarget`) is offered to the destination displays until `target_index_int` shows it (30 s at most). Vehicles with another IBIS get the second part only (`keypad: false`). `-32602` for a route the depot does not have, `1001` without a player vehicle.

### `bus.timetables`

List the map's timetables: line files, their tours and the runs of a tour.

**Result:** without `line`: `{available, lines: [{file, priority, user_allowed, tours, runs}]}`; with `line`: `{available, line, tours: [{tour, name, vehicle_group, runs}]}`; with `line` and `tour`: `{available, line, tour, name, runs: [{run, trip, title, line, start, stops}]}`

The map's `TTData` folder: `.ttl` line files with their tours (`[newtour]`) and the runs of each tour (`[addtrip]`: a trip file, a timing profile and the start time). Without `line` only the line files the player may drive (`[userallowed]`) are listed, all of them with `all: true`. `available: false` when the map has no timetable files. `1001` without a player vehicle, `-32602` for an unknown line or tour.

### `bus.set_timetable`

Follow a run of the map's timetable: its stops and times answer the vehicle's timetable callbacks, the depot's matching route is keyed on the IBIS.

**Result:** `{schedule: {line_file, tour, trip, title, line, start, stop_count, stops: [{station, name, arrival, departure, distance_m}]}, route_code, typing}`; `distance_m` is the distance along the run from its first stop (`null` unless `StnLinks.cfg` links every pair of stations); `route_code` is the depot route selected for the run (`null` when the depot has none for its line and destination)

`line` is the file name of a line file without extension, `tour` and `run` count from 0 (default 0). The run's trip gives the stops (`Busstops.cfg` names) and, with its timing profile, the scheduled arrival and departure of each; times without a manual entry in the profile are interpolated by station index. `GetTTDelay` is the simulation clock minus the arrival at the current stop (`bus.set_stop`). `stops` limits how many stops the answer lists (default 5). `clear` stops following. `-32602` for a run the map does not have or a map without timetable files.

### `bus.set_stop`

Set the stop of the route or timetable the vehicle is at (0-based).

**Result:** `{stop_index, stop_count}`

What `GetTTBusstopIndex` answers and which stop the delay is measured at. The game does not detect bus stops by itself yet, so the stop is set here (or by a script test). `-32602` beyond the stops of the selected route or timetable.

## Group `trace`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| [`trace.set`](#traceset) | `mode?` (string)<br>`tiles?` (json)<br>`kinds?` (json)<br>`file?` (string)<br>`radius?` (number)<br>`center?` (json)<br>`size_cap_mb?` (integer)<br>`overlay?` (boolean)<br>`reset?` (boolean) | 1.4 | Scene trace: set the mode and filters, or read the status |
| [`trace.pick`](#tracepick) | `x?` (number)<br>`y?` (number)<br>`limit?` (integer) | 1.4 | Scene trace: what is under a screen pixel or the crosshair |
| [`trace.tile`](#tracetile) | `tile` (json)<br>`kind?` (string)<br>`file?` (string)<br>`stage?` (string)<br>`issues_only?` (boolean)<br>`full?` (boolean)<br>`limit?` (integer) | 1.4 | Scene trace: counts and records of one tile |
| [`trace.find`](#tracefind) | `file?` (string)<br>`kind?` (string)<br>`stage?` (string)<br>`tile?` (json)<br>`issues_only?` (boolean)<br>`full?` (boolean)<br>`limit?` (integer) | 1.4 | Scene trace: records by file pattern, kind, decode stage or tile |
| [`trace.region`](#traceregion) | `radius` (number)<br>`x?` (number)<br>`z?` (number)<br>`kind?` (string)<br>`file?` (string)<br>`stage?` (string)<br>`issues_only?` (boolean)<br>`full?` (boolean)<br>`limit?` (integer) | 1.4 | Scene trace: records within a radius of a ground point |
| [`trace.export`](#traceexport) | `path` (string)<br>`file?` (string)<br>`kind?` (string)<br>`tile?` (json)<br>`stage?` (string)<br>`issues_only?` (boolean)<br>`limit?` (integer) | 1.4 | Scene trace: write matching records as JSON Lines |

### `trace.set`

Scene trace: set the mode and filters, or read the status.

**Result:** `{mode, file, bytes, truncated, records, in_memory, issues, map, tiles, filters, overlay}`

Mode `off` (default, free), `summary` (counts and issues) or `full` (every placed thing). Records are written as JSON Lines next to the run logs; switch the trace on before loading the map (meshes loaded earlier carry no decode information). `tiles` is `[[x, y], ...]`, `kinds` a list of `scenery`, `tree`, `attachment`, `parked`, `spline`, `terrain`, `traffic`, `human`, `vehicle`, `file` a substring or `*` pattern, `radius` metres around `center` (`[x, z]` in Bevy world coordinates, the camera by default). Without arguments the call is the status query. See `docs/src/architecture/scene-trace.md`.

### `trace.pick`

Scene trace: what is under a screen pixel or the crosshair.

**Result:** `{hit, parents, candidates, ray, ground, pixel, view}`; `hit` is the full record of the nearest object on the ray

`x`, `y` are physical pixels as in a `screenshot`; without them the centre of the view (the crosshair) is used. `hit.file` is the `.sco`, `hit.meshes[*].file` the `.o3d` with its decode `stage` and texture-coordinate quality. `candidates` lists the next objects on the ray; `ground.tile` names the tile under the pixel. Needs the trace on (`1001` otherwise).

### `trace.tile`

Scene trace: counts and records of one tile.

**Result:** `{tile, summary, count, records}`

### `trace.find`

Scene trace: records by file pattern, kind, decode stage or tile.

**Result:** `{count, records}`

At least one criterion is required. `file` is matched against the placed thing's file and its mesh files (substring or `*`). `stage` is one of `plain`, `twin`, `decode-cache`, `decode-background`, `decoded-now`, `decode-deferred`, `rejected`, `extent`, `read-failed`, `parse-failed`, `hidden`, `cached`.

### `trace.region`

Scene trace: records within a radius of a ground point.

**Result:** `{center, radius, count, records}`

### `trace.export`

Scene trace: write matching records as JSON Lines.

**Result:** `{path, records}`

Writes the records of the in-memory index (at most 150,000) to a path the control server may write to (`--cli-allow-dir`). The run file next to the logs holds everything.
