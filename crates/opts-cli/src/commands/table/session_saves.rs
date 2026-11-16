//! Session load, save listing and deletion commands of the table, in protocol order.

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
    .result("without `values`: `{context, repaint, family, count, options: [{name, label, description, kind, min, max, values, value, default, group, declared_in, evidence, range_unverified, respawn}], not_offered}`; with `values`: `{context, count, changed: [{name, previous, value}], live}`")
    .doc("The options of the vehicle in a drive (its player vehicle) or, outside a drive, of the vehicle chosen in *Plan a drive* once its repaints are discovered; `1001` when there is none. Each option has a `kind`: `switch` (0 or 1), `choice` (one of `values`), `number` (a whole number from `min` to `max`) or `fraction` (decimal, listed but not settable). `default` is the value of the chosen repaint; `evidence` tells whether repaints set it, the vehicle declares it and the vendor documents it; `range_unverified` marks a documented range that is narrower than what repaints set. `respawn` says whether a change reaches a vehicle that exists (protocol 1.31): `required` (the vendor documents that the vehicle must be built again), `live` (a live test showed the running vehicle reacts) or `unknown` (the default; nobody has settled it for the option). Only variables the vehicle declares are offered (`not_offered` names the others). `values` is an object of option name to value: the whole batch is validated and applied all or none (`-32602` with `data.errors: [{key, code, message}]`). In the wizard a change is a choice of the plan, like the equipment dialog; in a drive it is set on the vehicle's scripts at once (`live: true`: a script that reads the variable only at start shows no change) and kept for a respawn. A name the vehicle does not offer is refused unless `allow_unlisted` is true."),
];
