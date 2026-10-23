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
