# Density Baselines

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented (2026-10-23) |
| **Crates** | `opts-game` (`in_game/omsi_world/{traffic,pedestrians}`, `passengers`, `settings_registry`), `opts-sim` (`passengers`) |
| **Related** | [AI traffic](ai-traffic.md), [passengers](../simulation/passengers.md), [`global.cfg`](../formats/map/global-cfg.md) |

How many vehicles, people and passengers there are is **a baseline of OPTS, adjustable in the settings, and the map's factors are applied to it**. The map supplies factors (`global.cfg` `[trafficdensity_road]` and `[trafficdensity_passenger]` by the hour, the group curves and lane rules of AI traffic); it does not decide the amount.

| Setting | Unit | Default | Baseline of … | The map's factor |
| :--- | :--- | ---: | :--- | :--- |
| `gameplay.traffic` | 0–1000 | 150 | roaming AI vehicles around the player (within the despawn disc, 320 m) at factor 1 | `[trafficdensity_road]`, each group's curve and day type, the lane rules (the baseline is spread over the groups by their density-weighted lane metres) |
| `gameplay.parked` | 0–100 % | 100 | the share of the parking bays (`carpark_p` markers) that hold a vehicle | none (a bay either holds a car or not; the same bays stay empty on every load, a lower share only empties more of them) |
| `gameplay.pedestrians` | 0–1000 | 120 | people walking around the player at factor 1 | `[trafficdensity_passenger]` |
| `gameplay.passenger_rate` | 0–1000 per hour | 120 | people who come to a stop per hour at factor 1 | `[trafficdensity_passenger]` |

`--traffic N` and `--pedestrians N` on the command line set the first and third for one run.

## Limits the roads and paths set

A baseline the network cannot hold is scaled down, never packed: AI vehicles keep one per 25 m of density-weighted lane (`MIN_LANE_SPACING_M`; all groups are scaled in proportion), pedestrians one per 8 m of footpath in the loaded tiles. A short street therefore does not receive the whole baseline.

## Passengers at stops

People gather at a stop at `passenger_rate × factor(hour)` per hour while no vehicle serves it (Poisson draws, at most 40 minutes of arrivals between two visits; a stop visited for the first time has gathered 8 minutes). A vehicle that cannot take everybody leaves the rest at the stop for the next one. Up to 24 people are drawn waiting at a time; the rest of a crowd is counted and boards through the open doors without a figure (about one person per door every 0.35 s), as do people beyond 90 drawn figures inside one vehicle. The per-stop state is `StopCrowds`.

## Capacity

A vehicle takes `capacity()` people: every place of its cabin (`[passpos]`, seats and standing places) plus the standing room the simulation adds along the aisles (`opts_sim::passengers::standing_room`: a single file in a narrow aisle, two columns in a wide one, not on stairs, not at the doors). Boarding stops at capacity; people take a seat with a chance of 0.7 and stand otherwise, a seat when no standing place is free and the other way round, so a full vehicle has every seat and every standing place taken. On the installed cabins with an entry the added room is small where the authors listed their standing places (the Lion's City articulated bus holds 101 with 47 seats), and 15 to 60 places where they did not.

Tests: `opts-game` `in_game/omsi_world/traffic/density_tests.rs` (baseline spread, factors, limit), `job/lookup.rs` (bay occupancy), `opts-sim` `passengers/tests.rs` (standing room, every place taken, Poisson draws), `tests/passenger_capacity.rs` (the install's cabins).
