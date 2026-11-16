//! Localisation report, vehicle setup, reload, watch and debug commands of the table, in protocol order.

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
];
