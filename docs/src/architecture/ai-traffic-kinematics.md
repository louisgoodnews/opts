# AI Traffic Kinematics

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟢 Implemented (C3, 2026-10-17); see "Implementation notes" below for deviations |
| **Date** | 2026-10-17 |
| **Crates** | [`opts-traffic`](../../crates/opts-traffic.md) (`agent.rs`, `idm.rs`, `traits.rs`, `lanechange.rs`, `graph.rs`, `geometry.rs`, `source.rs`), `opts-game` (`in_game/omsi_world/{traffic,traffic_chain,traffic_variant}.rs`), `opts-core` (vehicle model parser), `opts-bevy` (`vehicle.rs`, `vehicle_lights.rs`, `scenery_runtime/lights.rs`) |
| **Builds on** | [AI road traffic](ai-traffic.md), [AI vehicle configuration](ai-vehicle-config.md), [map tiles `[rule]`](../formats/map/tiles.md), [`.sco` `[path]`](../formats/text/sco.md) |

The traffic model stays kinematic: an IDM point moves along lanes and the body is posed from it. This page fixes how the body follows the lane, how fast a vehicle may go, when it changes lanes and how it shows lights. Numbers are from Berlin-Spandau: 7,339 directed road lanes (285.8 km), headless runs of the real lane graph (600 s simulated, 30 and 150 vehicles).

## 1. Lane geometry

* **Arcs for every spline radius.** Car lanes are straight for `|r| ≥ 999` (`geometry.rs`), foot/rail/bike lanes for `|r| ≥ 900` (`opts-sim` `lane.rs`), while the road mesh arcs. The end error is `L²/2R`: median 1.6 m, max 45 m; 41 road splines exceed the 0.75 m join tolerance and leave dead ends (373 → 279 when arced). `−999.99` stays the straight marker of `.sco` `[path]` records only.
* **`mirror`** is applied to car lanes as to the mesh (63 mirrored road splines).
* **Elevation** of car lanes uses the `[spline_h]` cubic ramp shared with the mesh (`spline_pose`), not a linear gradient (up to 3.8 m off).

## 2. Pose

Today the body is laid along the tangent at the front bumper (`agent.rs:126-131`, `traffic.rs:845-864`): in a curve of radius R it yaws `front_offset / R` ahead and the rear swings out by about `L²/2R` (W123 at R = 12 m: 10°, rear axle 0.5 m out); at lane joins the heading jumps by up to 15°.

New model:

1. **Front axle on the path:** `F = path(s − front_overhang)`; before the lane start, the previous lane (or the start tangent) is used.
2. **Rear axle as state:** `Agent.rear: DVec2`, moved each step as a tractrix, `R ← F − L · normalize(F − R)`, L = wheelbase. Reset to `F − L · tangent` on spawn, lane-graph rebuild or when `| |F − R| − L | > 0.3 m`.
3. **Body:** heading = direction `R → F`, origin = `R − dir · rear_axle_y`; front wheel steering = path heading at F − body heading. The artificial lane-change yaw is dropped (the tractrix produces it).
4. **Axle data** comes from the AI vehicle (`m.axles[*].longitudinal_m`, as `traffic_variant.rs:190-198` already does for trailers); fallback front axle = front_offset − 0.85 m, L = 0.6 · length. Trailers keep following the lead (`traffic_chain.rs`).

## 3. Speed

* **Driver factor, not an absolute speed.** Today every driver wants 50 km/h ± 10 % (`traits.rs`), capped for life by the spawn lane's limit (`traffic.rs:616`). New: a per-driver `speed_factor ≈ 1 ± spread`; on each lane `v_des = min(v_top, factor · limit)`, where `limit` is the lane's `[rule] speedlimit` (km/h; ≥ 200 = no limit, Spandau has one 250) or the open-road default of 50 km/h.
* **Curve limit:** lanes are constant-radius arcs, so `v_c = √(a_lat · |R|)` per lane with `a_lat` = 2.5 m/s² for cars and 2.0 m/s² for trucks and buses, scaled slightly by the driver. Today 22 % of samples on arcs exceed 2.5 m/s², max 46 m/s².
* **Look-ahead:** over the current lane and the planned lanes up to `v²/(2·b_comf) + 15 m`, `v_cap = min √(v_lim² + 2 · b_comf · d)` with `v_lim = min(limit, v_c)` of each lane at distance d; `v_cap` caps the IDM desired speed (`IdmParams::acceleration`). Vehicles brake before a bend or a lower limit instead of in it.

## 4. Lane changes

The MOBIL-like decision of `lanechange.rs` stays. Measured suppression at 150 vehicles (44,921 evaluations): junction gate 58 %, no neighbour 36 % (single-lane roads, legitimate), `trucks` rule 2,776 of 4,552 candidates, density-0 lanes 567, below threshold 603. Result: 65 changes in 10 minutes on 6 distinct lanes.

1. **`trucks` permits trucks, it does not exclude cars.** On Spandau the pattern is a right lane with `trucks` and density 1 beside a left lane without it and density 3 (723 neighbour pairs): the rule marks the lane trucks may use. `LaneRules::allows` stops excluding cars; trucks prefer `trucks` lanes and keep right. (Reverses an M-0049 project reading; [`tiles.md`](../formats/map/tiles.md) and [ai-traffic](ai-traffic.md) are updated with it.)
2. **Junction gate by distance, not by lane.** A change is blocked only inside a conflict zone or junction-object lane, or within `max(25 m, 2.5 s · v)` of the next zone entry or stop line ahead on the own or the target lane; zones already passed do not count.
3. **Strategic changes.** Route planning (`extend_plan`) may continue from a neighbour lane's successor; the change then becomes mandatory before `len − 15 m`, with an urgency bonus growing as the distance shrinks. If no gap appears, the vehicle waits at the end of its lane and re-plans after a timeout.
4. **Announcement** grows from 1.2 s to 3 s.

Headless effect (all of 1-3 with arcs, 40 m gate): 16 → 85 changes at 30 vehicles, 65 → 335 at 150.

## 5. Indicators and lights

* **Content:** all 11 AI vehicles of Spandau's `ailists.cfg` light only through `[light_enh_2]` blocks bound to `AI_Light` (headlights, tail lights), `AI_Brakelight` and `lights_blinker_l/r`; `Scripts/AI_Cars/main_AI.osc` maps `AI_Blinker_L/R` to the blinker variables. OPTS already sets these variables (`traffic.rs` `update_ai_variables`), but the vehicle model parser drops `[light_enh_2]`, so nothing is drawn (the reason for "no AI lights" and for invisible indicators).
* **Turn indicators from content:** `.sco` `[path]` parameter 12 is the indicator of a crossing path: 0 none (2,186 of 2,554 straight paths), 2 left (530 of 727 left turns), 3 right (565 of 701 right turns); value 1 is open. It replaces the 20°/30 m heuristic, which stays as fallback.
* **Rendering:** parse `[light_enh]`/`[light_enh_2]` in the vehicle model with the scenery parser (`parse_light_enh_2`), spawn the glow sprites under the vehicle root (not under a LOD group) for player and AI vehicles, brightness = variable × factor with the block's time constant. A pool of real, shadowless `SpotLight`s goes to the nearest six headlight pairs at night, with hysteresis. Brake lights also stay on while standing.

## 6. Tests

`opts-traffic` (existing patterns in `agent.rs`, `idm.rs`, `neighbours.rs`, `geometry.rs`, `source.rs`, `traits.rs`):

* R = 12 m arc: both axles within 0.1 m of the lane, speed ≤ √(2.5 · 12) + 0.3 m/s, braking starts before the arc;
* 12° lane join: no heading jump above 2° per step;
* a vehicle brakes before a 30 km/h lane 50 m ahead and does not exceed it;
* a car changes into a `trucks` lane; a change is possible 40 m after a passed merge zone; a mandatory change happens before a turn pocket;
* spline lane with r = 1100, L = 183 ends within 1 cm of the mesh end; mirrored lane; `[spline_h]` elevation;
* `[path]` parameter 12 → blinker side.

`opts-core`/`opts-bevy`: `[light_enh_2]` in a vehicle model parses; the sprites sit under the vehicle root; the spot-light pool picks the nearest pairs and is stable under small camera moves.

## Implementation notes (C3)

* **Mirror also reverses direction.** Negating the lane offsets alone left more dead ends (336) than ignoring `mirror` (279); flipping sides *and* directions gives 178 (Spandau, arced). Implemented that way and recorded in [`tiles.md`](../formats/map/tiles.md#mirror-flag-and-absheight-objects-2026-10).
* **Pose:** the front axle pursues the path 1 m ahead (rounds join kinks), the rear axle follows in 0.2 m sub-steps. The rear axle off-tracks *inside* the lane by up to `R − √(R² − L²)` (0.29 m at R = 12 m for a 2.64 m wheelbase), which is physical; the R = 12 m test checks the front axle within 0.1 m and the rear axle between that and the centre line. A 12° join stays under 2° per 0.1 s step at junction speed (25 km/h); at 47 km/h a kink still turns the body by up to ~3.5° per step (exponential over about one wheelbase of travel).
* **Speed:** the look-ahead cap alone lags (the IDM free term needs ~20 % overspeed to brake at 1.6 m/s²), so a vehicle faster than a lane speed ahead also brakes explicitly once it needs half its comfortable deceleration (it reaches the lane speed 2 m before the lane). `speedlimit` ≥ 200 km/h counts as none; default top speed 130 km/h cars, 80 km/h heavy.
* **Lane changes:** "junction-object lane" = a lane from a `.sco` path; no change on one or onto one (except a strategic change). The strategic route choice only offers neighbour successors that are new directions (not successors of the own lane nor parallel to one); after 10 s waiting at the lane end the vehicle re-plans without them.
* **Headless, Berlin-Spandau, 600 s (30 / 150 vehicles):** lane changes 20 / 93 → 31 / 123 (64 / 236 without the object-lane rule; the estimate 85 / 335 above did not exclude object lanes); lateral acceleration on arcs p99 34 / 30 → 2.6 / 2.6 m/s², max 46 / 44 → 7.7 / 10.6, share above 4 m/s² 18 / 15 % → 0 %; dead-end removals 148 / 509 → 67 / 226; dead-end lanes 373 → 178.
* **Lights:** headlights for the pool are forward-facing (`dir.y > 0.7`), white (all channels ≥ 150) `[light_enh_2]`; one spot light per vehicle (at the middle of the pair), not two.

## ❓ Open questions

* **Q-AK-01** Meaning of `[path]` parameter 12 value 1, and of the sixth value of `.sli` `[path_2]`.
* **Q-AK-02** Is 250 km/h the editor's "no limit"? (Treated so.)
* **Q-AK-03** 279 dead-end lanes remain with arcs (helper arms or tolerance): separate audit.
* **Q-AK-04** Lane changes between density-0 lanes (567 rejections) stay forbidden.
