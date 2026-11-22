//! The basic commands of the table: meta, queries, the UI tree and input, in protocol order.

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
        "token:s@0?",
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
];
