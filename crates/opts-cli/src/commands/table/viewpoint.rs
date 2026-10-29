//! The viewpoint editor's commands of the table (`camera.viewpoint.*`, M-0084).

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
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
        "camera.viewpoint.export",
        "scene",
        "Write the stored viewpoints (all vehicles and models) to a file to share",
        "path:s@0",
    )
    .since("1.30")
    .result("`{file, vehicles, models}`: the file written and how many vehicle and model entries it holds")
    .doc("The file has the format of `viewpoints.json` (offsets from the vehicle's own driver camera, so it works on any installation that has the vehicle). `path` must be below the reports directory or an allowed directory (`1002` otherwise) and its directory must exist (`1004`). Writes through a temporary file. Works in any view: it exports what is stored, not the open editor."),
    c(
        "camera.viewpoint.import",
        "scene",
        "Merge the viewpoints of a shared file into the stored ones",
        "path:s@0",
    )
    .since("1.30")
    .result("`{file, vehicles, models, replaced, problems}`: entries taken, entries that replaced a different stored value, and the entries of the file that were dropped (with the reason)")
    .doc("Reads the format `camera.viewpoint.export` writes. The entry of the file wins over a stored one for the same vehicle (or model) and camera; everything else stays. The store file is written before the answer and the vehicle in the driver's seat shows an imported value from the next frame. Invalid entries are dropped and listed in `problems`; a file that cannot be used at all answers `-32602` and is left where it is. `path` must be below the OMSI root or an allowed directory (`1002`), and exist (`1004`). Answers `1009` while the viewpoint editor is open."),
];
