//! The basic commands of the table: meta, queries, input, simulation and settings (`ping` to `l10n.reload`), in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c("ping", "meta", "Check that the server answers", ""),
    c("version", "meta", "App and protocol version", ""),
    c(
        "help",
        "meta",
        "Describe one command (or list all)",
        "command:s@0?",
    )
    .result("`{command, group, summary, since, args, result, doc, debug, implemented}`; without `command` a list of `{method, summary}`"),
    c("capabilities", "meta", "List all commands by group", ""),
    c(
        "auth",
        "meta",
        "Authenticate with the server token",
        "token:s@0",
    ),
    c("shutdown", "meta", "Exit the application", "force:b?"),
    c("state", "query", "App state, camera mode, fps, worst frame time of the last 10 s (frame_ms_max), frame", ""),
    c(
        "scene",
        "query",
        "Loaded map, tiles, entity and asset counts",
        "",
    )
    .result("`{map, start_tile, tiles_loaded, tiles_missing, entity_count, entities_live, meshes, materials, images, asset_count, stats}`")
    .doc("`entity_count` is the number of entity slots the world has allocated (it only grows); `entities_live` counts the entities that exist now (since 1.16), which is the number to compare before and after leaving and entering a session; `meshes`, `materials` and `images` are the asset counts that add up to `asset_count`."),
    c(
        "scene.tiles",
        "query",
        "Resident tiles, tile loading mode and radius",
        "",
    )
    .result("`{mode, radius, automatic, setting_radius, recommended_radius, load_all, map_radius, start_tile, listed, listed_missing, resident_count, resident, queued, building, stream_budget_ms}`")
    .doc("What the streamer keeps resident (M-0080). `mode` is `all` (every tile, never unloaded; `radius` is `null`) or `radius` (the tiles within `radius` of the tile the camera stands on: 1 is 3 x 3). `setting_radius` is the stored `gameplay.tile_radius` (`0`: automatic, then `radius` is `recommended_radius`, derived from the hardware profile and limited to `map_radius`, the radius that covers the map from the start tile); `load_all` is `gameplay.load_all_tiles`. `resident` lists the tile keys `[x, y]` (a tile whose build is still in progress is in it and named in `building`; it fills in over a few frames within `stream_budget_ms` per frame, M-0089), `queued` counts tiles waiting to be built, `listed` the tiles `global.cfg` names and `listed_missing` those among them without a file. Set the mode with `settings.set`. `1001` outside a drive."),
    c("camera.get", "query", "Camera mode and transform", ""),
    c(
        "sim.get",
        "query",
        "Pause state, simulation time, time scale, session clock, weather and lighting quality",
        "",
    ),
    c("log.tail", "query", "Last N log lines", "n:i@0?"),
    c(
        "log.filter",
        "query",
        "Filtered log lines",
        "target:s? level:s? contains:s? n:i?",
    ),
    c(
        "ui.tree",
        "ui",
        "The UI as a tree: id, role, label, state, rect, focusable",
        "visible_only:b?",
    ),
    c(
        "ui.click",
        "ui",
        "Activate an element by id (a text field gets focus)",
        "id:s@0 count:i?",
    )
    .doc("`count` activates the element that many times (stepper buttons like `settings/<category>/<key>/up`); each activation is a separate message, as separate clicks would be."),
    c(
        "ui.focus",
        "ui",
        "Move keyboard focus to an element",
        "id:s@0",
    ),
    c(
        "ui.action",
        "ui",
        "Send a UI action (navigate right, confirm, back, ...)",
        "action:s@0 arg:s@1?",
    ),
    c("ui.state", "ui", "State of one element by id", "id:s@0"),
    c("entity.list", "query", "List entities", "filter:s@0?"),
    c("entity.get", "query", "Components of one entity", "id:i@0"),
    c("resource.list", "query", "List resource types", ""),
    c("resource.get", "query", "Value of one resource", "name:s@0"),
    c("asset.list", "query", "List loaded assets", "kind:s@0?"),
    c("asset.get", "query", "Metadata of one asset", "path:s@0"),
    c(
        "input.key",
        "input",
        "Press and release a key",
        "key:s@0 duration_ms:i?=duration",
    ),
    c(
        "input.key_hold",
        "input",
        "Press a key and keep it down",
        "key:s@0",
    ),
    c("input.key_release", "input", "Release a key", "key:s@0"),
    c(
        "input.text",
        "input",
        "Type text into the focused field",
        "text:s@0",
    ),
    c(
        "input.mouse.move",
        "input",
        "Move the mouse relatively",
        "dx:f@0 dy:f@1",
    ),
    c(
        "input.mouse.click",
        "input",
        "Click a mouse button",
        "button:s@0 x:f? y:f?",
    ),
    c(
        "input.mouse.scroll",
        "input",
        "Scroll the wheel",
        "delta:f@0",
    ),
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
    c(
        "settings.list",
        "settings",
        "List the settings with their values (optionally one category)",
        "category:s@0?",
    ),
    c(
        "settings.get",
        "settings",
        "Value, default and options of one setting",
        "key:s@0",
    ),
    c(
        "settings.set",
        "settings",
        "Change a setting (validated like the settings page, saved)",
        "key:s@0 value:s@1 confirm:b?",
    )
    .result("the setting as `settings.get` shows it, plus `previous`, `applied` (it changed), `needs_restart` and, while it waits, `pending_confirm` and `confirm_within_s`")
    .doc("Applies and saves at once. A setting that the page protects with a 15 s prompt (`apply.revert_after_s` in `settings.list`: window mode, resolution) can instead be tried with `confirm: false`: it is applied but not saved and waits for `settings.confirm` (keep and save) or `settings.revert` (restore); after its time it is reverted by itself, like the page's prompt. Only one such change waits at a time (`1006 BUSY` otherwise); `confirm: false` on any other setting changes nothing about how it is applied. `needs_restart` is true when the change only takes effect after a restart."),
    c(
        "settings.reset",
        "settings",
        "Restore the default of a setting",
        "key:s@0 confirm:b?",
    )
    .doc("Like `settings.set` with the default value, including `confirm`."),
    c(
        "settings.reset_keys",
        "settings",
        "Restore the default of every key binding, or of one group",
        "group:s@0?",
    )
    .since("1.23")
    .result("`{changed, bindings:[{key, previous, value}]}`")
    .doc("`group` is `all` (default), `vehicle`, `drive`, `camera`, `map` or `ui`. Only bindings that differ from their default are written; the defaults of all actions are unique, so the order does not matter. The `controls.reset.*` rows of the Controls page do the same."),
    c(
        "settings.confirm",
        "settings",
        "Keep the risky setting change that waits and save it",
        "",
    )
    .since("1.7")
    .result("`{key, value, confirmed: true}`")
    .doc("Finishes a `settings.set` or `settings.reset` with `confirm: false`. `1001 NOT_READY` when nothing waits."),
    c(
        "settings.revert",
        "settings",
        "Undo the risky setting change that waits",
        "",
    )
    .since("1.7")
    .result("`{key, value, reverted: true}`")
    .doc("Restores the value from before a `settings.set` or `settings.reset` with `confirm: false`. `1001 NOT_READY` when nothing waits."),
    c(
        "settings.set_many",
        "settings",
        "Change several settings at once, all or none",
        "values:j@0",
    )
    .since("1.7")
    .result("`{count, changed: [{key, previous, value, needs_restart}], unchanged, needs_restart: [keys]}`")
    .doc("`values` is an object of setting key to value. The whole batch is validated first (unknown or unavailable keys, bad values, key bindings that clash inside the batch); if anything is refused nothing is applied and the answer is `-32602` with `data.errors: [{key, code, message}]` listing every refusal. Settings with a revert prompt are applied at once, without the `confirm` wait. Saved once at the end; at most 1000 entries."),
    c(
        "settings.diff",
        "settings",
        "The settings that differ from their defaults",
        "",
    )
    .since("1.7")
    .result("`{count, differences: [{key, value, default}]}`")
    .doc("Visible settings only (internal state such as the last map is left out)."),
    c(
        "settings.export",
        "settings",
        "A profile of the changed settings, to import later",
        "all:b?",
    )
    .since("1.7")
    .result("`{format: \"opts-settings\", version: 1, count, settings: {key: value}}`")
    .doc("Without `all` only the settings that differ from the defaults; with `all: true` every visible setting. Hand the result (its `settings` object) to `settings.import`."),
    c(
        "settings.import",
        "settings",
        "Apply a settings profile, all or none",
        "settings:j@0 reset_missing:b?",
    )
    .since("1.7")
    .result("as `settings.set_many`")
    .doc("`settings` is the object `settings.export` returned (`values` is accepted as a synonym). Validated and applied like `settings.set_many`. With `reset_missing: true` the settings that differ from the defaults and are not in the profile go back to their defaults, so the result is exactly the profile."),
    c(
        "hardware.profile",
        "settings",
        "The detected hardware profile and the recommended defaults (local, no identifiers)",
        "redetect:b?",
    )
    .since("1.2")
    .result("{ probing, stored, allow_in_reports, profile, recommendations }")
    .doc("`profile` is the versioned `opts-hardware` document (every fact is `{known: v}` or `{unknown: reason}`; `null` until the first probe finished). `redetect: true` starts a new probe in the background; poll again. Nothing is stored by this call. See the hardware probe charter."),
    c(
        "l10n.languages",
        "l10n",
        "List the languages (built in and installed packs) with completeness, authors and problems",
        "",
    )
    .since("1.1")
    .result("`{current, packs_dir, languages:[{code, name, native_name, direction, builtin, from_pack, authors, version, total, translated, copied, missing, percent, errors, warnings, outdated, missing_fonts, selectable, current, dir}], problems:[{folder, issues}]}`")
    .doc("`translated` counts messages that are really translated, `copied` those that are still the English text, `missing` those that show English. `selectable` is false for right-to-left packs. `problems` lists pack folders that cannot be used (no valid `pack.json`). Packs are files in `<data dir>/lang/<code>/`, see the localisation packs page."),
    c(
        "l10n.set",
        "l10n",
        "Switch the interface language (saved like a setting)",
        "code:s@0",
    )
    .since("1.1")
    .result("`{current, previous}`")
    .doc("`1004` for a language that is not installed, `1007` for a right-to-left pack (not supported yet). Stores `interface.language` and updates every text at once."),
    c(
        "l10n.reload",
        "l10n",
        "Scan the language folder again (new or edited packs)",
        "",
    )
    .since("1.1")
    .result("the `l10n.languages` result plus `{usable, broken, errors}`")
    .doc("The same as the *Reload languages* button. A pack that cannot be used or a file with errors never fails the call: read `problems` and `l10n.report`. If the active language is gone the game shows English."),
];
