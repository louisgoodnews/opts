//! Vehicle dynamics commands of the table, in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
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
        "vehicle.fuel",
        "drive",
        "The fuel or battery charge of the player's vehicle; with `level` set it, with `refuel` fill it",
        "level:f? refuel:b?",
    )
    .since("1.32")
    .result("`{model, kind, capacity, content, level, mode, low, torque_factor}` (`{model: false}` without a fuel model), and `refused` (boolean) when `refuel` is sent")
    .doc("`level` is the share of the capacity (0 to 1, `-32602` outside it); `refuel: true` fills the store first, unless the vehicle moves faster than 0.5 m/s (either direction) or has no model: then `refused` is `true` and the store is unchanged (`refused` is `false` after a refill; it is absent without `refuel`; since 1.37). `mode` is the setting *Fuel and battery* (`off`, `realistic`, `accelerated`), `low` the low-level warning, `torque_factor` the share of the drive torque kept (0 for an empty diesel tank). `capacity` and `kind` are the model's assumptions (`docs/src/simulation/fuel-battery.md`), not OMSI values. `1001` without a player vehicle.")
    .debug(),
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
    .doc("What `GetTTBusstopIndex` answers and which stop the delay is measured at. While a timetable is followed the game advances the stop itself when the bus stands at the next stop; this sets it by hand. `-32602` beyond the stops of the selected route or timetable."),
    c(
        "rail.switch",
        "drive",
        "The points ahead of the player's rail vehicle and the way it takes; with `way` choose it",
        "way:s[straight|left|right]?",
    )
    .since("1.28")
    .result("`{placed, points_objects, points: [{x, y, position, lanes}], object: {x, y, position, needed} or null, ahead: {distance_m, ways, chosen} or null}`")
    .doc("The points within 150 m ahead of the player's rail vehicle: the distance, the ways they open (`straight` first) and the way the vehicle takes (its choice when the points offer it, else `straight`). `way` chooses the way for the next points, as the `rail_branch_*` actions do. The points object ahead (`object`, session-frame place) has a position that follows the trains: when the vehicle approaches and no vehicle is on the points, the position is set to the one the vehicle's way needs (`needed`); `points_objects` counts the points objects of the loaded tiles and `points` lists up to 50 of them. A vehicle that finds the points set against it takes the open way; the points never change under a vehicle. `1001` when the player has no rail vehicle; `placed` is false while it is not yet on a track. M-0113."),
    c(
        "traffic.scheduled",
        "drive",
        "The timetable runs on the road, the vehicles that drive them and their punctuality",
        "",
    )
    .since("1.27")
    .result("`{available, runs_on_the_road, vehicles, traffic_agents, spawned, arrivals, departures, missed_stops, mean_delay_s, positions: [{x, y, heading_deg}], last_arrivals: [{station, scheduled, actual, delay_s}]}`")
    .doc("Scheduled AI vehicles (`docs/src/architecture/scheduled-traffic.md`): the runs of the map's timetable that are on the road or start within 90 s at the session clock, how many of them have a vehicle now, the stops reached and left, and the delay at arrival (clock minus the scheduled arrival, seconds; negative is early) of the last arrivals. `mean_delay_s` is `null` before the first arrival. Vehicles appear only near the player, at the bus stop objects of the loaded tiles."),
];
