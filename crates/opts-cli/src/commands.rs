//! The command table of the control protocol and the translation of command-line words into
//! JSON parameters.
//!
//! Parameters are declared with a tiny notation: `name:type[@position][?][=alias,alias]` where `type`
//! is `s` (string), `i` (integer), `f` (float), `b` (bool) or `j` (JSON value), `@n` makes the
//! parameter the n-th positional word, and `?` marks it optional.

use serde_json::{json, Map, Value};

/// One protocol method.
#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    /// Method name sent to the server.
    pub method: &'static str,
    /// Group shown by `list`.
    pub group: &'static str,
    /// One-line description.
    pub summary: &'static str,
    /// Parameter declaration (see the module docs); empty for none.
    pub params: &'static str,
    /// Protocol version that introduced the method (`1.0`, `1.1`, ...).
    pub since: &'static str,
    /// What the result contains (one line); empty when it is only `{ok:true}`.
    pub result: &'static str,
    /// Longer description: semantics, gating, caveats; empty when the summary says it all.
    pub doc: &'static str,
    /// A debug command that deliberately skips the input layer and its gates (documented as such).
    pub debug: bool,
}

impl CommandSpec {
    /// Sets the version that introduced the method.
    #[must_use]
    pub const fn since(mut self, v: &'static str) -> Self {
        self.since = v;
        self
    }

    /// Sets the result description.
    #[must_use]
    pub const fn result(mut self, r: &'static str) -> Self {
        self.result = r;
        self
    }

    /// Sets the long description.
    #[must_use]
    pub const fn doc(mut self, d: &'static str) -> Self {
        self.doc = d;
        self
    }

    /// Marks a debug command that bypasses the input layer.
    #[must_use]
    pub const fn debug(mut self) -> Self {
        self.debug = true;
        self
    }

    /// The parameters as `{name, type, required, position, aliases, values}` objects.
    #[must_use]
    pub fn args_schema(&self) -> Value {
        Value::Array(
            parse_decls(self.params)
                .iter()
                .map(|d| {
                    let ty = match d.kind {
                        Kind::Str => "string",
                        Kind::Int => "integer",
                        Kind::Float => "number",
                        Kind::Bool => "boolean",
                        Kind::Json => "json",
                    };
                    let mut o = json!({
                        "name": d.name,
                        "type": ty,
                        "required": !d.optional,
                        "position": d.position,
                        "aliases": d.aliases,
                    });
                    if !d.values.is_empty() {
                        o["values"] = json!(d.values);
                    }
                    o
                })
                .collect(),
        )
    }

    /// The full descriptor as sent by `help` and `commands`.
    #[must_use]
    pub fn descriptor_json(&self) -> Value {
        json!({
            "method": self.method,
            "group": self.group,
            "summary": self.summary,
            "since": self.since,
            "args": self.args_schema(),
            "result": self.result,
            "doc": self.doc,
            "debug": self.debug,
        })
    }
}

const fn c(
    method: &'static str,
    group: &'static str,
    summary: &'static str,
    params: &'static str,
) -> CommandSpec {
    CommandSpec {
        method,
        group,
        summary,
        params,
        since: "1.0",
        result: "",
        doc: "",
        debug: false,
    }
}

/// Every method of protocol 1.x.
pub const COMMANDS: &[CommandSpec] = &[
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
    ),
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
        "path:s@0 entry:s?",
    )
    .doc("A debug shortcut around the plan-a-drive wizard: it starts a session directly. The player-facing route is the wizard (`ui.*`). Answers when the HUD is up. A load that failed answers at once with `1008 FAILED` (the problems in the message). A load with problems answers while the loading screen waits for *Continue* (`waiting_for_continue: true`, the problems listed): click `loading.continue` (`ui.click`) to start the drive.")
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
        "menu.state",
        "query",
        "Page, choices and free entry points of the in-game vehicle menu",
        "",
    ),
];

/// A spec for methods this client does not know (a newer server): any `name=value` or `--name value`
/// pair becomes a string/number/bool/JSON parameter.
#[must_use]
pub fn generic_spec() -> CommandSpec {
    c("", "", "", "")
}

/// The group names in table order, without repeats.
#[must_use]
pub fn groups() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for c in COMMANDS {
        if !out.contains(&c.group) {
            out.push(c.group);
        }
    }
    out
}

/// Looks a method up by name.
#[must_use]
pub fn find(method: &str) -> Option<&'static CommandSpec> {
    COMMANDS.iter().find(|c| c.method == method)
}

/// Method names starting with `prefix` (for completion).
pub fn completions(prefix: &str) -> impl Iterator<Item = &'static str> + '_ {
    COMMANDS
        .iter()
        .map(|c| c.method)
        .filter(move |m| m.starts_with(prefix))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Str,
    Int,
    Float,
    Bool,
    Json,
}

struct ParamDecl<'a> {
    name: &'a str,
    kind: Kind,
    position: Option<usize>,
    optional: bool,
    aliases: Vec<&'a str>,
    values: Vec<&'a str>,
}

fn parse_decls(decl: &str) -> Vec<ParamDecl<'_>> {
    decl.split_whitespace()
        .filter_map(|word| {
            let (head, aliases) = word.split_once('=').unwrap_or((word, ""));
            let (name, rest) = head.split_once(':')?;
            let optional = rest.contains('?');
            let position = rest
                .split_once('@')
                .map(|(_, tail)| {
                    tail.trim_end_matches('?')
                        .parse::<usize>()
                        .unwrap_or(usize::MAX)
                })
                .filter(|p| *p != usize::MAX);
            let values: Vec<&str> = rest
                .split_once('[')
                .and_then(|(_, t)| t.split_once(']'))
                .map(|(v, _)| v.split('|').filter(|x| !x.is_empty()).collect())
                .unwrap_or_default();
            let kind = match rest.chars().next()? {
                's' => Kind::Str,
                'i' => Kind::Int,
                'f' => Kind::Float,
                'b' => Kind::Bool,
                'j' => Kind::Json,
                _ => return None,
            };
            Some(ParamDecl {
                name,
                kind,
                position,
                optional,
                aliases: aliases.split(',').filter(|a| !a.is_empty()).collect(),
                values,
            })
        })
        .collect()
}

fn coerce(kind: Kind, name: &str, raw: &str) -> Result<Value, String> {
    let bad = |what: &str| format!("parameter `{name}` must be {what}, got `{raw}`");
    match kind {
        Kind::Str => Ok(json!(raw)),
        Kind::Int => raw
            .parse::<i64>()
            .map(|v| json!(v))
            .map_err(|_| bad("an integer")),
        Kind::Float => raw
            .parse::<f64>()
            .map(|v| json!(v))
            .map_err(|_| bad("a number")),
        Kind::Bool => match raw {
            "true" | "1" | "on" | "yes" => Ok(json!(true)),
            "false" | "0" | "off" | "no" => Ok(json!(false)),
            _ => Err(bad("true or false")),
        },
        // A word that is not JSON (`drive`, `toggle`) is a string, so `drive.set gear drive` works.
        Kind::Json => Ok(serde_json::from_str(raw).unwrap_or_else(|_| json!(raw))),
    }
}

/// Builds the `params` object of `spec` from command-line words.
///
/// Words are positional values, `--name value`, `--name=value` or `name=value`; `--json '{...}'`
/// supplies a whole parameter object (explicit words override it).
///
/// # Errors
/// A readable message for unknown flags, bad values and missing required parameters.
pub fn build_params(spec: &CommandSpec, words: &[String]) -> Result<Value, String> {
    let decls = parse_decls(spec.params);
    let mut out = Map::new();
    let mut positional = 0usize;
    let mut i = 0;
    while i < words.len() {
        let w = &words[i];
        let (key, inline): (Option<&str>, Option<&str>) = if let Some(flag) = w.strip_prefix("--") {
            match flag.split_once('=') {
                Some((k, v)) => (Some(k), Some(v)),
                None => (Some(flag), None),
            }
        } else if let Some((k, v)) = w.split_once('=').filter(|(k, _)| is_name(k)) {
            (Some(k), Some(v))
        } else {
            (None, None)
        };
        if let Some(key) = key {
            if key == "json" {
                let raw = match inline {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        words.get(i).cloned().ok_or("--json needs a value")?
                    }
                };
                let v: Value = serde_json::from_str(&raw).map_err(|e| format!("--json: {e}"))?;
                let Value::Object(map) = v else {
                    return Err("--json must be an object".into());
                };
                for (k, v) in map {
                    out.entry(k).or_insert(v);
                }
                i += 1;
                continue;
            }
            let key = key.replace('-', "_");
            if spec.method.is_empty() {
                // Unknown (newer) method: take the pair as written; JSON if it parses, text otherwise.
                let raw = match inline {
                    Some(v) => v.to_string(),
                    None => {
                        i += 1;
                        words
                            .get(i)
                            .cloned()
                            .ok_or_else(|| format!("`--{key}` needs a value"))?
                    }
                };
                let value = serde_json::from_str(&raw).unwrap_or_else(|_| json!(raw));
                out.insert(key, value);
                i += 1;
                continue;
            }
            let decl = decls
                .iter()
                .find(|d| d.name == key || d.aliases.iter().any(|a| a.replace('-', "_") == key))
                .ok_or_else(|| format!("`{}` has no parameter `{key}`", spec.method))?;
            let raw = match inline {
                Some(v) => v.to_string(),
                None if decl.kind == Kind::Bool
                    && words.get(i + 1).is_none_or(|n| n.starts_with("--")) =>
                {
                    "true".to_string()
                }
                None => {
                    i += 1;
                    words
                        .get(i)
                        .cloned()
                        .ok_or_else(|| format!("`--{key}` needs a value"))?
                }
            };
            out.insert(decl.name.to_string(), coerce(decl.kind, decl.name, &raw)?);
        } else {
            let decl = decls
                .iter()
                .find(|d| d.position == Some(positional))
                .ok_or_else(|| format!("`{}` takes no value `{w}`", spec.method))?;
            out.insert(decl.name.to_string(), coerce(decl.kind, decl.name, w)?);
            positional += 1;
        }
        i += 1;
    }
    if let Some(missing) = decls
        .iter()
        .find(|d| !d.optional && !out.contains_key(d.name))
    {
        return Err(format!("`{}` needs `{}`", spec.method, missing.name));
    }
    Ok(if out.is_empty() {
        Value::Null
    } else {
        Value::Object(out)
    })
}

fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(method: &str, words: &[&str]) -> Result<Value, String> {
        let words: Vec<String> = words.iter().map(|s| (*s).to_string()).collect();
        build_params(find(method).expect("known method"), &words)
    }

    #[test]
    fn positional_and_flag_forms_agree() {
        let a = params("input.key", &["W", "--duration", "2000"]).unwrap();
        let b = params("input.key", &["key=W", "duration_ms=2000"]).unwrap();
        assert_eq!(a, json!({"key": "W", "duration_ms": 2000}));
        assert_eq!(a, b);
    }

    #[test]
    fn typed_values_are_coerced() {
        assert_eq!(params("sim.step", &["60"]).unwrap(), json!({"frames": 60}));
        assert_eq!(
            params("camera.teleport", &["1.5", "2", "3", "--yaw", "90"]).unwrap(),
            json!({"x": 1.5, "y": 2.0, "z": 3.0, "yaw": 90.0})
        );
        assert_eq!(
            params("debug.wireframe", &["on"]).unwrap(),
            json!({"enabled": true})
        );
        assert_eq!(
            params("shutdown", &["--force"]).unwrap(),
            json!({"force": true})
        );
    }

    #[test]
    fn json_flag_supplies_a_whole_object() {
        let v = params(
            "entity.spawn",
            &["--json", r#"{"asset":"a.sco","transform":{"x":1}}"#],
        );
        assert_eq!(v.unwrap(), json!({"asset": "a.sco", "transform": {"x": 1}}));
    }

    #[test]
    fn errors_are_readable() {
        assert!(params("sim.step", &[]).unwrap_err().contains("frames"));
        assert!(params("sim.step", &["many"])
            .unwrap_err()
            .contains("integer"));
        assert!(params("ping", &["x"]).unwrap_err().contains("no value"));
        assert!(params("input.key", &["W", "--nope", "1"])
            .unwrap_err()
            .contains("nope"));
    }

    #[test]
    fn new_commands_parse_words_into_typed_params() {
        // A JSON parameter that is not JSON is a string: `drive.set gear drive`.
        assert_eq!(
            params("drive.set", &["gear", "drive"]).unwrap(),
            json!({"control": "gear", "value": "drive"})
        );
        assert_eq!(
            params("drive.set", &["throttle", "0.5", "--hold_ms", "300"]).unwrap(),
            json!({"control": "throttle", "value": 0.5, "hold_ms": 300})
        );
        assert_eq!(
            params("drive.set", &["parking_brake", "false"]).unwrap(),
            json!({"control": "parking_brake", "value": false})
        );
        assert_eq!(
            params("ui.scroll", &["--target", "settings", "--to", "bottom"]).unwrap(),
            json!({"target": "settings", "to": "bottom"})
        );
        assert_eq!(
            params("ui.wait_for", &["distance_above", "--number", "100"]).unwrap(),
            json!({"until": "distance_above", "number": 100.0})
        );
        assert_eq!(
            params("ui.key", &["F", "--ctrl"]).unwrap(),
            json!({"key": "F", "ctrl": true})
        );
        assert!(params("ui.wait_for", &[]).unwrap_err().contains("until"));
    }

    #[test]
    fn every_enum_value_list_is_parsed() {
        for c in COMMANDS {
            for a in c.args_schema().as_array().expect("array") {
                // A declared value list is never empty and has no stray separators.
                if let Some(v) = a.get("values") {
                    assert!(
                        v.as_array()
                            .expect("values")
                            .iter()
                            .all(|x| x.as_str().is_some_and(|s| !s.is_empty())),
                        "{} / {}",
                        c.method,
                        a["name"]
                    );
                }
            }
        }
    }

    #[test]
    fn parameterless_commands_send_no_params() {
        assert_eq!(params("ping", &[]).unwrap(), Value::Null);
    }

    #[test]
    fn table_has_unique_names() {
        let mut names: Vec<_> = COMMANDS.iter().map(|c| c.method).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(n, names.len());
    }
}
