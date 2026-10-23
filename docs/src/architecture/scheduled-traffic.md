# Scheduled Traffic

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Road buses implemented (2026-10-23); trams and trains on rails, doors and passengers open |
| **Crates** | `opts-traffic` (`schedule.rs`, `agent/stops.rs`, `LaneGraph::nearest_place`), `opts-game` (`in_game/omsi_world/traffic/scheduled.rs`, `control_scheduled.rs`), `opts-omsi` (`AiLists::depot_groups`) |
| **Related** | [M-0112](../roadmap/phase-11/M-0112.md), [AI traffic](ai-traffic.md), [timetable files](../formats/text/timetable.md) |

A timetable run is a trip (its stops and times) started at a time of the day. This page is how the game turns the runs of the map's `TTData` into vehicles on the road.

## What drives a run

* **The vehicle.** A tour names a `vehicle_group` (`AVG Solowagen (Kranenburg)`), which is a depot group of `ailists.cfg` (`[aigroup_depot]` and the `[aigroup_depot_typgroup_2]` blocks after it, `AiLists::depot_groups`). On the installed maps 121 of 137 (Berlin-Spandau), 825 of 881 (Ahlheim), 59 of 67 (Köln) and 107 of 117 (Bremen-Nord) tours name a depot group with vehicles; the rest are trains, ships, or a spelling the lists do not share. The vehicles of the group wait as *idle* slots of the traffic pool and are parsed when a run first needs one; the choice among them is by how often the list names a vehicle. They never drive as unscheduled traffic.
* **The stops.** The bus stop objects of the map (`BusStopMarker`) carry the station id of the timetable as their ID code. A stop is the lane the object stands beside (`LaneGraph::nearest_place`: within 9 m, heading within 37°) at the place along it.

## Which runs, and where

`opts_traffic::schedule::active_runs` lists the runs on the road at the session clock or starting within 90 s (a run's start within 3 hours before or 90 s after the clock; a run that crosses midnight is found a day later too). `run_phase` says where a run is by its times: not started, standing at a stop, or between two stops by what fraction of the time. The game plans **one run per second** whose vehicle does not exist yet:

* the run's first (or current) stop must lie on a lane between 40 and 280 m from the camera, not in plain view when closer than 150 m; and for a run between two stops the next stop too;
* the vehicle appears at the stop (a run about to start, or standing), or on the way: the distance along the shortest lane route between the two stops by the fraction of the time;
* it gets the **fixed route** through all following stops that lie on the loaded lanes and a **stop point** at each, with the dwell of `Busstops.cfg` (8 to 45 s) and the scheduled departure;
* at most 24 scheduled vehicles at once; a vehicle that leaves the area (the 320 m despawn) frees its run, which gets a vehicle again when its stops are near the player.

## What the engine does

`opts_traffic::StopPoint` and `TrafficWorld::set_route` / `push_stop`: the plan follows the route instead of choosing at random; the first stop acts as a stop line for the driver model (the vehicle comes to rest with its front at the stop point); it **arrives** (event with the clock), stays at least the dwell and until the scheduled departure, then **departs** (event) and goes to the next stop. A stop whose lane is neither the vehicle's, nor in its plan, nor on the rest of its route is dropped (`StopMissed`). A vehicle waiting at its stop is not "stuck" (no forced despawn).

## Observing it

`traffic.scheduled` (control protocol): runs on the road, vehicles driving them, stops reached and left, dropped stops, the delay at arrival (clock minus scheduled arrival; early is negative) of the last arrivals. On Ahlheim Hauptbahnhof at 12:00 the first minutes bring buses that arrive at their stops within seconds of the timetable and leave at the scheduled departure.

## Not yet

* **Trams and trains.** They run on rails (`RailAiPool`, `.zug`), which have no lane graph: the same scheduler logic fits, the stop controller for rails does not exist.
* **Doors and passengers** of scheduled vehicles stay closed and empty; the AI vehicle script gets no stop request.
* **Destination and line text** are not fed to the vehicle's display variables.
* **Day validity** of tours (weekday, holiday) is not read: every run of the timetable counts.
* Vehicles appear only near the player and only for stops in loaded tiles; a run is not simulated beyond that.

## The player's schedule

A followed run (`route.select_run`) advances by itself: when the player's vehicle stands (below
1 km/h) within 18 m of a bus stop object whose ID code is the station id of one of the next three
stops of the run, `stop_index` moves to that stop (`route_progress.rs`). The delay is the session
clock minus that stop's scheduled arrival and `GetTTStopIndex`/`GetTTDelay` answer from both. A
HUD line for next stop and delay is still open.

### The route on the maps

The mini map and the large map draw the followed run in green: the road path from the current stop
through the stops still ahead (up to 14) and a ring at each stop, larger at the next one
(`hud/route_path.rs`, `PlayerRoutePath`). Stop positions come from the bus stop objects: those of
the resident tiles at once, all others from the background reader of the large map's overview,
which is started as soon as a timetable is followed. Between two stops the path follows the lane
graph where both stops are on resident tiles; a leg whose tiles are not resident is drawn as a
straight connector until the bus is near. Without a followed timetable nothing is drawn.

## Scheduled vehicles on the maps

Both maps draw each scheduled vehicle as a small filled green arrow (the route's colour), apart
from the hollow arrows of the session vehicles (`ScheduledVehicles`, `hud/route_path.rs`).

## Which runs run today

Line files carry no calendar, so the file name decides (`line_runs_on`): `Mo-Do`, `Freitag`,
`Samstag`, `Sonn- und Feiertag` give the weekdays of the session's date; a name without a day runs
every day. Files of the school holidays (`Ferien`) are not run, because the game knows no school
calendar and the term-time file of the same days runs instead; `Feiertag` files run on Sundays
only (public holidays are not known).

## The end of a run

A scheduled vehicle that stands at the last stop of its run is removed 8 s later if it is at least
80 m from the camera. Nearer, it drives on as ordinary traffic and is removed when it is far
away like any other vehicle, so nothing vanishes in view.

## Trains and trams

A tour whose vehicle group holds rail vehicles (`.zug` consists, or the trams of a depot group)
runs on the track network, not the lane graph (`rail/scheduled.rs`). The stops are the same bus
stop objects (platforms are `[busstop]` objects too); each is put on the nearest track within
14 m. The train follows the shortest way over the tracks through the stops on loaded tiles
(`PedestrianNet::shortest_path`; `Train::route` decides at the points), brakes to stand with the
centre of the train at the stop object, stays for the dwell (15 to 60 s) and until the scheduled
departure, and is removed 8 s after its last stop if the camera is 80 m or more away. A run
whose previous stop is not on loaded track starts upstream of its next loaded stop, as far back
as the time to its arrival at 15 m/s (at most 450 m). Over one run at Ahlheim Hauptbahnhof an S8
arrived at its stop 14 s after the timetable.

Trains and trams have no scripts yet (see below): their doors stay shut.

## Scripts of scheduled road vehicles

An unscheduled AI vehicle is a table of animation variables. A scheduled road vehicle gets its
script engine (`traffic/ai_script.rs`, the one-script path of M-0103 for the vehicles that need
it): the head unit's `[script]` set, with the depot (`.hof`) and the run's timetable behind the
`GetTerminus*`/`GetTT*` callbacks. Every 0.1 s within 150 m of the camera it is fed like a
player's vehicle (`opts_sim::script::inputs::feed`, with `AI` and `AI_Engine` set and the route
for `AI_Target_Index`), the traffic's own animation variables are copied in unless a script
writes them, and `{frame}` runs (OMSI's AI main scripts do their work in `{frame}`).

The door protocol, read from the scripts (`door_3D_G.osc`, `MAN_12C_main_AI.osc`): the host sets
`AI_Scheduled_AtStation` to 1 when the vehicle stands at its stop (the scripts open the doors; MAN
scripts want `PAX_Entry*_Req`/`PAX_Exit*_Req`, which the host sets for the same time), to -1 when
the passengers are through (half of the dwell; the doors close) and back to 0 when the vehicle
leaves. The dwell is at least 15 s so that the doors have time to open and close. In a run at
Ahlheim a Mercedes articulated bus opened both door pairs to 1.0 within a few seconds of
arriving, closed them again before leaving, and its destination display was handed terminus row
113 of the depot. No passengers walk to or from the vehicle (they do not exist for AI vehicles).

Not done: the destination text on the model (the display's script texture) was not looked at on
screen, only the variables; and the same for trains and trams, whose units are built from a
different path (`rail/ai.rs`).

## The HUD

While a timetable is followed, the clock pill of the status bar adds the line, the next stop with
its scheduled arrival and the delay (`Time 12:00:50 | Weather: … | Line S8: next Listau S Gl. 4
5:30 (+0:14)`), and the status bar stays visible instead of fading. The delay is the one when the
vehicle came to its current stop (the live value keeps growing while it drives on).
