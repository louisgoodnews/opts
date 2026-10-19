//! UI, reload, debug, output and capture commands of the table (`l10n.report` to `vehicle.teleport`), in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c(
        "l10n.report",
        "l10n",
        "Completeness and every finding of one language",
        "code:s@0?",
    )
    .since("1.1")
    .result("`{language, missing_keys, copied_keys, issues:[{severity, code, file, line, key, message}], issue_lines}`")
    .doc("Without `code` the current language. `issue_lines` are the same findings as `file:line: error[code]: message` text, as `opts-tools l10n check` prints them."),
    c(
        "vehicle.set",
        "scene",
        "Replace the player vehicle",
        "path:s@0",
    )
    .doc("Restarts the session with the vehicle and answers like `scene.load_map` (including `1008` for a failed load and `waiting_for_continue` for a load with problems).")
    .debug(),
    c(
        "entity.spawn",
        "scene",
        "Spawn an asset",
        "asset:s@0 transform:j?",
    ),
    c("entity.despawn", "scene", "Despawn an entity", "id:i@0"),
    c(
        "entity.set_component",
        "scene",
        "Set a component value",
        "id:i@0 type:s@1 value:j@2",
    ),
    c(
        "reload.asset",
        "reload",
        "Force a reload of one file",
        "path:s@0",
    ),
    c(
        "reload.scene",
        "reload",
        "Reload every resident tile and the vehicle",
        "",
    ),
    c(
        "reload.config",
        "reload",
        "Reload one configuration file",
        "path:s@0",
    ),
    c("watch.add", "reload", "Watch another path", "path:s@0"),
    c("watch.remove", "reload", "Stop watching a path", "path:s@0"),
    c("watch.list", "reload", "List watched paths", ""),
    c(
        "debug.wireframe",
        "debug",
        "Toggle wireframe rendering",
        "enabled:b@0",
    ),
    c(
        "debug.normals",
        "debug",
        "Toggle normal visualisation",
        "enabled:b@0",
    ),
    c("debug.stats", "debug", "Render and loader statistics", ""),
    c(
        "screenshot",
        "output",
        "Save a screenshot of the application window, with metadata",
        "path:s@0? name:s? format:s[png|jpeg]? quality:i? include_metadata:b?",
    )
    .result("`{path, width, height, format, bytes, sim_time, frame, metadata_path?, metadata?}`")
    .doc("Captures the window's own render target: never the desktop, other windows or the cursor, in windowed and fullscreen mode alike. Files go below the captures directory (`<user data dir>/opts/captures`). `name` is a relative file name or subpath there (preferred); `path` is an explicit path that must lie inside the captures directory; both together are `-32602`, neither generates `{datetime}_{session_id}_{counter}.png`. `format` is `png` (default) or `jpeg` (with `quality` 1-100, default 90). With `include_metadata` (default true) a sidecar `<file>.json` is written and the metadata is returned: simulation time (`Time<Virtual>`, frozen while paused), render frame, fixed tick, map, camera mode, vehicle, window mode and size, versions. `1001` without a renderer or while the window is minimized, `1002` for a name or path outside the captures directory, `1005` when the frame does not arrive in 5 s. Extended in 1.5; the 1.0 call `{path}` still works for paths inside the captures directory."),
    c(
        "report",
        "output",
        "File a report (screenshot + text)",
        "text:s@0",
    ),
    // ---- screen capture (protocol 1.5) -------------------------------------------------------
    c(
        "capture.burst",
        "capture",
        "Capture N frames at a fixed interval as a PNG sequence",
        "count:i@0 interval_ms:i@1 format:s[png|jpeg]? quality:i? dir:s? metadata:b?",
    )
    .since("1.5")
    .result("`{dir, paths:[..], frames:[{index, path, sim_time, frame, wall_ms}], dropped, manifest}`")
    .doc("1-300 frames, `interval_ms` 16-60000, and `count` x `interval_ms` at most 90 000 ms. Frames are written to `dir` (a relative name below the captures directory; default `{datetime}_{session_id}_{counter}_burst`) as `0001.png`, … with a `manifest.json` carrying the shared metadata and one entry per frame. Replies when the last frame is written; a frame the renderer does not deliver within 2 s counts in `dropped`. Only one capture job (burst or recording) runs at a time (`1006`); `1001` without a renderer or while minimized, `1002` for a `dir` outside the captures directory."),
    c(
        "capture.record.start",
        "capture",
        "Start a bounded recording of the application window",
        "name:s? format:s[av1|png_sequence|h264]? fps:i? duration_ms:i? max_size_mb:i? quality:i? scale:f?",
    )
    .since("1.5")
    .result("`{recording_id, path, format, fps, effective_limits:{duration_ms, max_size_mb, min_free_disk_mb}}`")
    .doc("Returns at once; the recording runs in the background and is off unless started. `format` is `av1` (default; AV1 in an `.ivf` file, encoded by rav1e) or `png_sequence` (a directory of PNGs, ends early at the size cap); `h264` needs an optional build feature. `fps` 1-60 (default 30), `duration_ms` default 60 000 (the `opts-cli` default is 30 000) up to 30 min, `max_size_mb` default 512 up to 2048, `quality` 1-100, `scale` 0.25-1, rounded to a whole reduction (1, 1/2, 1/3, 1/4) and **0.5 by default**: the encoder is a pure-Rust AV1 build without assembly and measures about 9 fps at 1920x1013, 23 fps at half and 29 fps at a quarter size; frames it cannot keep up with are dropped (`dropped`, and `timing_ms` in `status` shows where the time goes). It also stops below 1 GiB free disk space, when the frame rate stays under 30 for 5 s, or when the renderer stops delivering frames. `name` is a relative stem below the captures directory. A sidecar `<file>.json` lists every frame with its simulation time. `1006` while another job runs, `1008` when the disk is nearly full, `1001` without a renderer."),
    c(
        "capture.record.stop",
        "capture",
        "Stop a recording and return its result",
        "recording_id:s@0=id",
    )
    .since("1.5")
    .result("`{recording_id, state, path, format, duration_ms, frame_count, dropped, bytes, stopped_by}`")
    .doc("`stopped_by` is `request`, `duration`, `size`, `disk`, `degraded`, `encoder_error` or `window`. Waits (at most 10 s) until the file is closed; stopping a finished recording returns its stored result. `1004` for an unknown id."),
    c(
        "capture.record.status",
        "capture",
        "Progress of a recording",
        "recording_id:s@0=id",
    )
    .since("1.5")
    .result("`{recording_id, state, frame_count, elapsed_ms, dropped, bytes, fps_measured, degraded, stopped_by?}`")
    .doc("`state` is `recording`, `finishing`, `finished` or `failed`; a finished recording returns its final result (the last 8 are kept). Never waits."),
    c(
        "capture.prune",
        "capture",
        "Delete old captures written by this subsystem (dry run unless told otherwise)",
        "older_than_days:i@0 dry_run:b?",
    )
    .since("1.5")
    .result("`{dry_run, items:[{path, bytes}], bytes}`")
    .doc("Only files with a metadata sidecar and burst or recording directories with a `manifest.json` are candidates, judged by their modification time; other files in the captures directory are never touched. `dry_run` defaults to true."),
    // ---- protocol 1.1: discovery -------------------------------------------------------------
    c(
        "commands",
        "meta",
        "Every command with its argument schema, result and documentation",
        "group:s?",
    )
    .since("1.1")
    .result("`{protocol_version, count, commands:[{method, group, summary, since, args, result, doc, debug, implemented}]}`")
    .doc("The machine-readable form of the generated reference. `args` entries are `{name, type, required, position, aliases, values?}`; `debug` marks commands that skip the input layer on purpose."),
    // ---- protocol 1.1: UI -------------------------------------------------------------------
    c(
        "ui.screen",
        "ui",
        "Where the app is: state, screen, open dialogs, toasts, focus, scrollables",
        "",
    )
    .since("1.1")
    .result("`{app_state, screen, loading?, modal_open, dialogs:[{id, tag?, label, visible, controls:[{id, label, role, enabled, focused}]}], toasts:[{id, label}], focus, back_stack, scrollables}`")
    .doc("One call that answers \"what is on screen and what can I do now\". `dialogs` lists every open modal (confirmations, the pause overlay, settings) with the ids of its controls, so a script can answer a confirmation without reading the whole tree; `loading` (stage and progress) is present while a drive loads, which is when the screen has nothing to click."),
    c(
        "ui.scrollables",
        "ui",
        "List the scrollable views with offset, content size and position",
        "visible_only:b?",
    )
    .since("1.1")
    .result("`{count, scrollables:[{id, owner, rect, offset, viewport, content, max, at_top, at_bottom, items:[id...]}]}`")
    .doc("`id` is what `ui.scroll target` takes: the element's own id, else `<nearest id>/scroll` (suffix `-2`, `-3` for duplicates). `items` are the ids of the elements inside the view in layout order, so `ui.scroll item` can reveal any of them."),
    c(
        "ui.scroll",
        "ui",
        "Scroll a view by an amount, by pages, to the top or bottom, or to an item",
        "target:s? by:f? dx:f? pages:f? to:s[top|bottom|left|right]? item:s?",
    )
    .since("1.1")
    .result("the scrollable after the move (as in `ui.scrollables`) plus `moved`")
    .doc("Without `target` the view under focus is used, else the first visible one; while a dialog is open only views inside it can be scrolled (`1009` otherwise). Exactly one of `by` (logical pixels, positive scrolls down), `dx`, `pages` (viewport heights), `to` or `item` is needed. `item` reveals the element with that id (the same effect as focusing it); `1004` when the item is not inside the view."),
    c(
        "ui.text",
        "ui",
        "Type text into a text field (optionally by id; replaces the content by default)",
        "text:s@0 id:s@1? replace:b? submit:b?",
    )
    .since("1.1")
    .result("`{ok, id, value}` - the field's content once the text has arrived")
    .doc("Focuses the field `id` (like a click), clears it when `replace` is true (the default) and types the text as keyboard events, so validation, search-as-you-type and the settings search react exactly as for a person. `submit` presses Enter afterwards. Without `id` the focused field receives the text. `1001` when the field is disabled, hidden or behind a dialog."),
    c(
        "ui.select",
        "ui",
        "Choose an option of a combo box, a tab, a radio or a list row",
        "id:s@0 option:s@1? index:i?",
    )
    .since("1.1")
    .result("`{ok, id, selected}`")
    .doc("For a combo box `option` is the visible text (or `index` the position): the drop-down is opened and the entry activated like clicks. For a tab, radio or list row no option is needed. Disabled targets answer `1001` with the reason."),
    c(
        "ui.toggle",
        "ui",
        "Set a switch, check box or radio to on or off (or flip it)",
        "id:s@0 value:b@1?",
    )
    .since("1.1")
    .result("`{ok, id, checked, changed}`")
    .doc("Only activates the element when its state differs from `value`; without `value` it flips. Settings with a revert prompt open the prompt as for a click."),
    c(
        "ui.hover",
        "ui",
        "Move the pointer over an element (tooltips, hover states)",
        "id:s@0",
    )
    .since("1.1")
    .result("`{ok, id, pos}` with the pointer position in logical pixels")
    .doc("Places the cursor at the centre of the element's rectangle and reports it to the window like a mouse move, so picking, hover states and tooltips behave as for a person. `1001` for an element without a rectangle (not laid out or hidden)."),
    c(
        "ui.key",
        "ui",
        "Press a key with modifiers (Ctrl, Shift, Alt) through the input layer",
        "key:s@0 ctrl:b? shift:b? alt:b? duration_ms:i?",
    )
    .since("1.1")
    .result("`{ok, key, modifiers}`")
    .doc("Presses the modifiers, then the key, and releases everything after `duration_ms` (default 50). The events are ordinary keyboard events, so shortcuts (`Ctrl+F`, `Ctrl+Tab`) and gating are the same as for a keyboard."),
    c(
        "ui.drag",
        "ui",
        "Drag from one element to another element or by an offset",
        "from:s@0 to:s@1? dx:f? dy:f? steps:i?",
    )
    .since("1.1")
    .result("`{ok, from, to?, from_pos, to_pos}` once the button is released")
    .doc("Presses the left button on the centre of `from`, moves the cursor in `steps` frames (default 8) to the centre of `to` or by `(dx, dy)` logical pixels and releases. Answers after the release."),
    c(
        "ui.wait_for",
        "ui",
        "Wait until a condition holds (element, screen, dialog, app state, speed, ...)",
        "until:s[exists|visible|gone|enabled|disabled|focused|checked|text|screen|dialog|no_dialog|app_state|hud_ready|speed_above|speed_below|distance_above|paused|running|sim_time_above|items_above]@0 id:s@1? value:s? number:f? timeout_ms:i?",
    )
    .since("1.1")
    .result("`{ok, until, waited_ms}` on success; error `1005` with `data.last` (the last observed value) when `timeout_ms` (default 10000, at most 120000) passes first")
    .doc("Polled once per frame on the main thread. `exists`, `visible`, `gone`, `enabled`, `disabled`, `focused` and `checked` need `id`; `text` needs `id` and `value` (the field or label contains it); `screen` takes the shell id in `value` (`settings`), `dialog` a dialog tag or id in `value` (`no_dialog` when none is open); `app_state` takes `MainMenu`, `Loading`, `InGame`, ...; `speed_above`/`speed_below` take km/h in `number`, `distance_above` the metres driven since the session started (see `drive.state`), `sim_time_above` seconds of simulation time, `items_above` the number of entries a scrollable `id` lists (a list fills in after the catalogue scan). Use it instead of sleeping."),
    // ---- protocol 1.1: driving --------------------------------------------------------------
    c(
        "drive.controls",
        "drive",
        "List every vehicle control by name with its kind, range and current value",
        "",
    )
    .since("1.1")
    .result("`{vehicle, controls:[{name, kind, range?, value, bound?}], triggers:[{name, key, held}]}`")
    .doc("`throttle`, `brake`, `steer`, `gear`, `parking_brake` are built in; `triggers` are the script triggers the vehicle's keyboard bindings can raise (doors, lights, indicators, horn, ... named as in the vehicle's scripts), usable with `drive.trigger` and `drive.set`."),
    c(
        "drive.set",
        "drive",
        "Set a vehicle control by name, optionally held for a duration",
        "control:s@0 value:j@1 hold_ms:i?",
    )
    .since("1.1")
    .result("`{ok, control, value, hold_ms?}`")
    .doc("`throttle`/`brake` take 0..1, `steer` -1 (left)..1 (right), `gear` `drive|neutral|reverse`, `parking_brake` true/false/`toggle`; any other name is raised as a script trigger (`horn`, `bus_doorfront0`, ...). Analog values stay set for `hold_ms` of **simulation time** (while the sim is paused the time does not run) or, without it, until set again or released with `drive.release`. The values reach the vehicle through the same system as the keyboard, so they are ignored while a dialog is open or the vehicle does not take input (`1009`)."),
    c(
        "drive.release",
        "drive",
        "Release held controls (one by name, or all)",
        "control:s@0?",
    )
    .since("1.1"),
    c(
        "drive.trigger",
        "drive",
        "Raise a vehicle script trigger (doors, lights, indicators, horn, ...)",
        "name:s@0 hold_ms:i?",
    )
    .since("1.1")
    .result("`{ok, name, handled, held_ms?}`")
    .doc("Raises `<name>`; for a binding that is held (like the horn) `<name>_off` follows after `hold_ms` of simulation time (default 200). `handled` says whether a script of the vehicle reacted. An unbound name is raised as is. Gated like a key press."),
    c(
        "vehicle.key",
        "drive",
        "Press a vehicle key (engine start, lights, indicators, doors, ...) by action or by key",
        "action:s@0? key:s? hold_ms:i?",
    )
    .since("1.1")
    .result("`{ok, raised:[{ok, name, action, raised, handled, held_ms?}]}`")
    .doc("Raises the script trigger that the key press raises, whatever the player has bound. `action` is the short name of a rebindable action (`battery`, `engine_start`, `lights`, `high_beam`, `indicator_left`, `hazard`, `door_1`, `interior_upper`, ... all listed by `vehicle.keys`) or a bound trigger name; `key` is a key chord (`L`, `Ctrl+Numpad1`) and raises every trigger bound to it, so a test can also check the binding itself. Held actions (battery, engine start, indicators, horn) get their `_off` after `hold_ms` of simulation time (default 200); give a longer `hold_ms` to hold the engine start until the engine runs. `handled` says whether a script of the vehicle reacted. Gated like a key press (`1001` without a player vehicle, `1009` while a dialog is open)."),
    c(
        "vehicle.keys",
        "drive",
        "List the vehicle key actions with their keys, defaults and triggers",
        "",
    )
    .since("1.1")
    .result("`{actions:[{action, setting, trigger, key, default, held}], other:[{trigger, key, held}]}`")
    .doc("`actions` are the rebindable ones (setting `controls.vehicle.<action>`, change with `settings.set`; a key that another action or OPTS itself uses is refused); `other` are the remaining `keyboard.cfg` vehicle bindings (IBIS, destination blinds, cash desk)."),
    c(
        "drive.state",
        "drive",
        "Speed, position, heading, gear, pedals, parking brake, distance, camera and HUD state",
        "",
    )
    .since("1.1")
    .result("`{speed_kmh, signed_speed_kmh, x, y, z, heading_deg, gear, throttle, brake, steer, parking_brake, rpm?, distance_m, camera, paused, time_scale, sim_time, alert?, route, stops}`")
    .doc("`distance_m` is the path length driven since the session (or the last `vehicle.reset`) started. `route` and `stops` are `null`: the timetable is not implemented yet (`not available yet`); `alert` is the HUD alert on screen, if any."),
    c(
        "vehicle.teleport",
        "drive",
        "Place the player vehicle at an absolute OMSI position and heading, standing still",
        "x:f@0 y:f@1 heading_deg:f?",
    )
    .since("1.1")
    .doc("A debug command: it moves the vehicle directly (speed zero, controls neutral) without any gate. Coordinates are absolute OMSI metres as in `vehicle.get`.")
    .debug(),
];
