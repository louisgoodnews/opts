//! Vehicle and camera reset, snapshot, batch and trace commands of the table, in protocol order.

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
    .doc("Same code as the key. Free roam returns to the pose recorded on entry (speed 5 m/s); external to yaw 0, pitch -15 degrees, zoom 14 m, no pan; ego looks forward; the driver's view (2) and the passenger view (4) keep their sub camera and look straight ahead, and view 2 returns to the stored viewpoint (an open editor is cancelled). Never changes the mode. Answers `1009` while a dialog or the vehicle menu is open."),
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
];
