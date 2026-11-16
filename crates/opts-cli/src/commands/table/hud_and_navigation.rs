//! UI navigation, HUD and passenger commands of the table, in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
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
        "hud.fullmap",
        "scene",
        "Read or drive the full-size map: open or close, zoom span, centre, follow",
        "open:b? span:f? x:f? y:f? follow:b? fit:b?",
    )
    .since("1.15")
    .result("`{open, follow, span_m, centre, px_per_m, screen, texture_px, redraws, last_us, worst_us, road_chords, markers, min_span_m, max_span_m, overview}`")
    .doc("The full-size map of the session (key F6). Without parameters it reads. `open` opens or closes it (the change happens in the next frame; opening is refused silently while a dialog, the pause overlay or the vehicle menu holds the input, or before the session has loaded). `span` is the number of metres across the shorter side of the screen, limited to `min_span_m`..`max_span_m` (50 m to 20 km); `x` and `y` (east and north in OMSI metres) centre the map and stop following the vehicle; `follow` follows the active vehicle (or the camera) again; `fit` shows the whole map (every lane read so far, with a margin; the End key does the same). `centre` is the world point in the middle of the screen, `px_per_m` the scale in logical screen pixels, `screen` the logical window size. `last_us` and `worst_us` are the time of the last and the slowest redraw (a redraw follows a change of the view at once, a moving vehicle at most ten times a second, a still map twice a second). Mouse drag, wheel and Home work through `input.mouse_*` and `input.key` like for a person. `overview` is the reader of the roads of the whole map (`{started, complete, percent, tiles_done, tiles_total}`): the first opening starts a background thread that reads every tile of the map, and the map draws these roads for all tiles that are not resident while it fills in. `1001` outside a drive."),
    c(
        "camera.door",
        "scene",
        "Read the door camera or choose the door it shows",
        "door:i?",
    )
    .since("1.17")
    .result("`{enabled, doors:[{number, kind, zone, side, pathpoint, position}], manual, auto, shown, renders, picture}`")
    .doc("The door camera of M-0109: a picture-in-picture view from a door of the player's vehicle at the top right of the HUD. It exists only while the setting `camera.door_overlay` is on. `doors` are the pathpoints that the vehicle's passenger cabin names with `[entry]` and `[exit]` (`position` in the vehicle's Bevy frame); `manual` is the door chosen with F4 or with `door` (a number from the list, `0` for none), `auto` the door whose leaf animates, `shown` the door in the picture (`manual` wins), `renders` the pictures rendered so far (30 per second while shown). `-32602` for a door number outside the list."),
    c(
        "vehicle.passengers",
        "drive",
        "Read the passengers of the player's vehicle, or send some to its doors",
        "spawn:i? arrive:b? entry:i? mass_count:i?",
    )
    .since("1.25")
    .result("`{inside, mass: {source, count, on}, approaching, leaving, at_stop, stops_served, seats, standing_places, on_places, entry_requests, exit_requests, revenue, tickets_in_pack, inside_per_unit, passengers:[{id, age, phase, ticket, stamped, stops_left, unit, omsi_position, figure_root}]}`")
    .doc("The passengers of M-0101. They walk up to the doors when the bus stands at a bus stop (an object whose `.sco` says `[busstop]`), board when the scripts report the door open (`PAX_Entry<n>_Open`), buy a ticket of the map's ticket pack with its probability, stamp, take a seat or stand, and get off through an open exit (`PAX_Exit<n>_Open`) when their ride is over. `arrive: true` counts a stop reached (those whose ride is over head for the nearest exit and get off when it opens). `spawn n` sends up to 400 people (the first 24 are drawn walking up, the rest board unseen) to the first entry (or to entry number `entry` of the chain, counting the units' entries one after the other) now, stop or no stop (a test aid; they count as a stop visit: those who ride one stop get off). `inside` counts the people in the cabin, `on_places` the people on each `[passpos]`, `revenue` the money of the tickets sold (main currency unit), `passengers` the people inside with their `phase`. `mass_count n` makes the vehicle's dynamics carry `n` people (0 to 400) instead of the real ones, `-1` goes back to the real ones (M-0124; `mass.source` says which counts; `mass.on` is the setting `gameplay.passenger_weight`, `mass.by_unit` the real people in each unit of the chain; a consist's units each carry their own people, a test count is shared by curb mass). `-32000` (not ready) without a drive or for a vehicle without a passenger cabin.")
    .debug(),
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
];
