//! Simulation and scene commands of the table, in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c("sim.pause", "sim", "Pause the simulation", "")
    .doc("Pauses the simulation clock (`Time<Virtual>`) directly, not through the pause overlay; the overlay is reached with `ui.action menu` or `ui.key Escape`."),
    c("sim.resume", "sim", "Resume the simulation", ""),
    c(
        "sim.step",
        "sim",
        "Advance N frames while paused",
        "frames:i@0",
    ),
    c(
        "sim.time_scale",
        "sim",
        "Set the simulation speed",
        "factor:f@0",
    ),
    c(
        "sim.set_time",
        "sim",
        "Set the session clock (and date, weather, clock rate)",
        "seconds:f@0 date:s? weather:s? rate:f?",
    )
    .result("`{clock, seconds_of_day, date, map_date, weather, rate, revision}` after the change")
    .doc("Sets the session environment like the pause menu's *Environment* page: `seconds` since midnight (0 ≤ s < 86400), optionally `date` (`YYYY-MM-DD`; a new date is applied like *Apply* and may rebuild the map, Chrono), `weather` (an installed preset's id such as `Sturm` or `#CAVOK`, its slug, its localised name or a word of it; the old ids `Clear Summer Day`, `Rainy Afternoon`, … work too and stand for the closest installed preset; `sim.get` lists the available ids as `weather_presets`) and `rate` (clock seconds per simulated second, 0 < rate ≤ 3600). Answers two frames later with the environment; `1001` outside a drive. `sim.get` reports the clock as `clock`, `seconds_of_day`, `date`, `map_date`, `weather` and `clock_rate`, and the weather as `weather_canonical`, `weather_presets` and `weather_conditions`."),
    c(
        "scene.load_map",
        "scene",
        "Start a session on a map",
        "path:s@0 entry:s? accept_problems:j? vehicle:s? repaint:s? setvars:j? weather:s? date:s? time:s? trip:s?",
    )
    .doc("Since 1.24 the drive's choices can be given here like on the command line: `entry` (start point, a name or a word of it such as `Ahlheim Hbf`), `vehicle` (a `.bus` path, relative to `Vehicles`) with its `repaint`, `setvars` (`{name: n}` equipment options of the vehicle), `weather` (preset id or name), `date` (`YYYY-MM-DD`) and `time` (`HH:MM`). `accept_problems: true` starts the drive without waiting for *Continue* when the load ends with problems (broken tiles ...; since 1.22, one load; the `--accept-load-problems` flag does it for every load); since 1.27 `accept_problems: \"always\"` does it for this and every later load of the running game, as `loading.accept_problems {always: true}` does. `path` is a map folder, or since 1.27 just its name in the installation's `maps` folder (`Tram`); the installation is readable from the main menu on. Since 1.28 `trip` (`LINEFILE:TOUR:RUN`, as `bus.timetables` lists them) plans a timetable run as the wizard's *Trip* step does: the bus starts at the run's first stop instead of the start point and follows the run; a run the map's timetables do not have, or whose first stop is no bus stop, is dropped with a warning in the log. A debug shortcut around the plan-a-drive wizard: it starts a session directly. The player-facing route is the wizard (`ui.*`). Answers when the HUD is up. A load that failed answers at once with `1008 FAILED` (the problems in the message). A load with problems answers while the loading screen waits for *Continue* (`waiting_for_continue: true`, the problems listed): call `loading.continue` (or click `loading.continue` with `ui.click`) to start the drive.")
    .debug(),
    c(
        "loading.continue",
        "scene",
        "Continue past the problems of a load (broken tiles ...)",
        "accept_all:b?",
    )
    .since("1.22")
    .result("`{continued, outcome, problems:[{key, args, fatal, detail}]}`")
    .doc("Presses *Continue* on a loading screen that waits because the load ended with problems (a map tile or part of the vehicle did not load; minor problems never wait). `continued` is false when nothing waited (still loading, or already continued); `outcome` is `Loading`, `Ready`, `ReadyWithProblems` or `Failed`; a failed load answers `1008` (nothing to continue). `accept_all: true` also lets the next load start by itself, `false` switches that off. The problems are listed either way so a script can decide whether the drive is worth it.")
    .debug(),
    c(
        "loading.accept_problems",
        "scene",
        "Let every load start its drive despite problems",
        "always:b?",
    )
    .since("1.27")
    .result("`{always, next_load, command_line}`")
    .doc("The running game's form of the `--accept-load-problems` flag: `always: true` makes every later load that ends with problems (broken tiles, a part of the vehicle that did not load) start its drive without waiting for *Continue*, loads begun in the plan-a-drive wizard (`ui.*`) included, until `always: false`. Without `always` it only reports. The answer lists the three switches: `always`, `next_load` (the one-load acceptance of `scene.load_map {accept_problems: true}` or `loading.continue {accept_all}`) and `command_line` (the flag). A failed load still stops; the problems are still logged and kept in the load report.")
    .debug(),
    c("scene.unload", "scene", "Return to the main menu", "")
    .doc("A debug shortcut: leaves the drive without the confirmation dialog (`pause.leave` in the pause overlay is the player route).")
    .debug(),
    c(
        "camera.set_mode",
        "scene",
        "Switch camera mode",
        "mode:s@0 sub:i@1?",
    ),
    c(
        "camera.teleport",
        "scene",
        "Move the camera (OMSI coordinates)",
        "x:f@0 y:f@1 z:f@2 yaw:f? pitch:f?",
    )
    .debug(),
    c(
        "render.look",
        "scene",
        "Get or set the colour look (vivid, natural, bevy)",
        "look:s@0?",
    ),
    c(
        "render.stats",
        "scene",
        "Frame times, draw counts per render phase, memory",
        "reset:b? parts:b?",
    )
    .doc("`frame_times` covers the frames since the last reset (average, median, 99th percentile, 1 % low rate); `phases` the latest frame's batch sets, bins and entities of Bevy's render phases summed over all views (one `multidraw_sets` entry is one multi-draw call, a batchable bin one instanced draw, every unbatchable entity one draw); `parts` (opt-in, scans the world) the visible mesh entities with their distinct meshes, materials and pairs; `gpu_diagnostics_ms` needs `OPTS_RENDER_DIAG=1`. `reset` clears the frame-time window after reading."),
];
