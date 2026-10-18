# 🎛️ Control Protocol

**Status:** 🟢 Implemented (protocol 1.3; design from M-0043) · **Decision record:** [ADR-0006](../adr/0006-control-protocol.md) · **Client:** `opts-cli` · **Server:** `opts-bevy::control`

A running OPTS instance can be driven from outside: a human at a terminal or an AI agent in a loop can operate every screen by id,
play the simulation (throttle, brake, steering, gear, doors, lights, camera), query state, pause and step
the simulation, capture screenshots and state snapshots, file reports and trigger reloads. **The complete command list with argument
schemas is the generated [Control Protocol Reference](../reference/control-protocol.md)**; this page explains the design. The server is **opt-in** and **loopback-only by default**.

## 1. Transport

| Property | Value |
| :--- | :--- |
| Transport | TCP |
| Default bind | `127.0.0.1:44201` (only when `--cli-port <N>` is given) |
| Ephemeral port | `--cli-port 0`: the OS picks a port; the address is logged at `INFO` as `control server listening addr=127.0.0.1:NNNNN` |
| Non-loopback | refused unless `--cli-token <TOKEN>` is set (`--cli-bind 0.0.0.0` additionally required) |
| Framing | UTF-8 JSON, one message per line (`\n`-terminated), at most 1 MiB per line |
| Concurrency | one thread per connection; any number of clients; requests of one connection are answered in order |

The server never touches the Bevy `World` from its threads. It parses a request, sends a `ControlCommand`
through a channel and waits for the reply; a Bevy system drains the channel once per frame
(`handle_control_commands`, schedule `First`), runs the handler with `&mut World` access and sends the reply
back. A command that is not answered within **5 s** (for example because the app is frozen) fails with
error `1005`. Socket I/O can therefore never block rendering.

## 2. Message format — JSON-RPC 2.0

```json
{"jsonrpc":"2.0","id":1,"method":"camera.set_mode","params":{"mode":"F6"}}
{"jsonrpc":"2.0","id":1,"result":{"mode":"free"}}
{"jsonrpc":"2.0","id":2,"error":{"code":1004,"message":"entity 99 not found","data":{"id":99}}}
```

- `params` is an object (positional arrays are rejected with `-32602`). Unknown members are ignored so newer
  clients can talk to older servers.
- A request without `id` is a *notification*: executed, never answered.
- A line that is a JSON **array** is a standard JSON-RPC batch; the reply is an array. The `batch` method
  (§4.8) is the sequential variant with `stop_on_error`.

## 3. Errors

| Code | Name | Meaning |
| ---: | :--- | :--- |
| -32700 | Parse error | the line is not valid JSON |
| -32600 | Invalid request | not a JSON-RPC request object |
| -32601 | Method not found | unknown method (see `capabilities`) |
| -32602 | Invalid params | missing, mistyped or out-of-range parameter |
| -32603 | Internal error | handler panicked or failed unexpectedly |
| 1001 | Not ready | the command needs a state the app is not in (for example `vehicle.set` in the menu) |
| 1002 | Forbidden path | path outside the allowed read/write roots (§7) |
| 1003 | Unauthorized | token missing or wrong |
| 1004 | Not found | entity, asset or file does not exist |
| 1005 | Timeout | the app did not process the command within 5 s |
| 1006 | Busy | rate limit exceeded or command queue full |
| 1007 | Unsupported | the command exists in the protocol but this build does not implement it |
| 1008 | Failed | the operation ran and failed (`data` carries the cause) |
| 1009 | Gated | the input layer refused the command: a dialog or the vehicle menu is open, or the camera is in free roam so the vehicle takes no input - exactly the gate that would swallow the same key press |

Every error object carries `data.name` (the symbolic name in the table: `NOT_READY`, `GATED`, ...) and `data.retryable` (`true` for 1001, 1005, 1006 and 1009: the same call can succeed later without a change), next to command-specific details such as `valid` (the accepted spellings), `reason` (why an element is disabled) or `last` (the last value a `ui.wait_for` saw). An agent can branch on `data.name` and `data.retryable` without parsing messages.

## 4. Command surface

Methods are grouped by capability; `capabilities` returns the same table at runtime with a `since` protocol
version per method. **Priority** marks the delivery order: **P1** is implemented in the first pass, **P2** is
specified and answers `1007` until implemented.

### 4.1 Meta

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `ping` | — | `{pong:true, uptime_secs}` | P1 |
| `version` | — | `{version, git_hash, build_date, protocol_version}` | P1 |
| `help` | `[command]` | the descriptor of one command: `{command, group, summary, since, args:[{name,type,required,position,aliases,values?}], result, doc, debug, implemented}` | P1 |
| `commands` | `[group]` | every descriptor as above plus the error table; the machine-readable twin of the [reference page](../reference/control-protocol.md) (protocol 1.1) | P1 |
| `capabilities` | — | `{groups:{name:[{method,since,implemented}]}}` | P1 |
| `auth` | `{token}` | `{ok:true}` — required first when `--cli-token` is set | P1 |
| `shutdown` | `[force]` | `{ok:true}` then the app exits (`force` skips the clean shutdown path) | P1 |

### 4.2 Query

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `state` | — | `{app_state, camera_mode, fps, frame, map, vehicle}` | P1 |
| `scene` | — | `{map, tiles:[{x,y,entities}], entity_count, asset_count, stats}` | P1 |
| `camera.get` | — | `{mode, sub_index, transform:{translation,rotation}, fov}` | P1 |
| `sim.get` | — | `{paused, sim_time, time_scale, tick}`, in a drive also the session clock `{clock, seconds_of_day, date, map_date, weather, clock_rate}` and the weather `{weather_canonical, weather_presets, weather_conditions}` (the id the weather stands for, the ids `sim.set_time` accepts, what the resolved weather is: visibility, wind, temperature, clouds, precipitation, wetness); `light_quality {mode, level, level_name, values, target_ms, late_share, up_wait_s, vsync}` is the effective [lighting quality](lighting-system.md) (`mode` `automatic` or `custom`, ladder `level` 0-5, `values` = `shafts`, `shaft_steps`, `night_shafts`, `lantern_cones`, `headlight_cones`, `cone_cap`, `cone_depth_fade`, `mirror_count`, `mirror_every`, `glass_reflections`, `bloom`; `target_ms` = the frame-time budget the governor judges against, `late_share` = share of late frames (over 1.25 × `target_ms`) in its window, `up_wait_s` = clean seconds needed for the next level up, `vsync` = whether the window presents with vsync; the last four are present once the governor runs); `light_cones {total, eligible, active, hidden_by_budget, select_us}` counts the visible [light cones](lighting-cones.md#implementation) (cone entities in the world, lit ones in range, shown, cut by `cone_cap`, microseconds of the last selection); `settings.set graphics.light_quality_mode custom` switches the mode | P1 |
| `log.tail` | `[n=100]` | `{lines:[{ts,level,target,message}]}` | P1 |
| `log.filter` | `{target?, level?, contains?, n?}` | as `log.tail` | P1 |
| `entity.list` | `[filter]` | `{entities:[{id,name,summary}]}`; filter keys: `name`, `category` | P2 |
| `entity.get` | `id` | `{id, components:{type:value}}` for components implementing `ControlInspectable` | P2 |
| `resource.list` | — | `{resources:[name]}` | P2 |
| `resource.get` | `name` | serialised value | P2 |
| `asset.list` | `[kind]` | `{assets:[{kind,path}]}` | P2 |
| `asset.get` | `path` | `{kind, size, vertices?, dims?}` | P2 |

### 4.3 Input injection

Injected input is delivered as ordinary Bevy input events (`KeyboardInput`, `MouseButtonInput`,
`MouseMotion`, `MouseWheel`, text via `KeyboardInput.logical_key`), so it passes through exactly the same
systems — including the modal gating — as a physical keyboard.

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `input.key` | `{key, duration_ms?=50}` | press, hold for the duration, release | P1 |
| `input.key_hold` | `{key}` | press without release | P1 |
| `input.key_release` | `{key}` | release | P1 |
| `input.text` | `{text}` | typed into the focused text field | P1 |
| `input.mouse.move` | `{dx, dy}` | relative motion | P1 |
| `input.mouse.click` | `{button, pos?}` | `button` ∈ `left,right,middle` | P1 |
| `input.mouse.scroll` | `{delta}` | wheel lines | P1 |

Key names: `A`–`Z`, `0`–`9`, `F1`–`F12`, `Escape`, `Enter`, `Space`, `Tab`, `Backspace`, `ArrowUp/Down/Left/Right`,
`ShiftLeft/Right`, `ControlLeft/Right`, `AltLeft/Right`, `Period`, `Slash`, `Comma`, `Minus`, `Equal`, brackets, `Numpad0`-`Numpad9`, `NumpadAdd` and friends; unknown names fail with `-32602` and the list of valid names in `data`.

### 4.4 Simulation control

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `sim.pause` | — | pauses `Time<Virtual>` | P1 |
| `sim.resume` | — | resumes | P1 |
| `sim.step` | `{frames}` | advances exactly N frames while paused; replies after the last one | P1 |
| `sim.time_scale` | `{factor}` | relative speed, 0.0 < factor ≤ 20 | P1 |
| `sim.set_time` | `{seconds, date?, weather?, rate?}` | sets the [session environment](session-environment.md) like the pause menu's *Environment* page: time of day (0 ≤ s < 86400), date `YYYY-MM-DD` (applied like *Apply*: may rebuild the map), weather preset (an installed preset's id, its slug, localised name or a word of it; the old ids too; the available ids are `weather_presets` of `sim.get`) and clock rate (0 < r ≤ 3600); answers `{clock, seconds_of_day, date, map_date, weather, rate, revision}`; `1001` outside a drive | P2 |

### 4.5 Scene manipulation

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `scene.load_map` | `{path, entry?}` | leaves the current session if any, starts one on the map folder and answers when the HUD is up (≤ 120 s) | P1 |
| `scene.unload` | — | returns to the main menu | P1 |
| `camera.set_mode` | `{mode, sub?}` | `mode` ∈ `ego, cockpit, external, pax, free` or `1`–`5` (the camera number keys); injects the key, so unavailable modes answer `1001`; answers `{mode, sub_index}` two frames later | P1 |
| `camera.teleport` | `{x,y,z,yaw?,pitch?}` | switches to free roam and moves the camera to the **absolute** OMSI world position (metres; tile `(i,j)` starts at `300·i`, `300·j`); `yaw` is a compass heading in degrees (0 = north, clockwise), `pitch` degrees up | P1 |
| `vehicle.setvars` | `[{values, allow_unlisted}]` | **1.8**: the equipment options (setvars) of the vehicle being planned or driven with kind, range, value and default (`{context, options:[…]}`); with `values` an object of option name to value, applied all or none (`-32602` with `data.errors` otherwise), set on the scripts at once in a drive; see [setvars](../formats/text/vehicles/setvars.md) | P2 |
| `menu.do` | `{command}` | drives the in-game vehicle menu (Left Alt): `open`, `close`, `spawn`, `vehicle <text>`, `repaint <text>`, `entry <text>`, `config`, `toggle <var>`, `step <var> [-1|1]`, `done`, `confirm`, `despawn`, `back`; see [vehicle menu](in-game-vehicle-menu.md) | P2 |
| `menu.state` | — | page, chosen vehicle/repaint/entry and the free entry points of the vehicle menu | P2 |
| `render.look` | `[look]` | `{look, previous, available}`; without `look` it reads the colour look, with `look` ∈ `vivid, natural, bevy` it sets it: live on every camera and the sky, and saved in the settings; unknown names answer `-32602` | P1 |
| `settings.list` | `[category]` | `{count, settings:[{key, category, kind, value, default, options, apply, available, hidden}]}`; `category` is one of `graphics, audio, controls, interface, gameplay, content, advanced` (`-32602` otherwise). `apply` is `live`, `restart` or `{revert_after_s}`; `available` is `yes`, `{needs}` or `{not_yet: milestone}`. The same registry feeds the settings screen and [the generated reference](../reference/settings.md) | P1 |
| `settings.get` | `{key}` | one entry as above; `1004` for an unknown key | P1 |
| `settings.set` | `{key, value, [confirm]}` | the entry after the change plus `previous`, `applied` (it changed) and `needs_restart` (it takes effect after a restart); `value` may be a string, number or boolean and is validated against the setting (`-32602` with what is expected); a planned setting answers `1001` with "Feature not available yet!"; saved at once and announced like a change on the page; **risky settings (window mode, size) skip the 15 s revert prompt** unless `confirm: false` is given (1.7): then the value is applied but not saved, `pending_confirm` and `confirm_within_s` say so, and `settings.confirm` keeps it, `settings.revert` (or the timeout) restores the old one; one such change waits at a time (`1006`) | P1 |
| `settings.reset` | `{key, [confirm]}` | the entry after the change plus `previous`; `1004` for an unknown key; `confirm` as in `settings.set` | P1 |
| `settings.confirm` / `settings.revert` | — | finish a risky change that waits (`confirm: false`): keep and save it, or restore the old value; `1001` when nothing waits | 1.7 |
| `settings.set_many` | `{values}` | `{count, changed, unchanged, needs_restart}`; **all or none**: the batch is validated first and a refusal answers `-32602` with `data.errors: [{key, code, message}]` for every refused entry, nothing applied | 1.7 |
| `settings.diff` / `settings.export` / `settings.import` | `[all]` / `{settings, [reset_missing]}` | the settings that differ from the defaults; a profile `{format, version, count, settings}` of them (or of all with `all: true`); applying a profile like `settings.set_many`, with `reset_missing` also resetting what it does not name | 1.7 |
| `l10n.languages` | none | `{current, packs_dir, languages:[{code, name, native_name, direction, builtin, from_pack, authors, version, total, translated, copied, missing, percent, errors, warnings, outdated, missing_fonts, selectable, current, dir}], problems:[{folder, issues}]}`: built-in languages and installed [language packs](localisation-packs.md) with completeness | P1 |
| `l10n.set` | `{code}` | `{current, previous}`; saved like `settings.set`; `1004` for a language that is not installed, `1007` for a right-to-left pack | P1 |
| `l10n.reload` | none | the `l10n.languages` result plus `{usable, broken, errors}`; scans `<data dir>/lang` again, never fails because of a bad pack | P1 |
| `l10n.report` | `[code]` | `{language, missing_keys, copied_keys, issues:[{severity, code, file, line, key, message}], issue_lines}` for `code` (default: the current language) | P1 |
| `vehicle.get` | — | `{x, y, z, heading_deg, speed_kmh}` of the player vehicle (absolute OMSI coordinates) | P1 |
| `vehicle.script_vars` | `{contains?}` | the player vehicle's script variables `{count, variables, ops_executed}` (substring filter) | P1 |
| `vehicle.set` | `{path}` | takes a `.bus` file and restarts the session with it (answers when it is up) | P1 |
| `entity.spawn` | `{asset, transform}` | `{id}` | P2 |
| `entity.despawn` | `{id}` | ok | P2 |
| `entity.set_component` | `{id, type, value}` | ok (only types registered as settable) | P2 |

### 4.6 Reload & debug

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `reload.asset` | `{path}` | forces the hot-reload pipeline for one file | P1 |
| `reload.scene` | — | reloads every resident tile and the vehicle | P1 |
| `reload.config` | `{path}` | alias of `reload.asset` for `.cfg/.bus/.sco/.sli` | P1 |
| `watch.add` / `watch.remove` | `{path}` | adds/removes a watch root | P1 |
| `watch.list` | — | `{roots:[path]}` | P1 |
| `debug.wireframe` | `{enabled}` | toggles wireframe rendering | P2 |
| `debug.normals` | `{enabled}` | toggles normal visualisation | P2 |
| `debug.stats` | — | `{fps, frame_ms, entities, draw_calls?, descramble:{decoded,skipped}}` | P2 |

### 4.7 Output

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `screenshot` | `{path?}` | `{path}`; default: the reports directory | P1 |
| `report` | `{text}` | `{png, report}` — same files as F1 | P1 |

### 4.8 Batch

`batch` takes `{commands:[{method,params}], stop_on_error?=false}` and returns `{results:[{result|error}]}`.
Commands run sequentially in order; at most 100 per batch. Because each command completes in the frame in
which it is executed (except `sim.step` and `scene.load_map`), a batch of 20 usually costs 1–3 frames.

### 4.9 UI by id

The UI can be read and driven without coordinates. Every interactive element has a **stable path id** (`menu.start`, or an automatic `parent/role:slug` such as `button:start-drive`; duplicates get `-2`, `-3`). Screens that must survive copy changes and language switches set explicit ids; the automatic slug comes from the message key when there is one, else from the label text. The actions send exactly the messages a pointer release, a keyboard or a gamepad would, so they pass the same gating, and they answer at once - the effect is visible one or two frames later (read it back with `ui.state`).

| Method | Params | Result | Priority |
| :--- | :--- | :--- | :--- |
| `ui.tree` | `{visible_only?=false}` | `{focus, count, tree:[node]}` where a node is `{id, role, label, value?, state, rect, focusable, children}`; `role` is one of `button, checkbox, radio, switch, textbox, combobox, option, tablist, tab, tabpanel, tree, treeitem, listitem, dialog, toast, region`; `state` is `{hovered, focused, pressed, selected, checked, expanded, disabled, disabled_reason, error, visible}`; `rect` is `[x, y, w, h]` in logical pixels or `null` before layout | P1 |
| `ui.state` | `{id}` | one node without children; `1004` for an unknown id | P1 |
| `ui.click` | `{id}` | activates the element (a text field gets focus); `1001` when it is hidden or disabled (`data.reason` carries the reason), `1004` for an unknown id | P1 |
| `ui.focus` | `{id}` | moves keyboard focus; `1001` when the element cannot take focus now (hidden, disabled without a reason, behind a dialog) | P1 |
| `ui.action` | `{action, arg?}` | sends a `UiAction`: `navigate up\|down\|left\|right`, `next-region`, `prev-region`, `next`, `previous`, `next-tab`, `prev-tab`, `confirm`, `back`, `search`, `menu`, `help`, `scroll page-up\|page-down\|home\|end\|up\|down`; `-32602` with the valid spellings in `data` otherwise | P1 |
| `ui.navigate` | `{screen}` | opens a shell destination (`home`, `drive`, `settings`, `help`) or deep link (`screen:settings/graphics`) like a rail click; `1004` for an unknown id; a destination that is *not available yet* is refused by the shell (read `ui.tree` to see the state) | P1 |

From the shell: `opts-cli ui.tree --visible_only true`, `opts-cli ui.click menu.start`, `opts-cli ui.action navigate right`. A scripted keyboard traversal is `ui.action next` repeated while reading `ui.tree`'s `focus`.

### 4.10 Operating every screen (protocol 1.1)

`ui.tree` and `ui.click` (§4.9) are joined by the commands an agent needs to *finish* a task on any screen. All of them address elements by id and send the messages a person would cause. A command that targets an element answers `1009` when the element is behind an open dialog (the dialog owns focus, so it owns the commands too); keys are not gated at the command but at the systems that read them, as for a keyboard.

| Need | Command | Notes |
| :--- | :--- | :--- |
| Where am I, what can I do now? | `ui.screen` | app state, shell screen id, open dialogs *with their button ids and tag*, toasts, focus, back stack, visible scrollable views |
| Wait instead of sleeping | `ui.wait_for` | `until` = `exists`, `visible`, `gone`, `enabled`, `disabled`, `focused`, `checked`, `text`, `screen`, `dialog`, `no_dialog`, `app_state`, `hud_ready`, `speed_above`, `speed_below`, `distance_above`, `paused`, `running`, `sim_time_above`, `items_above` (a list has more than N entries); `timeout_ms` (default 10 s); a timeout is `1005` with the last observed value in `data.last` |
| What can scroll, and what is in it? | `ui.scrollables` | every scroll container and master list: `id`, `offset`, `viewport`, `content`, `max`, `at_top`, `at_bottom` and `items` (for a windowed list *all* entries, not only the rows that exist as entities right now) |
| Scroll | `ui.scroll` | exactly one of `by` (pixels), `dx`, `pages`, `to` (`top`, `bottom`, `left`, `right`) or `item` (an element id inside the view, or an entry id/title of a list); `target` defaults to the view under focus |
| Type | `ui.text` | into the field `id` (focus, replace or append, optional `submit`); the field's own change message fires, so search-as-you-type and validation react |
| Choose | `ui.select` | combo box (by text or index), tab, radio, list row; for a master list `option` is an entry id or title and `choose:true` double-clicks it |
| Switch | `ui.toggle` | sets a switch, check box or radio to a wanted state, idempotent; without `value` it flips |
| Steppers ("sliders") | `ui.click` with `count` | numeric settings are `down`/`up` buttons (`settings/<category>/<key>/up`); `count` presses them N times. `settings.set` sets the value directly through the same registry |
| Pointer | `ui.hover`, `ui.drag` | move the cursor over an element (tooltips, hover states); drag between elements or by an offset over several frames |
| Keyboard | `ui.key` | a key with `ctrl`/`shift`/`alt` held, as ordinary keyboard events (`Ctrl+F`, `Alt` for the vehicle menu) |

**Scrolling is the wheel's effect, not a bypass.** A scroll container's `ScrollPosition` and a master list's offset are set to the value the wheel handler would produce, clamped to the content. `item` reveals an element with the least movement, exactly like keyboard focus does (`scroll_focus_into_view`).

### 4.11 Driving (protocol 1.1)

The vehicle is operated by *named controls*; the names are discovered with `drive.controls` (they depend on the vehicle).

| Command | What it does |
| :--- | :--- |
| `drive.controls` | built-in controls with kind, range and current value, plus the script triggers the vehicle's key bindings can raise (`bus_doorfront0`, `blinker_left_set`, `horn`, ...) with their standard short aliases (`door_front`, `indicator_left`, `lights`) |
| `drive.set control value [hold_ms]` | `throttle`, `brake` 0..1, `steer` -1..1, `gear` `drive`/`neutral`/`reverse`, `parking_brake` true/false/`toggle`; any other name is a script trigger |
| `drive.release [control]` | drops held values (the keyboard decays the pedals as usual) |
| `drive.trigger name [hold_ms]` | raises a script trigger: doors, lights, indicators, horn; held bindings (the horn) get their `_off` after `hold_ms` |
| `vehicle.key action\|key [hold_ms]` | presses a vehicle key without the keyboard: `action` is a rebindable action (`engine_start`, `lights`, `indicator_left`, `door_1`, ...) or a bound trigger, `key` a chord (`L`, `Ctrl+Numpad1`) that raises every trigger bound to it |
| `vehicle.keys` | the rebindable vehicle key actions with their current key, default and trigger, plus the other `keyboard.cfg` vehicle bindings |
| `drive.state` | speed (signed and absolute), absolute position, heading, gear, pedals, parking brake, **distance driven**, camera, pause and time scale, HUD alert; `route` and `stops` are `null` because the timetable does not exist yet |
| `vehicle.teleport`, `vehicle.reset` | *debug* placement: absolute OMSI position and heading, or back to the session's start pose, standing still |
| `vehicle.get`, `vehicle.script_vars` | position/speed and the scripts' variables (route, stop and ticket variables live there) |
| `menu.do`, `menu.state` | the in-game vehicle menu (spawn, vehicle, repaint, entry, done) |

**Hold semantics.** An analog value stays in force for `hold_ms` of *simulation time* (it does not run down while paused, which keeps *pause - act - step N - observe* deterministic) or, without `hold_ms`, until it is set again or released. The pedals are overridden on their axis only while held; when the hold ends the keyboard model takes over and the pedal decays like a released key.

**One action path.** Values go into `RemoteControls`, which `vehicle_input_system` merges *inside the keyboard system*, under its run conditions (the vehicle must take input - not in free roam - and no dialog or vehicle menu may be open). The handler tests the same conditions when the command arrives and answers `1009` instead of queueing a command that would never run; a value still queued when a gate closes is dropped. Triggers pass the modal gate like a bound key. The two *debug* commands (`vehicle.teleport`, `vehicle.reset`) write the chassis directly and say so in the reference.

**Driving 100 m** is a loop, not a sleep: `drive.set gear drive`, `drive.set parking_brake false`, `drive.set throttle 0.6`, `opts-cli ui.wait_for distance_above --number 100`, `drive.set throttle 0`, `drive.set brake 1`, `opts-cli ui.wait_for speed_below --number 1`.

### 4.12 Camera (protocol 1.1)

`camera.set_mode` switches the mode (it presses the number key, so unavailable modes answer `1001`); `camera.look` turns the view (`yaw`/`pitch` in free roam, `dx`/`dy` as mouse motion with the button a person holds in that mode: right in the cockpit, left outside); `camera.move` holds the movement keys of the free camera for `duration_ms`; `camera.teleport` places it absolutely (debug); `camera.get` reads it.

### 4.12a Session vehicles (protocol 1.2)

| Method | Params | Result |
| :--- | :--- | :--- |
| `session.vehicles` | - | `{active, count, vehicles:[...], place_label}` the registry of [session vehicles](session-vehicles.md) |
| `session.spawn` | `vehicle` (absolute `.bus` path or catalogue id/name), `repaint?`, `setvars?` (object), `entry?` (name) or `entry_id?` | `{id, queued:true}`; asynchronous, poll `session.vehicles` |
| `session.switch` | `id?` or `step?` (1 next, -1 previous) | `{ok, active}` |
| `session.despawn` | `id?` (default: the active vehicle) | `{ok, active, removed}`; no confirmation |
| `session.respawn` | `id?` (default: the active vehicle) | `{ok, id}` |

Unknown ids answer `1004`, vehicles that are not in the world `1001`. `hud.state` also reports `place_label` and `has_vehicle`.

### 4.12b Scene trace (protocol 1.3)

`trace.set`, `trace.pick`, `trace.tile`, `trace.find`, `trace.region` and `trace.export` query the scene trace (M-0087): which mesh file is placed where, with decode stage and texture-coordinate quality. They answer `1001` while the trace is off. `trace.pick` takes the pixel of a screenshot (or the crosshair) and returns the nearest object's full record with its parents. Full description, record schema and the screenshot-to-asset recipe: [scene trace](scene-trace.md).

### 4.13 Snapshots and screenshots

`screenshot` saves the window; `snapshot` returns, in one call, the app state, `ui.screen`, `drive.state`, the HUD, camera, simulation clock and the last warnings, optionally with a screenshot and as a file. It is the call an agent makes after every few actions to *see* what happened.

### 4.14 Discovery

`help <command>` returns the descriptor of one command; `commands [group]` returns all of them with the argument schema (`name`, `type`, `required`, `position`, `values`), result, documentation, `since`, `debug` and whether this build implements it, plus the table of error codes. `capabilities` groups the names. All three come from the same table in `crates/opts-cli/src/commands.rs`, which also drives the `opts-cli` argument parser and generates the [reference page](../reference/control-protocol.md); a test fails when the page is stale and another when a registered handler has no descriptor (or a descriptor promises something no build registers). A new command is one table entry plus one handler.

## 5. Rate limiting

Per connection: token bucket of 200 requests/s with a burst of 400; beyond that `1006 Busy`. The Bevy side
executes at most 64 commands per frame; the rest wait in the queue (bounded at 1,024; a full queue answers
`1006`). `input.key` with a `duration_ms` schedules the release on a timer, not by blocking the reply.

## 6. Versioning

`version` reports `protocol_version` (semver; **1.1.0**: operating every screen, driving, snapshots, structured errors, `commands`; **1.2.0**: the session vehicles `session.vehicles|spawn|switch|despawn|respawn` and `place_label` in `hud.state`; **1.3.0**: the scene trace `trace.set|pick|tile|find|region|export`, see [scene trace](scene-trace.md); **1.12.0**: `bus.routes|get_route|set_route` (the lines and courses of the vehicle's depot and their selection, M-0099); **1.11.0**: `vehicle.dynamics` shows the springs per wheel, the body's heave, pitch and roll and the brakes; **1.10.0**: `vehicle.dynamics` (the dynamics the player's vehicle states, M-0073); **1.9.0**: `config.list|save|load|delete|update|duplicate` (bus configurations, M-0077); **1.8.0**: `vehicle.setvars` (the equipment options of a vehicle, M-0078); **1.7.0**: complete settings control: `confirm` on `settings.set|reset`, `settings.confirm|revert`, `settings.set_many|import|export|diff`, `applied` and `needs_restart` in the answer of a change). Minor versions only **add** methods, optional parameters and
result members; clients ignore unknown result members and servers ignore unknown parameters. A breaking change
bumps the major version and changes the default port. `capabilities` lets a client feature-detect instead of
comparing versions.

## 7. Security model

- Default is **off**. Without `--cli-port` no socket exists.
- Loopback only. A non-loopback bind requires `--cli-token` and `--cli-bind`; the token is compared in
  constant time and required via `auth` before any other call.
- **Filesystem:** the server never accepts a command that writes outside the *reports directory* or
  directories passed with `--cli-allow-dir`. Reads (`scene.load_map`, `vehicle.set`, `watch.add`,
  `reload.asset`) must resolve (after canonicalisation, so `..` and symlinks are resolved) below the OMSI
  installation root or an allowed directory; otherwise `1002`.
- No command executes code or shell commands.
- All requests and errors are logged at `DEBUG`/`WARN` under target `opts_bevy::control`.

## 8. Why this suits AI agents

- Line-delimited JSON is trivially produced and parsed by an agent or `jq`; every reply is machine-readable
  with stable error codes.
- `capabilities` and `help` are self-describing, so an agent can discover the surface without the doc.
- `batch` and `sim.step` give deterministic, low-round-trip loops: *pause → act → step N → observe*.
- `screenshot` and `log.tail` close the observe loop (look at the frame, read the warnings).
- Hot reload (see [Hot Reload](hot-reload.md)) closes the edit loop: write a file, call `reload.asset`, inspect.

## 9. Using the protocol from an AI agent

**What an agent can do.** Everything a player can: answer the first-run question, plan a drive in the wizard
(map, vehicle, time), wait through loading, drive (throttle, brake, steering, gear, parking brake, doors,
indicators, lights, horn), switch and turn the camera, open the pause overlay and the settings, scroll their
lists, change a setting, leave the drive - all by id, through the same gates a person meets. Besides playing it
can look at the result (`screenshot`, `snapshot`), read what the game complained about (`log.filter`), edit a
content file and have it reloaded, and file a report.

**The verified session.** `crates/opts-cli/examples/play_session.sh` plays exactly this against a real game (Berlin-Spandau,
an MAN SD77 bus) and was run live for this page: Home, *Plan a drive* (`ui.select` on the map and vehicle lists, `Next`
three times, `Drive`), the loading screen (`ui.screen` reports `loading.stage`; `screenshot` captures it), `drive.set` gear, parking
brake and throttle, `ui.wait_for distance_above 100` (the bus had driven 100.2 m at 47 km/h), brake to a stop, `Escape` for
the pause overlay, `pause.settings`, `ui.scroll to bottom` on the settings view, `ui.click` on a texture-filtering option
(`settings.get` confirmed it), `pause.leave` and the confirmation, back on Home. Nothing in it sleeps: every step waits for a condition.

**Connecting.** Start the game with `opts-game --cli-port 44201` (add `--cli-token <T>` if the port is
reachable by others). The listening address is logged at `INFO` (`control server listening addr=…`), which is
how an agent finds an ephemeral port (`--cli-port 0`).

| Use | When |
| :--- | :--- |
| `opts-cli <method> [args]` | quick one-shot commands from a shell; `--json` for machine output; exit codes 0/1/2/3 tell success, command error, no connection, bad protocol |
| `opts-cli batch` (stdin) / `batch` method | several commands in one round trip |
| Raw TCP | an agent runtime with sockets: send one JSON line, read one JSON line — `printf '{"jsonrpc":"2.0","id":1,"method":"ping"}\n' \| nc 127.0.0.1 44201` |
| `opts_cli::client::Client` | Rust code; see `crates/opts-cli/examples/` |

**A worked session** (drive to a spot, look, report). Each line is one request; results are abbreviated.

```text
 1 ping                                         → {"pong":true,"uptime_secs":3}
 2 capabilities                                 → which methods exist and are implemented
 3 scene.load_map  path="…/maps/BRT Berlin"     → {"map":"BRT Berlin","tiles_loaded":8,…}   (≈ 2–20 s)
 4 camera.set_mode external
 5 sim.resume
 6 input.key_hold W
 7 sim.step 120                                 → only if the sim is paused; otherwise just wait
 8 input.key_release W
 9 sim.pause
10 camera.get                                   → where am I?
11 screenshot                                   → {"path":"…/reports/screenshot_2026-10-10_16-01-40.png"}
12 log.filter level=warn n=10                   → what did the game complain about?
13 report text="the junction at … has no road surface"   → {"png":…,"report":…}
```

As one batch, steps 4–12 cost a single round trip:

```json
{"jsonrpc":"2.0","id":9,"method":"batch","params":{"commands":[
  {"method":"camera.set_mode","params":{"mode":"external"}},
  {"method":"sim.pause"},
  {"method":"sim.step","params":{"frames":60}},
  {"method":"screenshot"},
  {"method":"log.filter","params":{"level":"warn","n":10}}]}}
```

**Deterministic loops.** Prefer *pause → act → `sim.step N` → observe* over sleeping: with the simulation
paused, input injected before a step takes effect in exactly those frames.

**Known limits of what an agent can do (honest list).**

- `route` and `stops` in `drive.state` are `null`: the timetable (route, stops, tickets) is not implemented in OPTS yet; the vehicle scripts' own variables are readable with `vehicle.script_vars`.
- Numeric settings are `-`/`+` steppers, not sliders: use `ui.click ... count` or set the value directly with `settings.set`.
- `ui.drag` and `ui.hover` move the cursor and press the left button like a person; they were exercised against the test UI and the real game's hover, but no real screen needs a drag yet.
- A freshly installed OPTS scans the OMSI folder for about 8 s. A wizard opened meanwhile (`home.new-drive` while Home still says *Reading*) starts with empty lists and fills them when the scan ends (fixed in P-1014: the step is built again when the catalogue changes); `ui.wait_for items_above wizard.where.list` waits for it. Leaving the wizard afterwards used to panic (*Entity despawned* in a deferred command) because Home's change detection counted the reload as new on the return; Home now watches the lists in every state.
- Control commands that skip the gates are marked *debug* in the reference (`scene.load_map`, `scene.unload`, `vehicle.set`, `vehicle.teleport`, `vehicle.reset`, `camera.teleport`, `hud.alert`, `hud.toast`); everything else is refused with `1009` where a key press would be swallowed.

**When a command fails** (error `code`): `-32602` — fix the parameter (the message names it; `input.key`
lists valid keys in `data`); `1001` — the app is in the wrong state (load a map first, or the camera mode is
unavailable); `1002` — the path is outside the allowed directories (use `--cli-allow-dir` when starting the
game); `1009` — a dialog or the vehicle menu is open, or the camera is in free roam: answer or close it first (`ui.screen` lists the dialog's buttons), or `camera.set_mode external`; `1004` — nothing there; `1005` — the app did not answer: it is frozen or loading, retry after
`ping`; `1006` — slow down or batch; `1007` — not implemented in this build (`capabilities` shows which);
`1008` — the operation ran and failed (reload of a broken file: the previous content is still in the game).
Connection errors (exit code 2) mean the game is not running or the port differs.

**Rate limits.** 200 requests/s per connection with a burst of 400; the game runs at most 64 commands per
frame. Batches of up to 100 commands avoid both limits.

**Editing content.** Write the file with your own tools; the game's watcher reloads it after a 250 ms quiet
period. To be sure, call `reload.asset {path}` — it answers with what was rebuilt, or with error `1008` and
the parser's message when the file is broken. A bad save never breaks the running scene.

## 10. Path to bevy_remote

Bevy 0.14 has no remote-protocol crate. From 0.15 on, `bevy_remote` offers a JSON-RPC 2.0 reflection API
(`world.query`, `world.get_components`, …). If the engine is upgraded, `entity.*` and `resource.*` should be
re-implemented as thin forwards to it instead of `ControlInspectable`; the method names and results defined
here stay the public contract.
