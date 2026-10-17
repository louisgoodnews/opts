# Berlin-Spandau: terrain, traffic and environment systems (2026-10-17)

| Metadata | Value |
| :--- | :--- |
| **Map** | `OMSI 2/maps/Berlin-Spandau` |
| **Brief** | expanded scope: 19 symptoms in five systems (terrain/coordinates/tags, parser and Chrono, AI kinematics and lights, asset gaps, environment UI) |
| **Branch** | `fix/terrain-and-traffic-systems` (worktree `opts-wt-berlin-spandau-systems`, from `master` `85d2391`), carries the two report commits of `fix/map-fidelity-systemic`; not merged, merge only after user acceptance |
| **Status** | Phase A done (Checkpoint 1: premise corrections accepted, pause-menu environment page, Phase B scope); Phase B done, waiting for Checkpoint 2 |
| **Predecessor** | [Berlin-Spandau map fidelity investigation](berlin-spandau-2026-10-17.md): symptoms 1-13 reproduced and root-caused there (§2, §6), fix plan §7 |
| **Clean room** | no OMSI binary inspected; screenshots and probes kept locally in `~/.local/share/opts/captures/berlin-spandau-2026-10-17/` (`systems/` for this phase), not committed |

## 1. Triage of the new symptoms (A1)

Same session setup as the predecessor (release build of `master`, isolated config, control port, `OPTS_DEV_HOUR` for night).

| # | Symptom | Evidence | Reproduced | Finding |
| ---: | :--- | :--- | :--- | :--- |
| 14 | "World Coordinates Tag" (absolute vs. terrain-relative z) | probe `abs_probe.py` over all 30,000 `[object]` placements | data | The tags exist and OPTS applies them (`.sco` `[absheight]` or `[splinehelper]` → absolute, `tile.rs:477-489`). Placements without either tag: 27,206 terrain-relative, **0** absolute. With `[splinehelper]`: 271 absolute, 0 relative. With `[absheight]`: 87 absolute, 1 relative (a crossing left at tile position 0/0/0, a content leftover). `[worldcoordinates]` in `global.cfg` is a different thing (Mercator tile grid). The Rathaus crossing is **not floating**: it sits at the right height and is buried because its `[crossing_heightdeformation]` bend is missing |
| 15 | Terrain carving and snapping | probe `road_vs_terrain.py`, 25,002 samples on road centre lines | data | The editor **already bakes roads into the terrain**: raw terrain − road is 0.000 m at p25, p50 and p75. Residue: 14.1 % of samples > 5 cm above the road, 1.7 % > 0.3 m, 1.4 % > 1 m. OPTS carves at runtime (`terrain.rs:190-227`), but with one flat footprint per profile (widest x, lowest z), which digs the 16 m pits under embankments; objects sample the uncarved grid |
| 16 | AI never changes lanes | code | partly | Lane changes **exist** (`opts-traffic/src/lanechange.rs`, MOBIL-like, 1.2 s announce, smoothstep glide that reaches the rendered pose). They are suppressed within 80 m of any junction or traffic light (`JUNCTION_LOOK_M`), need a slower leader (traffic is capped at 30 vehicles), and need a parallel neighbour lane at 2.2-5.5 m within 12° (disturbed by the straightened large-radius lanes). `TrafficEvent::LaneChanged` is consumed by nothing: no indicator |
| 17 | AI ignores spline speed limits | code + data | partly | `[rule] speedlimit` is parsed (`source.rs:100`, 1,503 rules here: 30, 25, 80, 40, 60 km/h) and caps the desired speed (`agent.rs:615-616`). What is missing is the **curve** limit (no lateral-acceleration cap) and look-ahead braking for a lower limit ahead; whether rules land on the right path index needs the Phase B check |
| 18 | AI cars have no lights | `c02_night_ai_road.png` (21:00) | yes | AI cars on Altstädter Ring are unlit. Lights exist only for the player vehicle (`opts-bevy` `vehicle.rs:194,228`, `vehicle_lights.rs`). Parked cars render conspicuously bright at night |
| 19 | No in-game date/time/weather control | code | yes | Date, time and weather are chosen only in the new-game wizard (`new_game_screen/weather.rs`, five presets); sun, sky and ambient light are set up once at session start (`in_game/world.rs`, `sky.rs`); the pause menu has no environment page; `sim.set_time` and `debug.wireframe` answer "not implemented" |

## 2. Subsystem mapping of all 19 symptoms (A2)

Symptoms 1-13 are mapped and root-caused in the predecessor (§3, §6); repeated here in the five systems of this brief.

| System | Symptoms | Data origin | Code | State |
| :--- | :--- | :--- | :--- | :--- |
| **1. Terrain, coordinates, tags** | 1, 2, 5, 7, 14, 15 | `.map.terrain`, `.sli` `[heightprofile]`, `.sco` `[absheight]`/`[splinehelper]`/`[crossing_heightdeformation]`, tile grid | `terrain.rs` (carve, footprint, sampling), `tile_ground.rs`, `ground.rs`, `tile.rs`, `job.rs` (pitch), lane geometry in `opts-traffic`/`opts-sim` | root causes known: flat carve footprint, objects on the uncarved grid, crossings not bent, single tile pitch + f32 origins, straight lanes on large radii |
| **2. Parser, stitching, Chrono** | 3, 4, 10, 11 | tile `.map`, `.x` meshes, `Chrono/` | `opts-core` tile reader (`scan_blocks`), `mesh.rs` `.x` palettes, no Chrono code | root causes known: "missing" bridge/street piece = buried crossing ramp (no record missing, indentation not involved); `.x` material slots; blank values dropped; Chrono absent |
| **3. AI kinematics and lights** | 12, 16, 17, 18 | lane graph, `[rule]`, AI `.bus`/model configs | `opts-traffic` (`agent.rs`, `idm.rs`, `lanechange.rs`, `neighbours.rs`), `opts-game` `traffic.rs`, `opts-bevy` `vehicle_lights.rs` | known: front-bumper pose, no curve speed, no lights; to measure: why lane changes do not fire |
| **4. Asset ingestion and gaps** | 6, 8, 9, 13 | tree TGAs, parked-car configs, `[light_enh_2]`/`[maplight]`, attachments, `ailists`/`unsched_*` | `trees.rs`/`mesh.rs`, `model.rs` alpha, `vehicle.rs` CTC, `scenery_runtime/spawn.rs`, `attachments.rs`, `traffic.rs` density | root causes known: tree normals (not DDS alpha), whole-body blending, CTC only for the player, glow on the LOD band and `[maplight]` unused, repeater fields, density model |
| **5. Environment UI** | 19 | session setup | `new_game_screen`, `pause`, `in_game/{world,sky}.rs`, control `sim.*` | missing feature; design needed |

## 3. Corrections to the brief's premises

1. **No missing "World Coordinates Tag".** The z rule is implemented and matches the data (§1, #14). What the crossing lacks is the `[crossing_heightdeformation]` bend.
2. **Runtime carving is a residue fix, not the main mechanism.** Roads are already baked into the terrain files; a deep runtime carve is what produces the pits. The fix is a profile-shaped carve that only removes residue above the road surface and fits the terrain edge to the profile edge, with objects using the same surface. The guardrail "do not flatten the terrain" holds: nothing outside the road profile is touched.
3. **Indentation is not involved** (0 indented keywords in the 329 tiles; predecessor §1).
4. **Trees are not a DDS alpha problem** (TGAs with clean alpha; the fault is billboard normals, confirmed live).
5. **Lane changes and speed limits exist**; the work is to make them effective (junction rule, density, neighbour detection, curve limit, look-ahead) and visible (indicators).

## 4. Questions for Checkpoint 1 (answered: all accepted)

1. Accept the premise corrections of §3 as the basis for Phase B, in particular for #14 (no new tag; the deliverable becomes a precise spec of the existing tags in `sco.md`/`tiles.md`) and #15 (residue carve shaped by the profile, documented in `architecture/terrain-carving.md`).
2. Environment UI (#19): a page in the pause menu (date, time, weather, apply live) is my proposal; alternatively a HUD panel. Should changing the date also switch the Chrono state live (reload of the affected tiles), or only on the next session start?
3. Phase B for the five systems: the predecessor already holds root causes for 1-13; Phase B here would add the measurements still open (lane-change suppression, speed-rule path mapping, AI light sources in the vehicle configs, environment wiring) and the two design documents.

**Checkpoint 1 decisions:** premise corrections accepted; environment page in the pause menu; a date change switches the Chrono state live (affected tiles only); Phase B covers the open measurements and the design documents.

## 5. Phase B: findings

Three read-only investigations with probes (kept locally in `systems/probes-t1..t3`); symptoms 1-13 keep the root causes of the [predecessor](berlin-spandau-2026-10-17.md#6-phase-b-root-causes).

### System 1: terrain, coordinates, tags → [terrain carving](../architecture/terrain-carving.md), [streaming coordinates](../architecture/streaming-coordinates.md)

* Height tags: rule complete and correct (§1 #14; specs updated in [`tiles.md`](../formats/map/tiles.md#mirror-flag-and-absheight-objects-2026-10), [`sco.md`](../formats/text/sco.md), [`global-cfg.md`](../formats/map/global-cfg.md)).
* Carving, measured on three maps: today's flat-footprint carve changes 30,000-46,000 vertices per map, 3,800-7,300 by more than 5 m (max 40 m). A **triangle cap** limited to the flat drawn surface, guarded at 1 m, changes 10,800-34,600 vertices, **none by more than 1 m**, and leaves 151 / 1,259 / 1,610 poking triangles (raw data: 6,007 / 4,316 / 11,783; Hügelsdorf's remainder is buried content the guard keeps: tunnels, river bed). No terrain snapping (the gap below road edges is authored).
* One height function on the carved grid with the mesh's triangle split for objects, trees, lanes, pedestrians, rail and the vehicle (today 7-12 % of objects are more than 5 cm off the drawn ground).
* Crossings: bend by `[crossing_heightdeformation]`, then cap terrain under their ground faces (1,961 vertices on Spandau, p95 0.22 m).

### System 2: parser, stitching, Chrono

No change to the predecessor's findings (buried crossing ramp, `.x` material slots, blank values, Chrono semantics). Chrono becomes live-switchable (System 5).

### System 3: AI kinematics and lights → [AI traffic kinematics](../architecture/ai-traffic-kinematics.md)

* **Lane changes** are suppressed, not missing. Gates at 150 vehicles (44,921 evaluations): junction rule 58 % (any zone anywhere on the lane, even passed), `trucks` rule 2,776 of 4,552 candidates (read as "trucks only", which bars cars from every right lane: 857 lanes), density-0 lanes 567. Headless, 600 s: **16 → 85** changes (30 vehicles) and **65 → 335** (150) with `trucks` as "trucks permitted", a distance-based junction gate and arced lanes.
* **Speed rules** map correctly (0 rules out of range or without owner). No Spandau road is posted above 30 km/h (60/80 are rail); AI drive 50 km/h ± 10 %, but the spawn lane's limit caps a driver for life, and **22 % of samples on arcs exceed 2.5 m/s² lateral (max 46 m/s²)**.
* **Pose** from the front bumper (rear swings out, heading jumps at joins).
* **AI lights:** all 11 AI vehicles light through `[light_enh_2]` (`AI_Light`, `AI_Brakelight`, `lights_blinker_l/r`); OPTS sets these variables, but the vehicle model parser drops `[light_enh_2]`, so nothing is drawn. `.sco` `[path]` parameter 12 is the turn indicator of a crossing path (0 none, 2 left, 3 right; 70-86 % agreement with turn direction).

### System 4: asset ingestion

Unchanged from the predecessor: tree billboard normals, transmap blending and CTC for parked/AI cars, glow parented to the LOD band, `[maplight]` unused, repeater fields, parking-list selector, density model.

### System 5: environment controls

* Date, time and weather are frozen at spawn in three places (`world.rs:10-84`, `sky.rs:217-229`, session info) and re-derived from start values in six more (sky update, scenery clock, traffic density, AI lights, instruments, HUD clock); traffic uses an unwrapped hour and always "weekday"; weather affects only sky attenuation, fallback light and temperature (no fog exists); `sim.set_time` has a CLI spec but no handler.
* **Design:** one `SessionEnvironment` resource (date, seconds since midnight, weather, rate, revision, Chrono date) in `opts-bevy`, written only through an `EnvironmentChange` message by the pause page, `sim.set_time` and save restore; all nine consumers read it. Pause menu entry **Environment** (date, time, weather; steppers and chips shared with the wizard in a `shell::calendar` module). Live Chrono switch: if the active entry set changes, rebuild only the resident overlay tiles plus their carve neighbours (player tile first; AI on vanished lanes are removed; the player vehicle stays). Map-step date picker with the default `today + [realyearoffset]` clamped into `[years]` (Spandau 1994-12-31).

## 6. Phase C plan (for approval)

One commit (or short series) per item, tests with each; ordinary 300 m maps pinned by tests.

| # | Item | Design |
| ---: | :--- | :--- |
| C1 | Tile grid (f64, Mercator rows, re-anchoring) | [streaming coordinates](../architecture/streaming-coordinates.md) |
| C2 | Terrain triangle cap + one height function; crossing bend + carve under crossing faces | [terrain carving](../architecture/terrain-carving.md), [predecessor §7](berlin-spandau-2026-10-17.md#7-fix-plan-phase-c-for-approval) |
| C3 | Lane geometry (arcs, mirror, Hermite), pose (axle tractrix), curve speed + look-ahead, lane-change gates, `trucks` reading, strategic changes, `[light_enh_2]` for vehicles + headlight pool, indicators from `[path]` parameter 12 | [AI traffic kinematics](../architecture/ai-traffic-kinematics.md) |
| C4 | Tile reader keeps blank values; attachments (spacing/range/p2, chain index); parking-list selector; Chrono (parser, overlays, data files); density model | predecessor §6-7 |
| C5 | Rendering gaps: tree normals, transmap split, CTC for parked/AI cars, scenery glow on the object root, `[maplight]` pool, `.x` shared palettes | predecessor §6-7 |
| C6 | `SessionEnvironment`, pause Environment page, live Chrono switch, map-step date picker, `sim.set_time` | §5 System 5 |

## 7. Questions for Checkpoint 2

1. **`trucks` rule:** switch the reading to "trucks permitted" (the data supports it; it reverses an M-0049 project decision)?
2. **Environment page:** time and weather preview live while the page is open, the date (and any Chrono rebuild) applies on *Apply*; Resume waits for the rebuild to finish. Weather stays sky/light/temperature (no fog or rain rendering in this task).
3. **Terrain guard** 1 m (recommended) rather than 2 m; deeper buried content is left for terrain holes (`[terrainhole]`, not in this task).
4. **Order:** C1 → C2 → C3 → C4 → C5 → C6, each reported briefly as it lands, Checkpoint 3 at the end?
