//! The spatial zones of the loaded tiles and the membership of the tracked subject (M-0115).

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[c(
    "zones.query",
    "scene",
    "The spatial zones (tunnels) of the loaded tiles and whether the tracked subject is inside one",
    "limit:i?",
)
.since("1.36")
.result("`{count, zones, subject}`: `count` is every zone, `zones` the first `limit` of them (`{id, kind, min, max}`, `kind` is `tunnel`); `subject` is `null` until the subject has a sample, else `{point, inside, zone}` with `zone` `null` outside every zone")
.doc("Read only. The zones are the entities the zone registry built at tile load; a tile that is not resident has none. The subject is the entity with the zone membership sampled at 10 Hz (the player's vehicle or the camera once one is tracked, M-0115). `limit` defaults to 100 and does not change `count`.")];
