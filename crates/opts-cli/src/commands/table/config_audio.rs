//! Config, vehicle hotspot, mouse, audio and menu commands of the table, in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
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
        "vehicle.hotspots",
        "query",
        "The cockpit controls of the player's vehicle that react to the mouse (hot-spots)",
        "contains:s@0?",
    )
    .since("1.20")
    .result("`{count, hotspots:[{name, meshes, position}], hovered}`")
    .doc("`name` is the `[mouseevent]` trigger name a click raises (`<name>`, then `<name>_drag` while the mouse moves and `<name>_off` on release); `meshes` counts the meshes that carry it, `position` is the first one's place (Bevy metres). `contains` filters by a substring of the name (case-insensitive). `hovered` (1.21) is the name of the control under the mouse cursor in the cockpit and passenger views, `null` when none (the HUD shows the same label). `1001` without a player vehicle."),
    c(
        "input.mouse.position",
        "input",
        "Put the cursor at a window position without clicking",
        "x:f@0 y:f@1",
    )
    .since("1.21")
    .result("`{position:[x, y]}`")
    .doc("Physical window pixels, like `x`/`y` of `input.mouse.click`. In the cockpit and passenger views the control under the cursor shows in the HUD and as `hovered` of `vehicle.hotspots`; a real mouse movement moves the cursor on. `1001` without a window."),
    c(
        "vehicle.click",
        "drive",
        "Operate a cockpit hot-spot like a player: press, drag, hold, release",
        "name:s@0 drag_x:n? drag_y:n? frames:i? hold_ms:i?",
    )
    .since("1.20")
    .result("`{queued, name, drag, drag_frames, hold_ms}`")
    .doc("Raises `<name>` (press), then `<name>_drag` on each of `frames` frames (default 10) while `mouse_x`/`mouse_y` carry the share of `drag_x`/`drag_y` pixels of that frame, holds the button for `hold_ms`, then raises `<name>_off` with the mouse values at 0. The same triggers and system values a real click makes, so it works in any camera. A control that reads the mouse position in steps (the ignition key of the Citybus 530 turns by `mouse_y / -75` per drag call) needs a drag, and the ignition key two (to the drive position, release, then to the starter and hold; see the start-procedures page). The command returns at once; poll `vehicle.script_vars`. `1004` for a name that is not a hot-spot of the vehicle."),
    c(
        "audio.state",
        "query",
        "The sound engine: volumes, voice budget, listener and the loudest emitters",
        "",
    )
    .since("1.19")
    .result("`{enabled, volumes:{master, effects, ambience, engine, interface}, voices:{budget, emitters, audible, with_voice, virtualized, playing}, listener, clips_cached, clips:{ready, loading, failed, bytes}, scenery:{voices, playing, one_shots, cold_misses, unsupported_conditions, files_not_found}, traffic:{voices, playing}, player:{voices, loops, one_shots, cold_misses, unsupported, files_not_found, listener_inside}, top:[{entity, bus, gain, distance, voice}]}`")
    .doc("`gain` is the level an emitter computes: its own gain × the distance fade × its bus' volume × the master volume (volumes are percent, squared into a gain). `voices.playing` counts voices whose sink exists, that is the sound card accepted them: on a machine without an output it stays 0 while `with_voice` rises. `listener` is the main camera's position (Bevy metres) or `null`. `clips` counts the sound files the worker threads decoded, are decoding or could not decode, with the bytes held; `scenery` reports the sound definitions of placed objects (M-0139): all of them, the continuous ones playing now, the one-shots started by script triggers, triggers that found their file still decoding, definitions waiting for a `[conditionSingle]` (not supported yet) and files not found; `traffic` the continuous sound voices of AI vehicles (engine, rolling noise) and how many have an emitter now (M-0140); `player` those of the player's vehicle (M-0102): voice entities, loops with an emitter, one-shots started by script events or conditions, definitions left out for an unsupported condition mode or a missing file, and whether the listener is inside the vehicle."),
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
