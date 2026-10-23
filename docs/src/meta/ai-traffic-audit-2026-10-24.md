# AI traffic audit (M-0111, phase A), 2026-10-24

Read from the code (`opts-traffic`, the traffic systems of `opts-game`) and checked by a headless
run at Ahlheim Hauptbahnhof with 400 vehicles. Paths below are relative to `crates/`; `T` is
`opts-traffic/src`, `G` is `opts-game/src/in_game/omsi_world`. Status: **present**, **partial** or
**absent**; only partial and absent rows become work.

| # | Wanted behaviour | Status | Evidence |
| :-- | :-- | :-- | :-- |
| 1 | A standing vehicle ahead is a leader although it is on another lane id | absent for agents, partial for foreign obstacles | `leader()` (`T/agent/gaps.rs:26-49`) looks along the planned lanes by lane id only. Foreign footprints (the player, parked cars) become phantoms on every lane whose centre line is within `width/2 + 0.95 m` (`T/obstacles.rs:80-135`). A dwelling bus that sticks into a neighbour lane is not seen laterally. |
| 2 | Parked vehicles are obstacles | partial → closed here for bay vehicles | `G/traffic/graph.rs` `register_parked_object` → `TrafficState.parked` → `set_obstacles_with_parked` (400 m around the camera). Only vehicles that replace a `carpark_p` marker are registered (`G/tile.rs`); other scenery vehicles are not. |
| 3 | Overtaking a stopped vehicle only when allowed | partial | No stopped-vehicle rule; the normal lane-change check applies. `no_overtaking` blocks only the left (`T/lanechange.rs:229`). Neighbours are same-direction lanes only, so no passing through the oncoming lane happens and no oncoming gap exists. |
| 4 | Lane change: indicator ≥ 2 s, gap check, abort, none in junctions | mostly present | Indicator 3 s (`ANNOUNCE_S`), gap checked at announce and at due (not continuously), abort with a 3 s cooldown, no change in junction lanes or conflict zones. No explicit "resume speed" state (IDM does it). |
| 5 | Yellow by comfortable braking, `[rowdy_factor]` raises passing, red running rare | yellow partial → closed here, red running absent | `T/agent/gaps.rs` stopped whenever 3 m/s² sufficed, for every driver; rowdiness only scaled speed. Now `IdmParams::rowdy` and `yellow_decel` (careful drivers 3.0 m/s², the rowdiest 0.9). Red is always obeyed. `[traffic_light_stop]`/`[traffic_light_jump]` have no effect. |
| 6 | Lane rules at spawn, routing and lane change | present at spawn and lane change; **holes in routing (closed here)** | `pick_next` filtered successors by class only, not by density group (`T/agent/routing.rs`), and a strategic target lane was not checked for closure. A lane closed to a group (`trafficdensity 0`, how bus lanes are marked) was entered by plain routing. Fixed with three tiers: open to class and group, then open to class, then any (never stranded). Test `a_fork_is_taken_on_the_lane_that_is_not_closed_to_the_cars_group` (fails without the rule). |
| 7 | Level-crossing barriers as obstacles | absent, no data hook | `SignalProgram` is clock driven; nothing reads an object's animation variable. Needs a data check of the crossing objects first. |

## Measurements

Ahlheim Hbf, 400 vehicles, 60 s: 8 of 60 vehicles stood on a lane closed to them. Seven of the eight
were scheduled buses of depot groups (group 0 closed on bus lanes); scheduled vehicles now belong to
no density group (`G/traffic/spawn.rs`). The eighth was a car on a 4 m connector lane at the moment
of the sample. Cars on bus lanes by routing are what the routing fix removes.

## Not done yet (rows 1, 3, 4 partly, 5 red, 7)

Lateral leader search for standing agents, passing rules, continuous gap evaluation, red running by
`[rowdy_factor]`, barriers. Counters for light violations do not exist.

## Counters

`traffic.scheduled` reports `hard_brakes` (a deceleration beyond 3 m/s² starting) and
`lane_change_aborts` (a due lane change found no gap). First reading, Ahlheim Hbf, `--traffic 150`
(only 2 agents were on the lanes around the player): 2 hard brakes and 0 aborts in 240 s. The sample
is too small to compare changes; a run with a populated area is needed for before/after numbers.
