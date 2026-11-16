//! Vehicle query and session-spawn commands of the table, in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
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
        "Script variables of the player vehicle (substring filter; `unit` picks a car of a consist)",
        "contains:s@0? unit:i?",
    ),
    c(
        "vehicle.script_textures",
        "query",
        "Write what the vehicles' scripts drew on their canvases (boards, IBIS, displays) as PNG files",
        "prefix:s@0?",
    )
    .since("1.26")
    .result("`{count, files:[{unit, slot, path, width, height, drawn}]}`")
    .doc("One PNG per script-texture slot of every loaded unit, `<prefix>-<unit>-<slot>.png` (default prefix `canvas`) in the captures directory: what the scripts drew, on slots no mesh shows too; `drawn` is false while nothing has been drawn on the canvas. A test aid for the displays of M-0100."),
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
        "vehicle:s@0 repaint:s? setvars:j? entry:s? entry_id:i? stop:j?",
    )
    .since("1.2")
    .result("`{id, queued:true}`; poll `session.vehicles` until `lifecycle` is `alive` (or `failed`, with `error`)")
    .doc("Since 1.29 `stop` (a station id, or a stop name of the map's `Busstops.cfg`) places the vehicle at that bus stop object with its front at the stop instead of at an entry point; while another vehicle (traffic or a session vehicle) stands within 10 m of it the spawn fails with `the stop is occupied`. `vehicle` is an absolute `.bus` path (below an allowed read root) or a catalogue id or name. `repaint` defaults to the vehicle's own; `setvars` is an object `{variable: 0|1}` set on the scripts after the repaint's. `entry` is an entry point name (exact, else the only one containing it), `entry_id` its record number; without either the session's start entry is used. The previous active vehicle is parked (parking brake on)."),
    c(
        "session.change_map",
        "scene",
        "Change the map of the running drive (the Alt menu's Change map): the vehicle and the time are carried over",
        "map:s@0 entry:s? vehicle:s[carry|none]? dry_run:b?",
    )
    .since("1.16")
    .result("`{map, map_id, entry_point, entry_defaulted, vehicle, repaint, date, time, weather, dropped, started}`")
    .doc("Leaves the drive and enters it again on another installed map through the loading path of *Load game*; poll `state` until `loading.ready` is true. `map` is a folder name, a friendly name or a part of one that names one map (else `-32602` with the candidates). `entry` is an entry point of the target map (exact, else a part that names one; default: the map's first). The active vehicle is built again at it with its repaint and equipment (`vehicle: carry`, the default; `none` goes on without a vehicle); the session date, time of day and weather are kept; the other session vehicles are dropped and listed in `dropped`. A vehicle that is not in the catalogue answers `1004` (ask for `vehicle: none` or choose another vehicle first). `dry_run` only answers what would happen (`started: false`). `1001` outside a loaded drive."),
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
        "session.reset_road",
        "scene",
        "Put a session vehicle on the nearest road lane under it, at rest (default: the active one)",
        "id:i@0?",
    )
    .since("1.29")
    .result("`{ok, id}`")
    .doc("The pause menu's *Reset on road*: the vehicle goes to the nearest road lane within 60 m (one that runs its way if there is one) and faces along it, as it faced before (against the lane, if it stood against it); speed, pedals and steering are cleared. The spawn point is not changed. Answers `1001` when the vehicle is not in the world or no road lane is near."),
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
