//! Drive, session and menu commands of the table (`session.load` to `menu.state`), in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c(
        "session.load",
        "scene",
        "Restore a save (leaves the running drive); poll `session.saves` for the progress",
        "name:s@0",
    )
    .since("1.3")
    .result("`{ok, started, id, name, unknown_sections, missing_sections, warnings}`")
    .doc("`name` is a save's name or id. The restore goes through the normal loading screen and works from Home and from inside a drive. It is asynchronous: `session.saves` reports `restore.state` (`loading`, `done` with `warnings`, `failed`). Answers `1004` for an unknown save and `1008` for a damaged or newer file (`data.kind` `corrupt`, `newer_format`) or when the map or every vehicle is missing (`data.kind = \"missing_content\"`, `data.issues`)."),
    c(
        "session.saves",
        "query",
        "The list of saves, the registered save sections and the state of a restore",
        "",
    )
    .since("1.3")
    .result("`{dir, running, saves:[{id, name, map, vehicles, play_time_s, saved_unix, sections, unknown_sections, from_backup, issues}|{id, error}], sections:[{id, version, about}], restore:{state, id?, warnings?}}`")
    .doc("Most recent first. A file that cannot be read is listed with its `error` (`kind`: `corrupt`, `newer_format`, ...). `issues` name missing or changed content (`MissingMap`, `MissingVehicle`, `Changed`)."),
    c(
        "session.delete_save",
        "scene",
        "Delete a save and its backup (no confirmation)",
        "name:s@0",
    )
    .since("1.3")
    .result("`{ok, id}`"),
    c(
        "session.rename_save",
        "scene",
        "Change the name of a save (the file keeps its id)",
        "name:s@0 new_name:s",
    )
    .since("1.3")
    .result("`{ok, id, name}`"),
    c(
        "menu.do",
        "scene",
        "Drive the in-game vehicle menu (open, spawn, vehicle &lt;text&gt;, repaint &lt;text&gt;, done, ...)",
        "command:s@0",
    )
    .doc("Queues one command of the vehicle menu (see the in-game vehicle menu page); read the result with `menu.state`."),
    c(
        "vehicle.setvars",
        "scene",
        "List or change the equipment options (setvars) of the vehicle being planned or driven",
        "values:j? allow_unlisted:b?",
    )
    .since("1.8")
    .result("without `values`: `{context, repaint, family, count, options: [{name, label, description, kind, min, max, values, value, default, group, declared_in, evidence, range_unverified}], not_offered}`; with `values`: `{context, count, changed: [{name, previous, value}], live}`")
    .doc("The options of the vehicle in a drive (its player vehicle) or, outside a drive, of the vehicle chosen in *Plan a drive* once its repaints are discovered; `1001` when there is none. Each option has a `kind`: `switch` (0 or 1), `choice` (one of `values`), `number` (a whole number from `min` to `max`) or `fraction` (decimal, listed but not settable). `default` is the value of the chosen repaint; `evidence` tells whether repaints set it, the vehicle declares it and the vendor documents it; `range_unverified` marks a documented range that is narrower than what repaints set. Only variables the vehicle declares are offered (`not_offered` names the others). `values` is an object of option name to value: the whole batch is validated and applied all or none (`-32602` with `data.errors: [{key, code, message}]`). In the wizard a change is a choice of the plan, like the equipment dialog; in a drive it is set on the vehicle's scripts at once (`live: true`: a script that reads the variable only at start shows no change) and kept for a respawn. A name the vehicle does not offer is refused unless `allow_unlisted` is true."),
    c(
        "vehicle.dynamics",
        "drive",
        "What the player's vehicle states about its dynamics (mass, centre of gravity, inertia, rolling resistance, axles) with the source of each value, and the live axle loads",
        "",
    )
    .since("1.10")
    .result("`{spec: {mass_kg, cg_height_m, yaw_inertia_kg_m2, pitch_inertia_kg_m2, roll_inertia_kg_m2, rolling_coefficient}, axles: [{index, longitudinal_m, track_width_m, wheel_diameter_m, spring_n_per_m, damping_n_s_per_m, max_force_n, driven, steered, static_load_n, load_n: [left, right], deflection_m, deflection_speed_m_s, compression_m: [left, right], brake_force_n, brake_limited}], state: {speed_m_s, head_mass_kg, consist_mass_kg, units, wheelbase_m}, body: {heave_m, pitch_rad, roll_rad}, brakes: {pressure, apply_tau_s, force_n}}`; every `spec` entry is `{value, source}` with `source` one of `file`, `estimate`, `default`; since 1.11 the answer also has `pitch_inertia_kg_m2`, `roll_inertia_kg_m2`, `compression_m`, `brake_force_n`, `brake_limited`, `body` and `brakes`")
    .doc("`1001` without a player vehicle. `file` values are read from the vehicle's `.bus` (`[mass]`, `[schwerpunkt]`, `[momentofintertia]` or lines 2 to 4 of `[mass]`, `[rollwiderstand]`); `estimate` is the yaw inertia of a box with the bounding box's width and length; `default` is the generic transit bus, used because the file states nothing usable. The rolling coefficient is the file's rolling resistance (a force) over the stated weight. Axle values are those the physics uses (the `.bus` axle block, with bus defaults for what is missing); `static_load_n` follows from the axle positions, the mass and the centre of gravity."),
    c(
        "profiles.list",
        "drive",
        "The map profiles of the planned drive's map: the profile that applies, the saved ones, the presets, the load profile and the advice",
        "",
    )
    .since("1.14")
    .result("`{map, chrono, chrono_entries, resolved: {source, profile, dropped}, picked, saved: [{chrono, name, created, based_on, settings}], presets: [...], load: {tiles, objects, splines, ai_pool_vehicles, area_km2, estimated, counting, avg_frame_time_ms, p95_frame_time_ms}, advice: [{id, severity, preset, suggested}]}`")
    .doc("`1001` without a map in the plan. `chrono` is the newest Chrono entry active on the plan's date (`default` when none is); `resolved.source` is `chrono`, `map` or `global` (the fallback chain: this Chrono entry, the map's default, your global settings). The load profile is counted in the background the first time the map is asked for: `counting` is true until the full count is in, `estimated` while only a sample is. `avg_frame_time_ms` comes from the last session of this key on this computer."),
    c(
        "profiles.advice",
        "drive",
        "The load profile of the planned drive's map and the recommendations that follow from it",
        "",
    )
    .since("1.14")
    .result("`{load, advice}` as in `profiles.list`")
    .doc("`advice[].id` is one of `geometry-above-typical`, `geometry-high`, `light-map`, `many-splines`, `high-ai-load`, `ran-slowly`, `ran-smoothly`, `not-played-yet`; `suggested` lists settings to change as `{key, value}` and `preset` names the preset that carries the suggestion."),
    c(
        "profiles.apply",
        "drive",
        "Use a profile for the next drive of the planned map (Apply Selected)",
        "profile:s make_default:b?",
    )
    .since("1.14")
    .result("the answer of `profiles.list`")
    .doc("`profile` is `none` (your global settings), `preset:<id>` (`performance`, `balanced`, `quality`) or the name of a saved profile of the map. The profile is applied when the drive starts and the global settings return when it ends; the settings file never holds profile values. With `make_default` the profile is also written as the profile of this key. `-32602` for a profile that does not exist, `1001` without a map."),
    c(
        "profiles.save",
        "drive",
        "Create a profile from the current values of the map-eligible settings (Create New from Current)",
        "name:s chrono:b?",
    )
    .since("1.14")
    .result("the answer of `profiles.list`")
    .doc("Saved as the map's default profile, or with `chrono: true` as the profile of the newest active Chrono entry. Settings nothing is stored for are saved with their default. An existing profile of the same key is replaced."),
    c(
        "profiles.delete",
        "drive",
        "Delete the map's profile (the Chrono entry's with `chrono: true`)",
        "chrono:b?",
    )
    .since("1.14")
    .result("the answer of `profiles.list` plus `removed` (false when there was none)")
    .doc("Presets cannot be deleted."),
    c(
        "profiles.status",
        "drive",
        "Which profile the running drive uses",
        "",
    )
    .since("1.14")
    .result("`{active, map, chrono, profile, source, keys, overlaid}`")
    .doc("`keys` are the settings the profile overrides, `overlaid` the keys the settings currently hold as session values (the global values return when the drive ends)."),
    c(
        "bus.routes",
        "drive",
        "List the lines and courses of the depot (.hof) of the player's vehicle",
        "limit:i?",
    )
    .since("1.12")
    .result("`{depot, count, routes: [{route_code, line, course, line_string, route_string, terminus_code, terminus}], selected}`; `selected` is the route code or `null`")
    .doc("`1001` without a player vehicle. `route_code` is `line * 100 + course` as keyed on the IBIS (`13001`: line 130, course 01); `terminus` is the first string of the destination's row in the depot. The depot is the `.hof` the game chose for the vehicle and the map (the vehicle's folder first); a vehicle without a depot lists nothing."),
    c(
        "bus.get_route",
        "drive",
        "The route the player's vehicle has selected, and whether its IBIS is still being keyed",
        "",
    )
    .since("1.12")
    .result("`{depot, selected: {route_code, line, course, line_string, route_string, terminus_code, terminus} or null, stop_index, typing}`")
    .doc("`1001` without a player vehicle. `typing` is true while the keys of a selection are still being pressed (one key per physics step)."),
    c(
        "bus.set_route",
        "drive",
        "Select a route for the player's vehicle the way a driver does: the IBIS keys, then the destination to the displays",
        "route_code:i? line:i? course:i? clear:b?",
    )
    .since("1.12")
    .result("`{selected: {...} or null, keypad, typing}`; without a keypad also `note`")
    .doc("Give `route_code`, or `line` and `course`, or `clear` (the driver cancels the route). The selection changes the timetable callbacks at once and the vehicle's own scripts take it: the keys of the MAN-type IBIS (`ibis_setmode_linie_kurs`, the digits, `ibis_eingabe`, ...) are pressed one per physics step, and the target (`AI_Target_Index`, `SetLineTo`, the trigger `ai_scheduled_settarget`) is offered to the destination displays until `target_index_int` shows it (30 s at most). Vehicles with another IBIS get the second part only (`keypad: false`). `-32602` for a route the depot does not have, `1001` without a player vehicle."),
    c(
        "bus.timetables",
        "drive",
        "List the map's timetables: line files, their tours and the runs of a tour",
        "line:s? tour:i? limit:i? all:b?",
    )
    .since("1.13")
    .result("without `line`: `{available, lines: [{file, priority, user_allowed, tours, runs}]}`; with `line`: `{available, line, tours: [{tour, name, vehicle_group, runs}]}`; with `line` and `tour`: `{available, line, tour, name, runs: [{run, trip, title, line, start, stops}]}`")
    .doc("The map's `TTData` folder: `.ttl` line files with their tours (`[newtour]`) and the runs of each tour (`[addtrip]`: a trip file, a timing profile and the start time). Without `line` only the line files the player may drive (`[userallowed]`) are listed, all of them with `all: true`. `available: false` when the map has no timetable files. `1001` without a player vehicle, `-32602` for an unknown line or tour."),
    c(
        "bus.set_timetable",
        "drive",
        "Follow a run of the map's timetable: its stops and times answer the vehicle's timetable callbacks, the depot's matching route is keyed on the IBIS",
        "line:s? tour:i? run:i? stops:i? clear:b?",
    )
    .since("1.13")
    .result("`{schedule: {line_file, tour, trip, title, line, start, stop_count, stops: [{station, name, arrival, departure, distance_m}]}, route_code, typing}`; `distance_m` is the distance along the run from its first stop (`null` unless `StnLinks.cfg` links every pair of stations); `route_code` is the depot route selected for the run (`null` when the depot has none for its line and destination)")
    .doc("`line` is the file name of a line file without extension, `tour` and `run` count from 0 (default 0). The run's trip gives the stops (`Busstops.cfg` names) and, with its timing profile, the scheduled arrival and departure of each; times without a manual entry in the profile are interpolated by station index. `GetTTDelay` is the simulation clock minus the arrival at the current stop (`bus.set_stop`). `stops` limits how many stops the answer lists (default 5). `clear` stops following. `-32602` for a run the map does not have or a map without timetable files."),
    c(
        "bus.set_stop",
        "drive",
        "Set the stop of the route or timetable the vehicle is at (0-based)",
        "index:i@0",
    )
    .since("1.13")
    .result("`{stop_index, stop_count}`")
    .doc("What `GetTTBusstopIndex` answers and which stop the delay is measured at. The game does not detect bus stops by itself yet, so the stop is set here (or by a script test). `-32602` beyond the stops of the selected route or timetable."),
    c(
        "config.list",
        "scene",
        "List the saved bus configurations (vehicle, repaint, equipment options) with their check against the installation",
        "",
    )
    .since("1.9")
    .result("`{count, configurations: [{id, name, vehicle, vehicle_name, repaint, setvars, notes, favourite, created, last_used, usable, problems: [{code, message}]}]}`")
    .doc("The configurations are kept in `bus_configurations.json` beside the drive history. Every one is checked when listed: `usable` is false when its vehicle is not installed (any more); `problems` also names a repaint the vehicle no longer offers (`repaint_missing`, the default is used) and equipment values the vehicle's catalogue no longer accepts (`setvar_unknown`, `setvar_rejected`: not applied). `1008` when the file cannot be read (damaged, or from a newer version)."),
    c(
        "config.save",
        "scene",
        "Save the vehicle, repaint and equipment options of the plan (or of the driven vehicle) under a name",
        "name:s@0 notes:s? replace:b?",
    )
    .since("1.9")
    .result("`{id, replaced, configuration}`")
    .doc("Takes the vehicle chosen in *Plan a drive* or, in a drive, the player's vehicle (it must be in the vehicle catalogue). An existing name (ignoring case) is refused with `-32602` unless `replace` is true; a replaced configuration keeps its id, creation time and favourite mark. `1001` when there is no vehicle to save."),
    c(
        "config.load",
        "scene",
        "Write a saved configuration into the plan (vehicle, repaint, equipment options)",
        "id:s@0",
    )
    .since("1.9")
    .result("`{id, name, context, vehicle, repaint, setvars, problems}`")
    .doc("`id` is the id or the name. The configuration is checked first: a missing vehicle refuses it (`1008`, `data.problems`), a missing repaint falls back to the default and values the catalogue no longer accepts are left out; both are listed in `problems`. In a running drive nothing is changed (`1007`): configurations are loaded in *Plan a drive*. `1004` for an unknown configuration."),
    c(
        "config.spawn",
        "scene",
        "Build a saved configuration as a new vehicle of the running drive",
        "id:s@0 entry:s? entry_id:i?",
    )
    .since("1.18")
    .result("`{id, queued:true, configuration, name, problems}`; poll `session.vehicles` until `lifecycle` is `alive` (or `failed`)")
    .doc("As `session.spawn` with the vehicle, repaint and options of the configuration (`id` is its id or name), after the same check as `config.load`: a missing vehicle or `.bus` file refuses it (`1008`, `data.problems`), a missing repaint falls back to the default, values the catalogue no longer accepts are left out and listed in `problems`. The new vehicle becomes the active one and the previous one is parked; `entry`/`entry_id` as in `session.spawn`. `1001` outside a loaded drive, `1004` for an unknown configuration."),
    c(
        "config.delete",
        "scene",
        "Delete a saved configuration",
        "id:s@0",
    )
    .since("1.9")
    .result("`{ok, id}`"),
    c(
        "config.update",
        "scene",
        "Rename a saved configuration, change its notes or its favourite mark",
        "id:s@0 name:s? notes:s? favourite:b?",
    )
    .since("1.9")
    .result("`{configuration}`")
    .doc("At least one of `name`, `notes`, `favourite`. A name that is empty or taken by another configuration is refused with `-32602`."),
    c(
        "config.duplicate",
        "scene",
        "Copy a saved configuration under a new name",
        "id:s@0 name:s",
    )
    .since("1.9")
    .result("`{id, configuration}`"),
    c(
        "audio.state",
        "query",
        "The sound engine: volumes, voice budget, listener and the loudest emitters",
        "",
    )
    .since("1.19")
    .result("`{enabled, volumes:{master, effects, ambience, engine, interface}, voices:{budget, emitters, audible, with_voice, virtualized, playing}, listener, clips_cached, top:[{entity, bus, gain, distance, voice}]}`")
    .doc("`gain` is the level an emitter computes: its own gain × the distance fade × its bus' volume × the master volume (volumes are percent, squared into a gain). `voices.playing` counts voices whose sink exists, that is the sound card accepted them: on a machine without an output it stays 0 while `with_voice` rises. `listener` is the main camera's position (Bevy metres) or `null`."),
    c(
        "audio.tone",
        "scene",
        "Play a sine tone (a test signal for the sound output)",
        "frequency:n? seconds:n? gain:n? speed:n? bus:s[effects|ambient|vehicles|ui]? loop:b? at:j? reference:n? max:n?",
    )
    .since("1.19")
    .result("`{entity, bus, loop}`")
    .doc("`frequency` 20 to 20000 Hz (440), `seconds` 0.05 to 30 (1). Without `at` the tone plays everywhere; with `at: [x, y, z]` (Bevy metres) it is a source that fades from `reference` (5 m) to `max` (120 m) and is panned. The sound is kept after it ended, so `audio.state` can show it; `audio.stop` removes it. `1001` before the engine runs."),
    c(
        "audio.play",
        "scene",
        "Play a sound file of the installation through the engine",
        "file:s@0 gain:n? speed:n? bus:s[effects|ambient|vehicles|ui]? loop:b? at:j? reference:n? max:n?",
    )
    .since("1.19")
    .result("`{entity, bus, loop}`")
    .doc("`file` is a `.wav` below an allowed read root (`1002` outside, `1004` missing). It is decoded once and shared (`audio.state.clips_cached`). A file that is not RIFF/WAVE or has an unusual sample format answers `1008` with the reason (five MPEG files in the corpus are named `.wav`). `at`, `reference`, `max` as for `audio.tone`."),
    c(
        "audio.stop",
        "scene",
        "Stop and remove the sounds `audio.tone` and `audio.play` made",
        "",
    )
    .since("1.19")
    .result("`{stopped}`"),
    c(
        "menu.state",
        "query",
        "Page, choices and free entry points of the in-game vehicle menu",
        "",
    ),
];
