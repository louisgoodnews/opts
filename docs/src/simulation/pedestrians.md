# Pedestrians

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Walking pedestrians implemented (2026-10-11); crossings, stops, boarding, density schedule open |
| **Code** | `opts_sim::pedestrian` (lanes, network, walkers, gait), `opts-game` `omsi_world/pedestrians.rs` (characters, spawning, animation) |
| **Milestone** | [M-0050](../roadmap/phase-4/M-0050.md) |
| **Specs used** | [`.sli` paths](../formats/text/sli.md#path), [`.sco` paths](../formats/text/sco.md), [`.hum`](../formats/text/humans.md), [map lists](../formats/map/lists.md) |

## 1. Model

Pedestrians are **kinematic walkers on a footpath network**; they have no physics and do not collide (with each other, with vehicles, or with the player) yet.

**Footpaths.** OMSI marks walkable lines with `[path]` records of type `1`:
* in spline profiles (`.sli`): `offsetX` (right of the centre line is positive), `offsetZ`, `width`, `reverse` (`0` forward, `1` backward, `2` both ways). 3,725 of 15,291 installed profiles have such a path (sidewalks, paths);
* in scenery objects (`.sco`): twelve numbers per `[path]` — object-local start, heading, radius (`≈ −999.99` = straight), length, two slopes, type, width, direction (crossings, bus stops, junction plates).

`lanes_from_spline` and `lanes_from_object` sample both into polylines (1 m spacing) in the world frame; mirrored splines flip the lateral offset. Arcs use the same convention as the spline ribbons (positive radius turns right, heading clockwise from north).

**Network.** `PedestrianNet` holds the lanes of the resident tiles (inserted when a tile is built, removed when it is unloaded). Two lane ends belong to one junction when they are within 0.6 m horizontally and 0.8 m vertically (same geometric rule as the spline connections, [`tiles.md`](../formats/map/tiles.md)); a lane can be entered at an end only if its `direction` allows walking away from that end.

**Walker.** A walker has a lane, an arc length, a direction (`±1`) and a speed (1.25–1.55 m/s; children under 14 about 1.0–1.3, over 70 about 0.9–1.2 — a modelling choice, OMSI's speeds are not documented ❓). At a lane end it takes a random continuation; with none it turns round if the lane is two-way, otherwise it finishes and is removed.

**Gait.** One phase cycle is a double step of `schrittweite` metres (`[walk_param]`, [`.hum`](../formats/text/humans.md)): the phase advances by `2π · speed / schrittweite` per second. Thigh swing follows from the stride and the leg length (`step = 2·L·sin(a)` with `L` the hip height), the swing leg's knee flexes (up to 0.9 rad), the arms swing against the legs scaled by `armschwing` on top of the resting flare `upper_arm_beta`, the hips shift sideways by `walk_huefte`, the waist twists by `anim_walk_taille` (plus a small constant), the body bobs 2 cm. All amplitudes except the stride are modelling choices ❓.

## 2. Characters and population

* Characters are the `.hum` lines of the map's `humans.txt` (duplicates weight the draw), built with the same rig as the player's driver ([`opts_bevy::spawn_human`](../architecture/ego-perspective.md): 13 joints, one skinned mesh per material). A character whose mesh has no bone weights is rejected (`ERROR`) — a skinned draw call with a non-skinned mesh is a render validation error.
* Population: one pedestrian per 80 m of footpath in the loaded tiles, at most `--pedestrians N` (default 40, `0` disables). They appear on lanes 25–140 m from the camera and are removed beyond 175 m or when their lane's tile unloads. The time-of-day curve `[trafficdensity_passenger]` of `global.cfg` is **not** applied yet.

## 3. Verified

Unit tests: lane geometry (right of travel, arc length, object rotation), network continuation rules, walker transitions (cross lanes, turn round, finish), tile removal, spawn ring, gait symmetry/periodicity/stride. In game on *Bad Hügelsdorf 2020* (pedestrians walk along the plaza paths with the gait visible) and *Oberstedt 2* (12 characters; the `aXYZ` models without bone weights are rejected).

## 4. Open

| Topic | Note |
| :--- | :--- |
| Crossings and traffic lights | Pedestrian paths of a junction are walkable, but no signal phases are consulted (`[use_traffic_light]`) and AI traffic does not yield. Needs the AI-traffic session's signal model. |
| Bus stops and boarding | First slice done: see [Passengers](passengers.md) (stops, boarding at the scripts' doors, tickets, places, alighting). Open there: the callbacks for seat and link counts, the cash desk, trams and articulated buses, voices. |
| Collision avoidance | Walkers pass through each other; `width` of the lane is read but unused. |
| Density schedule | Apply `[trafficdensity_passenger]`; time-of-day, holidays. |
| Idle behaviour | Standing groups, benches, phone — OMSI's scenery uses static `people_standing*.x` objects. |
| LOD | Each pedestrian is 13 joints + skinned meshes; beyond ~60 m a billboard or frozen pose would be cheaper. |
