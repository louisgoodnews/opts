//! Drive-controls commands of the table, in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
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
];
