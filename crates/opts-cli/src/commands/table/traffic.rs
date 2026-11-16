//! Traffic commands of the table: the AI vehicles on the lanes and the baseline of the traffic.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c(
        "traffic.dump",
        "drive",
        "The AI vehicles on the lanes (unscheduled and scheduled): lane, position, speed, waiting and stuck time, pose",
        "limit:i?",
    )
    .since("1.33")
    .result("`{available, baseline, lanes, agents, shown: [{id, lane, s, speed_ms, accel, waiting_s, stuck_s, changing_lane, lateral, x, y, heading_deg}]}`")
    .doc("The first `limit` vehicles (default 50, at most 500) of the traffic world by id order, `agents` counts all of them. `lanes` is the size of the lane graph of the resident tiles, `baseline` the traffic setting (`gameplay.traffic`, or the value `traffic.set_density` gave). `s` is the front bumper's position along its lane (m), `waiting_s` the time spent at a conflict zone, `stuck_s` the time standing without a red light ahead. `x`, `y` and `heading_deg` are `null` when the vehicle's lane is gone. `available` is false outside a drive (no traffic world). M-0049."),
    c(
        "traffic.set_density",
        "drive",
        "The baseline of the AI traffic around the player, for this session (`0` stops new vehicles)",
        "baseline:i",
    )
    .since("1.33")
    .result("`{baseline, previous}`")
    .doc("Sets the number the spawner aims at within the despawn disc (the setting `gameplay.traffic`, default 150, at most 1000); the map's density factors still shape it. The value lasts until the next map load, which takes the setting again, and is not saved. Vehicles already on the road leave by the usual distance rule. `1001` without a traffic world (start a drive first). M-0049.")
    .debug(),
];
