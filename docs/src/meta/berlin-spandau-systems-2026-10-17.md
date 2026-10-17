# Berlin-Spandau: terrain, traffic and environment systems (2026-10-17)

| Metadata | Value |
| :--- | :--- |
| **Map** | `OMSI 2/maps/Berlin-Spandau` |
| **Brief** | expanded scope: 19 symptoms in five systems (terrain/coordinates/tags, parser and Chrono, AI kinematics and lights, asset gaps, environment UI) |
| **Branch** | `fix/terrain-and-traffic-systems` (worktree `opts-wt-berlin-spandau-systems`, from `master` `85d2391`), carries the two report commits of `fix/map-fidelity-systemic`; not merged, merge only after user acceptance |
| **Status** | Phase A (triage) done, waiting for Checkpoint 1 |
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

## 4. Questions for Checkpoint 1

1. Accept the premise corrections of §3 as the basis for Phase B, in particular for #14 (no new tag; the deliverable becomes a precise spec of the existing tags in `sco.md`/`tiles.md`) and #15 (residue carve shaped by the profile, documented in `architecture/terrain-carving.md`).
2. Environment UI (#19): a page in the pause menu (date, time, weather, apply live) is my proposal; alternatively a HUD panel. Should changing the date also switch the Chrono state live (reload of the affected tiles), or only on the next session start?
3. Phase B for the five systems: the predecessor already holds root causes for 1-13; Phase B here would add the measurements still open (lane-change suppression, speed-rule path mapping, AI light sources in the vehicle configs, environment wiring) and the two design documents.
