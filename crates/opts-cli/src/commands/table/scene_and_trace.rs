//! Scene, trace and query commands of the table (`vehicle.reset` to `session.save`), in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c(
        "vehicle.reset",
        "drive",
        "Put the player vehicle back where the session placed it, standing still",
        "",
    )
    .since("1.1")
    .doc("A debug command: restores the start position, zeroes speed and the controls and the distance counter.")
    .debug(),
    c(
        "camera.reset",
        "scene",
        "Reset the current camera view to its default, like Shift+Space",
        "",
    )
    .since("1.2")
    .result("`{mode, sub_camera}`")
    .doc("Same code as the key. Free roam returns to the pose recorded on entry (speed 5 m/s); external to yaw 0, pitch -15 degrees, zoom 8 m, no pan; ego looks forward; the driver's view (2) and the passenger view (4) keep their sub camera and look straight ahead, and view 2 returns to the stored viewpoint (an open editor is cancelled). Never changes the mode. Answers `1009` while a dialog or the vehicle menu is open."),
    c(
        "camera.look",
        "scene",
        "Turn the camera: absolute yaw/pitch (free roam) or relative look (cockpit, external)",
        "yaw:f? pitch:f? dx:f? dy:f?",
    )
    .since("1.1")
    .result("`{mode, yaw_deg, pitch_deg}`")
    .doc("`dx`/`dy` are mouse-motion pixels delivered like a right-drag; `yaw`/`pitch` (degrees, compass yaw) set the free-roam camera directly and answer `1001` in other modes."),
    c(
        "camera.move",
        "scene",
        "Move the free camera forward/right/up for a duration, like the movement keys",
        "forward:f? right:f? up:f? duration_ms:i?",
    )
    .since("1.1")
    .result("`{ok, mode, duration_ms}`")
    .doc("Switches nothing: answers `1001` unless the camera is in free roam (use `camera.set_mode free`). Positive values move forward, right and up; the movement keys are held for `duration_ms` (default 500) and released, so speed follows `free_roam_speed`."),
    c(
        "camera.viewpoint.get",
        "scene",
        "The driver's-view viewpoint: offsets, field of view, stored value, editor state",
        "",
    )
    .since("1.2")
    .result("`{editing, dirty, row, vehicle, camera, cameras, viewpoint:{x,y,z,fov_deg}, stored:{x,y,z,fov_deg}, fov_deg, default_fov_deg, limits, file}`")
    .doc("Offsets are metres from the vehicle's own `[add_camera_driver]` eye: `x` right, `y` up, `z` forward (limit +-0.6). `fov_deg` in the viewpoint is `null` while the vehicle's own field of view is used; the top-level `fov_deg` is the one shown. `vehicle` is the catalogue identity (the `.bus` file stem, lower case) the viewpoint is stored under, `camera` the driver camera index."),
    c(
        "camera.viewpoint.set",
        "scene",
        "Change the driver's-view viewpoint live (opens the editor; not stored until saved)",
        "x:f? y:f? z:f? fov_deg:f?",
    )
    .since("1.2")
    .result("the `camera.viewpoint.get` result")
    .doc("Only the given values change. Out-of-range values answer `-32602` (offsets +-0.6 m, `fov_deg` 30..120) instead of being clamped. Answers `1001` unless the driver's view (`camera.set_mode cockpit`) is active and `1009` while a dialog or menu is open. Equivalent to the editor panel: the camera shows the change at once; `camera.viewpoint.save` stores it, leaving the view or changing the sub camera cancels it."),
    c(
        "camera.viewpoint.save",
        "scene",
        "Store the shown viewpoint for this vehicle (or all of its model) and camera and close the editor",
        "scope:s[vehicle|model]@0?",
    )
    .since("1.2")
    .result("the `camera.viewpoint.get` result")
    .doc("Writes `viewpoints.json` (next to `drive_history.json`) before it answers. `scope` `model` (default `vehicle`) applies the viewpoint to every vehicle of the same model (the folder of its `.bus` file) and drops this vehicle's own override; other vehicles' own entries keep winning. A viewpoint equal to the vehicle's own removes the stored entry (or keeps an explicit default when the model has one). Answers `1001` unless the driver's view is active."),
    c(
        "camera.viewpoint.reset",
        "scene",
        "Back to the vehicle's own viewpoint and remove the stored one",
        "",
    )
    .since("1.2")
    .result("the `camera.viewpoint.get` result")
    .doc("Resets offsets and field of view, stores the default (which removes the entry) and closes the editor. Answers `1001` unless the driver's view is active."),
    c(
        "snapshot",
        "output",
        "One call that returns everything an agent needs to judge the state, optionally with a screenshot",
        "path:s@0? screenshot:b? warnings:i?",
    )
    .since("1.1")
    .result("`{state, screen, drive?, hud?, camera, sim, warnings:[log lines], screenshot?}`")
    .doc("`path` writes the JSON to a file below the reports directory (or an allowed directory). `screenshot` also saves a PNG (the loading screen included) and returns its path. `warnings` is the number of recent `warn`/`error` log lines (default 20)."),
    c(
        "batch",
        "meta",
        "Run several commands in order in one round trip",
        "commands:j@0 stop_on_error:b?",
    )
    .result("`{results:[{result}|{error}]}` in command order")
    .doc("At most 100 commands, no nesting. A command that waits (`ui.wait_for`, `scene.load_map`, `ui.drag`) finishes before the next one starts, so a batch is a script: act, wait, observe."),
    // ---- scene trace (M-0087) ---------------------------------------------------------------
    c(
        "trace.set",
        "trace",
        "Scene trace: set the mode and filters, or read the status",
        "mode:s@0? tiles:j? kinds:j? file:s? radius:f? center:j? size_cap_mb:i? overlay:b? reset:b?",
    )
    .since("1.4")
    .result("`{mode, file, bytes, truncated, records, in_memory, issues, map, tiles, filters, overlay}`")
    .doc("Mode `off` (default, free), `summary` (counts and issues) or `full` (every placed thing). Records are written as JSON Lines next to the run logs; switch the trace on before loading the map (meshes loaded earlier carry no decode information). `tiles` is `[[x, y], ...]`, `kinds` a list of `scenery`, `tree`, `attachment`, `parked`, `spline`, `terrain`, `traffic`, `human`, `vehicle`, `file` a substring or `*` pattern, `radius` metres around `center` (`[x, z]` in Bevy world coordinates, the camera by default). Without arguments the call is the status query. See `docs/src/architecture/scene-trace.md`."),
    c(
        "trace.pick",
        "trace",
        "Scene trace: what is under a screen pixel or the crosshair",
        "x:f? y:f? limit:i?",
    )
    .since("1.4")
    .result("`{hit, parents, candidates, ray, ground, pixel, view}`; `hit` is the full record of the nearest object on the ray")
    .doc("`x`, `y` are physical pixels as in a `screenshot`; without them the centre of the view (the crosshair) is used. `hit.file` is the `.sco`, `hit.meshes[*].file` the `.o3d` with its decode `stage` and texture-coordinate quality. `candidates` lists the next objects on the ray; `ground.tile` names the tile under the pixel. Needs the trace on (`1001` otherwise)."),
    c(
        "trace.tile",
        "trace",
        "Scene trace: counts and records of one tile",
        "tile:j kind:s? file:s? stage:s? issues_only:b? full:b? limit:i?",
    )
    .since("1.4")
    .result("`{tile, summary, count, records}`"),
    c(
        "trace.find",
        "trace",
        "Scene trace: records by file pattern, kind, decode stage or tile",
        "file:s? kind:s? stage:s? tile:j? issues_only:b? full:b? limit:i?",
    )
    .since("1.4")
    .result("`{count, records}`")
    .doc("At least one criterion is required. `file` is matched against the placed thing's file and its mesh files (substring or `*`). `stage` is one of `plain`, `twin`, `decode-cache`, `decode-background`, `decoded-now`, `decode-deferred`, `rejected`, `extent`, `read-failed`, `parse-failed`, `hidden`, `cached`."),
    c(
        "trace.region",
        "trace",
        "Scene trace: records within a radius of a ground point",
        "radius:f x:f? z:f? kind:s? file:s? stage:s? issues_only:b? full:b? limit:i?",
    )
    .since("1.4")
    .result("`{center, radius, count, records}`"),
    c(
        "trace.export",
        "trace",
        "Scene trace: write matching records as JSON Lines",
        "path:s@0 file:s? kind:s? tile:j? stage:s? issues_only:b? limit:i?",
    )
    .since("1.4")
    .result("`{path, records}`")
    .doc("Writes the records of the in-memory index (at most 150,000) to a path the control server may write to (`--cli-allow-dir`). The run file next to the logs holds everything."),
    // ---- game methods that existed without a table entry -------------------------------------
    c(
        "ui.navigate",
        "ui",
        "Open a shell destination or deep link (home, drive, settings, help, screen:settings/graphics)",
        "screen:s@0",
    )
    .doc("Sends the same `Navigate` message as a rail click; the shell may refuse a destination that is not available yet. While a dialog is open (the first-run question, a confirmation) the rail is behind it for a person, so the command answers `1009`."),
    c(
        "hud.state",
        "query",
        "What the HUD shows: widgets, texts, alert, speed, gear",
        "",
    ),
    c(
        "hud.minimap",
        "scene",
        "Read or change the HUD mini map: zoom (radius in metres), zoom mode, orientation, on/off",
        "zoom:i? zoom_mode:s[speed|fixed]? orientation:s[north|heading]? enabled:b?",
    )
    .since("1.4")
    .result("`{enabled, shown, zoom_m, zoom_mode, radius_m, target_m, speed_kmh, orientation, zoom_steps_m, texture_px, side_px, compact, redraws, last_us, worst_us, budget_us, road_chords, markers, centre, up_heading_deg}`")
    .doc("Without parameters it reads. `zoom` is the visible radius, one of 100, 200, 500, 1000 (m); `zoom_mode` is `speed` (the radius follows the speed: `zoom` is then the largest radius, `radius_m` the radius drawn now, `target_m` what the speed asks for, `speed_kmh` the filtered speed) or `fixed`; `orientation` is `north` (north up) or `heading` (the player's heading up); `enabled` shows or hides the map. The three are the settings `hud.minimap_zoom`, `hud.minimap_orientation` and `hud.minimap` (changed and saved the same way; the keys F7, Shift+F7, `+` and `-` do the same). The map is centred on the player's vehicle, or on the free camera when there is no vehicle. `last_us` and `worst_us` are the time of the last and the slowest redraw (budget `budget_us`, at most 10 redraws per second); `hud.state` carries the same object as `minimap`. An unknown zoom or orientation answers `-32602`."),
    c(
        "hud.fullmap",
        "scene",
        "Read or drive the full-size map: open or close, zoom span, centre, follow",
        "open:b? span:f? x:f? y:f? follow:b? fit:b?",
    )
    .since("1.15")
    .result("`{open, follow, span_m, centre, px_per_m, screen, texture_px, redraws, last_us, worst_us, road_chords, markers, min_span_m, max_span_m, overview}`")
    .doc("The full-size map of the session (key F6). Without parameters it reads. `open` opens or closes it (the change happens in the next frame; opening is refused silently while a dialog, the pause overlay or the vehicle menu holds the input, or before the session has loaded). `span` is the number of metres across the shorter side of the screen, limited to `min_span_m`..`max_span_m` (50 m to 20 km); `x` and `y` (east and north in OMSI metres) centre the map and stop following the vehicle; `follow` follows the active vehicle (or the camera) again; `fit` shows the whole map (every lane read so far, with a margin; the End key does the same). `centre` is the world point in the middle of the screen, `px_per_m` the scale in logical screen pixels, `screen` the logical window size. `last_us` and `worst_us` are the time of the last and the slowest redraw (a redraw follows a change of the view at once, a moving vehicle at most ten times a second, a still map twice a second). Mouse drag, wheel and Home work through `input.mouse_*` and `input.key` like for a person. `overview` is the reader of the roads of the whole map (`{started, complete, percent, tiles_done, tiles_total}`): the first opening starts a background thread that reads every tile of the map, and the map draws these roads for all tiles that are not resident while it fills in. `1001` outside a drive."),
    c(
        "hud.alert",
        "debug",
        "Raise or clear a test alert on the HUD",
        "severity:s[info|warning|critical]@0? clear:b?",
    )
    .debug(),
    c(
        "hud.toast",
        "debug",
        "Show a test toast",
        "message:s@0 details:s?",
    )
    .debug(),
    c(
        "vehicle.get",
        "query",
        "Position, heading and speed of the player vehicle (absolute OMSI coordinates)",
        "",
    )
    .result("`{x, y, z, heading_deg, speed_kmh}`"),
    c(
        "vehicle.script_vars",
        "query",
        "Script variables of the player vehicle (substring filter)",
        "contains:s@0?",
    ),
    c(
        "session.vehicles",
        "query",
        "The session's user vehicles: id, name, repaint, spawn entry point, lifecycle, active marker",
        "",
    )
    .since("1.2")
    .result("`{active, count, vehicles:[{id, name, catalogue_id, repaint, setvars, spawn_entry_name, spawn_point, lifecycle (spawning|alive|despawned|failed), despawn_reason, error, active, trailers}], place_label}`")
    .doc("The registry of the vehicles the user spawned (the wizard's choice, the pause menu, `session.spawn`); parked and AI vehicles are not listed. Despawned vehicles stay in the list as history. `place_label` is what the HUD shows for the active vehicle (the entry point it was spawned at) or `null`."),
    c(
        "session.spawn",
        "scene",
        "Spawn a vehicle at an entry point of the map; it becomes the active vehicle when it exists",
        "vehicle:s@0 repaint:s? setvars:j? entry:s? entry_id:i?",
    )
    .since("1.2")
    .result("`{id, queued:true}`; poll `session.vehicles` until `lifecycle` is `alive` (or `failed`, with `error`)")
    .doc("`vehicle` is an absolute `.bus` path (below an allowed read root) or a catalogue id or name. `repaint` defaults to the vehicle's own; `setvars` is an object `{variable: 0|1}` set on the scripts after the repaint's. `entry` is an entry point name (exact, else the only one containing it), `entry_id` its record number; without either the session's start entry is used. The previous active vehicle is parked (parking brake on)."),
    c(
        "session.switch",
        "scene",
        "Make a session vehicle the active one (cockpit camera, HUD profile, input and scripts follow)",
        "id:i@0? step:i?",
    )
    .since("1.2")
    .result("`{ok, active}`")
    .doc("`id` names the vehicle; `step` (1 next, -1 previous) cycles through the living ones like the F8 key. The vehicle that was active keeps its state with the parking brake on. Answers `1004` for an unknown id and `1001` when the vehicle is not in the world."),
    c(
        "session.despawn",
        "scene",
        "Remove a session vehicle with its trailers (default: the active one)",
        "id:i@0?",
    )
    .since("1.2")
    .result("`{ok, active, removed}`: the vehicle that is active afterwards (`null`: none, free camera) and the entities removed")
    .doc("No confirmation (the confirmation dialog belongs to the pause menu). Despawning the active vehicle makes the most recently spawned other vehicle active, or none."),
    c(
        "session.respawn",
        "scene",
        "Put a session vehicle back at its spawn point, at rest (default: the active one)",
        "id:i@0?",
    )
    .since("1.2")
    .result("`{ok, id}`")
    .doc("Shows the HUD place label again. Answers `1001` when the vehicle is not in the world."),
    c(
        "formation.validate",
        "scene",
        "Check a train formation against the installation (every problem is listed)",
        "saved:s? zug:s? cars:j? name:s? reverse:b?",
    )
    .since("1.6")
    .result("`{ok, name, cars, issues:[{key, car, message}]}`; `car` is the 0-based car or null")
    .doc("The formation is a saved one (`saved`, its id from `formation.list`), a `.zug` of the installation (`zug`, a path relative to the OMSI root such as `Trains\\X10_112_IC-8.zug`) or `cars`, an array of vehicle files or `{vehicle, orientation, powered}` objects (`orientation` `forward` or `backward`; `powered` true or false says whether the car pulls, else the first car and every multi-file car pull and single-file cars behind the head are hauled). `reverse` runs the train the other way round (order reversed, every car turned). Rules: at least two cars; every file exists and is a rail vehicle (`[type]` 2 with `[boogies]`); the first car has a driver camera and is not turned; each car has the coupling point towards a neighbour; no vehicle group names; at most 64 vehicle files in total counting the parts of multi-file cars; a multi-file car (a multiple unit) can be turned, mirrored as a whole. Needs a running session (the installation index)."),
    c(
        "formation.spawn",
        "scene",
        "Spawn a train formation at an entry point; its first car becomes the active vehicle",
        "saved:s? zug:s? cars:j? name:s? reverse:b? entry:s? entry_id:i?",
    )
    .since("1.6")
    .result("`{id, queued:true, cars}`; poll `session.vehicles` until `lifecycle` is `alive` (or `failed`, with `error`)")
    .doc("As `formation.validate` for the formation, then like `session.spawn`: the first car is the vehicle, the other cars are coupled behind it on the track the entry point lies on (within 12 m, else the train stays a road vehicle). A formation with problems answers `-32602` with the problems in the message. The player drives the first car; the other cars are passive. The mass of the whole train acts on the dynamics (resistance and gradient on all cars, brakes on all, traction on the cars that pull)."),
    c(
        "formation.save",
        "scene",
        "Save a train formation in the user's formation folder",
        "saved:s? zug:s? cars:j? name:s? reverse:b? id:s? overwrite:b?",
    )
    .since("1.6")
    .result("`{ok, id, name, cars}`")
    .doc("The formation is described as for `formation.validate`; `name` renames it. It is written as `<user data>/formations/<id>.toml`, the id made from the name and never one that exists (`name`, `name-2`, ...). `id` with `overwrite: true` replaces that file instead. An empty formation is refused. A formation with open problems can be saved (it can be fixed later)."),
    c(
        "formation.list",
        "scene",
        "List the saved train formations, newest first",
        "",
    )
    .since("1.6")
    .result("`{dir, formations:[{id, name, cars, modified} | {id, error, modified}]}`; `modified` is Unix seconds")
    .doc("A file that cannot be read is listed with its `error` and kept."),
    c(
        "formation.delete",
        "scene",
        "Delete a saved train formation",
        "id:s@0",
    )
    .since("1.6")
    .result("`{ok, id}`")
    .doc("Answers `1004` for an unknown id and `-32602` for an id that is not a formation id."),
    c(
        "session.save",
        "scene",
        "Save the running drive under a name",
        "name:s@0 overwrite:b?",
    )
    .since("1.3")
    .result("`{ok, id, name, overwritten}`")
    .doc("Writes a session save (see the session-save page): every registered section is captured and the file is written atomically. Answers `1001` without a loaded drive, `1008` with `data.kind = \"exists\"` when a save of that name exists and `overwrite` is not set. The save is the same one the pause overlay's *Save game* writes."),
];
