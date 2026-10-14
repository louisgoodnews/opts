<!-- Generated from the command table of opts-cli (crates/opts-cli/src/commands.rs) by the test `reference_page_is_up_to_date`; regenerate with `UPDATE_CONTROL_DOC=1 cargo test -p opts-cli reference`. -->

# Control Protocol Reference

Protocol version **1.2.0**. Every command below is declared once in the command table; the same table answers `help <command>` and `commands` at runtime, drives the `opts-cli` argument parser and generates this page. Transport, message format, gating and worked sessions are in [the architecture page](../architecture/control-protocol.md).

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
| `state` | - | 1.0 | App state, camera mode, fps, frame |
| `scene` | - | 1.0 | Loaded map, tiles, entity and asset counts |
| `camera.get` | - | 1.0 | Camera mode and transform |
| `sim.get` | - | 1.0 | Pause state, simulation time, time scale |
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
| `menu.state` | - | 1.0 | Page, choices and free entry points of the in-game vehicle menu |

### `vehicle.get`

Position, heading and speed of the player vehicle (absolute OMSI coordinates).

**Result:** `{x, y, z, heading_deg, speed_kmh}`

### `session.vehicles`

The session's user vehicles: id, name, repaint, spawn entry point, lifecycle, active marker.

**Result:** `{active, count, vehicles:[{id, name, catalogue_id, repaint, setvars, spawn_entry_name, spawn_point, lifecycle (spawning|alive|despawned|failed), despawn_reason, error, active, trailers}], place_label}`

The registry of the vehicles the user spawned (the wizard's choice, the pause menu, `session.spawn`); parked and AI vehicles are not listed. Despawned vehicles stay in the list as history. `place_label` is what the HUD shows for the active vehicle (the entry point it was spawned at) or `null`.

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
| `sim.set_time` | `seconds` (number) | 1.0 | Set the session clock |

### `sim.pause`

Pause the simulation.

Pauses the simulation clock (`Time<Virtual>`) directly, not through the pause overlay; the overlay is reached with `ui.action menu` or `ui.key Escape`.

## Group `scene`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| [`scene.load_map`](#sceneload_map) *(debug)* | `path` (string)<br>`entry?` (string) | 1.0 | Start a session on a map |
| [`scene.unload`](#sceneunload) *(debug)* | - | 1.0 | Return to the main menu |
| `camera.set_mode` | `mode` (string)<br>`sub?` (integer) | 1.0 | Switch camera mode |
| `camera.teleport` *(debug)* | `x` (number)<br>`y` (number)<br>`z` (number)<br>`yaw?` (number)<br>`pitch?` (number) | 1.0 | Move the camera (OMSI coordinates) |
| `render.look` | `look?` (string) | 1.0 | Get or set the colour look (vivid, natural, bevy) |
| [`vehicle.set`](#vehicleset) *(debug)* | `path` (string) | 1.0 | Replace the player vehicle |
| `entity.spawn` | `asset` (string)<br>`transform?` (json) | 1.0 | Spawn an asset |
| `entity.despawn` | `id` (integer) | 1.0 | Despawn an entity |
| `entity.set_component` | `id` (integer)<br>`type` (string)<br>`value` (json) | 1.0 | Set a component value |
| [`camera.look`](#cameralook) | `yaw?` (number)<br>`pitch?` (number)<br>`dx?` (number)<br>`dy?` (number) | 1.1 | Turn the camera: absolute yaw/pitch (free roam) or relative look (cockpit, external) |
| [`camera.move`](#cameramove) | `forward?` (number)<br>`right?` (number)<br>`up?` (number)<br>`duration_ms?` (integer) | 1.1 | Move the free camera forward/right/up for a duration, like the movement keys |
| [`camera.viewpoint.get`](#cameraviewpointget) | - | 1.2 | The driver's-view viewpoint: offsets, field of view, stored value, editor state |
| [`camera.viewpoint.set`](#cameraviewpointset) | `x?` (number)<br>`y?` (number)<br>`z?` (number)<br>`fov_deg?` (number) | 1.2 | Change the driver's-view viewpoint live (opens the editor; not stored until saved) |
| [`camera.viewpoint.save`](#cameraviewpointsave) | - | 1.2 | Store the shown viewpoint for this vehicle and camera and close the editor |
| [`camera.viewpoint.reset`](#cameraviewpointreset) | - | 1.2 | Back to the vehicle's own viewpoint and remove the stored one |
| [`session.spawn`](#sessionspawn) | `vehicle` (string)<br>`repaint?` (string)<br>`setvars?` (json)<br>`entry?` (string)<br>`entry_id?` (integer) | 1.2 | Spawn a vehicle at an entry point of the map; it becomes the active vehicle when it exists |
| [`session.switch`](#sessionswitch) | `id?` (integer)<br>`step?` (integer) | 1.2 | Make a session vehicle the active one (cockpit camera, HUD profile, input and scripts follow) |
| [`session.despawn`](#sessiondespawn) | `id?` (integer) | 1.2 | Remove a session vehicle with its trailers (default: the active one) |
| [`session.respawn`](#sessionrespawn) | `id?` (integer) | 1.2 | Put a session vehicle back at its spawn point, at rest (default: the active one) |
| [`menu.do`](#menudo) | `command` (string) | 1.0 | Drive the in-game vehicle menu (open, spawn, vehicle <text>, repaint <text>, done, ...) |

### `scene.load_map`

Start a session on a map.

A debug shortcut around the plan-a-drive wizard: it starts a session directly. The player-facing route is the wizard (`ui.*`). Answers when the HUD is up. A load that failed answers at once with `1008 FAILED` (the problems in the message). A load with problems answers while the loading screen waits for *Continue* (`waiting_for_continue: true`, the problems listed): click `loading.continue` (`ui.click`) to start the drive.

### `scene.unload`

Return to the main menu.

A debug shortcut: leaves the drive without the confirmation dialog (`pause.leave` in the pause overlay is the player route).

### `vehicle.set`

Replace the player vehicle.

Restarts the session with the vehicle and answers like `scene.load_map` (including `1008` for a failed load and `waiting_for_continue` for a load with problems).

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

Store the shown viewpoint for this vehicle and camera and close the editor.

**Result:** the `camera.viewpoint.get` result

Writes `viewpoints.json` (next to `drive_history.json`) before it answers. A viewpoint equal to the vehicle's own removes the stored entry. Answers `1001` unless the driver's view is active.

### `camera.viewpoint.reset`

Back to the vehicle's own viewpoint and remove the stored one.

**Result:** the `camera.viewpoint.get` result

Resets offsets and field of view, stores the default (which removes the entry) and closes the editor. Answers `1001` unless the driver's view is active.

### `session.spawn`

Spawn a vehicle at an entry point of the map; it becomes the active vehicle when it exists.

**Result:** `{id, queued:true}`; poll `session.vehicles` until `lifecycle` is `alive` (or `failed`, with `error`)

`vehicle` is an absolute `.bus` path (below an allowed read root) or a catalogue id or name. `repaint` defaults to the vehicle's own; `setvars` is an object `{variable: 0|1}` set on the scripts after the repaint's. `entry` is an entry point name (exact, else the only one containing it), `entry_id` its record number; without either the session's start entry is used. The previous active vehicle is parked (parking brake on).

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

### `menu.do`

Drive the in-game vehicle menu (open, spawn, vehicle <text>, repaint <text>, done, ...).

Queues one command of the vehicle menu (see the in-game vehicle menu page); read the result with `menu.state`.

## Group `settings`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| `settings.list` | `category?` (string) | 1.0 | List the settings with their values (optionally one category) |
| `settings.get` | `key` (string) | 1.0 | Value, default and options of one setting |
| `settings.set` | `key` (string)<br>`value` (string) | 1.0 | Change a setting (validated like the settings page, saved) |
| `settings.reset` | `key` (string) | 1.0 | Restore the default of a setting |
| [`hardware.profile`](#hardwareprofile) | `redetect?` (boolean) | 1.2 | The detected hardware profile and the recommended defaults (local, no identifiers) |

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
| `screenshot` | `path?` (string) | 1.0 | Save a screenshot |
| `report` | `text` (string) | 1.0 | File a report (screenshot + text) |
| [`snapshot`](#snapshot) | `path?` (string)<br>`screenshot?` (boolean)<br>`warnings?` (integer) | 1.1 | One call that returns everything an agent needs to judge the state, optionally with a screenshot |

### `snapshot`

One call that returns everything an agent needs to judge the state, optionally with a screenshot.

**Result:** `{state, screen, drive?, hud?, camera, sim, warnings:[log lines], screenshot?}`

`path` writes the JSON to a file below the reports directory (or an allowed directory). `screenshot` also saves a PNG (the loading screen included) and returns its path. `warnings` is the number of recent `warn`/`error` log lines (default 20).

## Group `drive`

| Command | Arguments | Since | Summary |
| :--- | :--- | :--- | :--- |
| [`drive.controls`](#drivecontrols) | - | 1.1 | List every vehicle control by name with its kind, range and current value |
| [`drive.set`](#driveset) | `control` (string)<br>`value` (json)<br>`hold_ms?` (integer) | 1.1 | Set a vehicle control by name, optionally held for a duration |
| `drive.release` | `control?` (string) | 1.1 | Release held controls (one by name, or all) |
| [`drive.trigger`](#drivetrigger) | `name` (string)<br>`hold_ms?` (integer) | 1.1 | Raise a vehicle script trigger (doors, lights, indicators, horn, ...) |
| [`drive.state`](#drivestate) | - | 1.1 | Speed, position, heading, gear, pedals, parking brake, distance, camera and HUD state |
| [`vehicle.teleport`](#vehicleteleport) *(debug)* | `x` (number)<br>`y` (number)<br>`heading_deg?` (number) | 1.1 | Place the player vehicle at an absolute OMSI position and heading, standing still |
| [`vehicle.reset`](#vehiclereset) *(debug)* | - | 1.1 | Put the player vehicle back where the session placed it, standing still |

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
