# Berlin-Spandau: terrain, traffic and environment systems (2026-10-17)

| Metadata | Value |
| :--- | :--- |
| **Map** | `OMSI 2/maps/Berlin-Spandau` |
| **Brief** | expanded scope: 19 symptoms in five systems (terrain/coordinates/tags, parser and Chrono, AI kinematics and lights, asset gaps, environment UI) |
| **Branch** | `fix/terrain-and-traffic-systems` (worktree `opts-wt-berlin-spandau-systems`, from `master` `85d2391`), carries the two report commits of `fix/map-fidelity-systemic`; not merged, merge only after user acceptance |
| **Status** | Phase A done (Checkpoint 1: premise corrections accepted, pause-menu environment page, Phase B scope); Phase B done (Checkpoint 2: all four decisions accepted); Phase C done, waiting for Checkpoint 3 |
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

**Checkpoint 2 decisions ("Go"):** `trucks` = "trucks permitted"; time and weather preview live on the Environment page, the date applies on *Apply*, *Resume* waits for the rebuild, weather stays sky/light/temperature; terrain guard 1 m; order C1 → C6.

## 8. Phase C: implementation

Six agents in separate worktrees, merged into `fix/terrain-and-traffic-systems` (37 commits besides merges, 157 files, +13,475 / −1,985 lines). Each agent ran fmt, clippy `-D warnings` and the tests of its crates before every commit; the lead re-ran the whole workspace after each merge.

| Item | Commits | What it does | Design |
| :--- | :--- | :--- | :--- |
| **C1** tile grid | `3c40630` `1aead49` | `TileGrid` (f64, uniform or Mercator rows, chosen only when the seam fit agrees within 0.05 m); every tile origin, spawn, height lookup, water, neighbour offset, control coordinate goes through it; global tile size removed | [streaming coordinates](../architecture/streaming-coordinates.md) |
| **C2** terrain, crossings | `89ed6f0` `537e714` `4ebe1c2` `94e8fd2` `b405e6a` `5c7f251` | ground surface of a profile; triangle-cap carve (ε 3 cm, guard 1 m) replaces the flat footprint; carved grid returned and one `terrain_height` for everything; `[crossing_heightdeformation]` bends meshes, collision, paths and arms; terrain capped under crossing ground faces | [terrain carving](../architecture/terrain-carving.md) |
| **C3** AI | `37afa83` `72d8ffe` `c7ed22b` `2430633` `783b50d` `995480b` | arced and mirrored lanes with the `[spline_h]` ramp; axle pose; per-driver speed factor, curve limit, look-ahead; distance-gated and strategic lane changes, `trucks` = permitted; indicators from `[path]` parameter 12; `[light_enh_2]` for vehicles with a headlight pool | [AI traffic kinematics](../architecture/ai-traffic-kinematics.md) |
| **C4a** Chrono | `8cc7001` `7b117b0` `3da3c60` `30b5853` | entries, activation, overlay grammar, data-file overrides; tiles read through Chrono; live switch on `MapDateChanged` rebuilding only changed tiles plus the carve ring | [chrono.md](../formats/map/chrono.md) |
| **C4b** config | `1ca78df` `a0c564c` `7a42c62` | blank string values kept; attachment runs on a map-wide chain index; parking-list selector; per-group density from the rule flag with time factors and a proportional ceiling | [lists.md](../formats/map/lists.md), [ai-traffic.md](../architecture/ai-traffic.md) |
| **C5** rendering | `1b38c45` `37d2341` `ad78244` `59fee97` `0c71c54` `29930eb` `5ff3112` | tree crown normals; `[onlyeditor]` per mesh; shared `.x` palettes; opaque body + blended glass; `[CTC]` for parked/AI cars; glow under the object root; `[maplight]` pool; night maps | [scenery runtime](../architecture/scenery-runtime.md) |
| **C6** environment | `3f24497` … `9b47c3e` (6) | `SessionEnvironment` with one writer; pause Environment page; map-step date picker; `sim.set_time`; hook for the Chrono rebuild | [session environment](../architecture/session-environment.md) |

Fixes found while integrating: `401380a` (the glass image kept a mip chain longer than its data: wgpu panic at tile load, test added that fails on the old code), `a4221e7` (lamp glow was a solid sphere and the map lights were invisible at 15 lx: now a soft camera-facing disc and 10,000 lx), `aed3eb9` (wizard test leaked the host install through the shared folder names of the fixture maps).

### Numbers

| | Before | After |
| :--- | :--- | :--- |
| Tests (whole workspace, no `OMSI_PATH`) | not counted before | **1,707 passed, 0 failed** (with the install: 1,088 in the five content crates, 0 failed apart from the two known golden screens) |
| Terrain at the Güterschuppen (2402/11279) | 31-33 m → 15-17 m (1,468 vertices) | deepest lowering **0.79 m** (89 vertices) |
| Bent crossings below terrain (11 placements, 72,469 m²) | 19.4 % | **0.000 %**; 38 of 39 arms within 1 cm, one at 1.3 cm |
| Lateral acceleration in curves (p99, 30 / 150 cars) | 34.0 / 30.4 m/s² | **2.6 / 2.6** (max 7.7 / 10.6) |
| Dead-end lanes | 373 | 178 |
| Lane changes in 600 s (30 / 150 cars) | 20 / 93 | 31 / 123 |
| Parking bays / lamps placed | 762 / 716 | 3,774 / 1,806 (authored 3,858 / 1,923; the difference is repeaters repeating another on the same spline) |
| Chrono, 1986 → 1994 | not supported | 16 entries toggled, 108 tiles changed, 25 in the carve ring, 9 resident tiles rebuilt in place |

### Live checks (release build, Berlin-Spandau)

Confirmed in the game: the Rathaus junction and the Ruhlebener ramp no longer lie under grass; the Güterschuppen has no pit and the building stands on the ground; trees look the same from both sides; the "Brunsbütteler Damm" sign has its plate; parked cars are opaque, in varied colours and in an even 7 m row at the kerb; at night lamps glow softly and light the pavement and the bus shelter; AI cars follow the junction lanes without sliding; the Wall area differs between 1986 (border-strip paving) and 1994 (a road with traffic); `sim.set_time` with a date switches Chrono state live.

Not confirmed live: the median break on Falkenseer Chaussee (photo 2; the joins measure exact and the median is continuous in the shots taken, the exact vantage is unknown), lane arrows on crossings (no vantage taken), AI headlights at night with traffic in view, traffic density against the map's config at several hours, the player's tile through a Chrono switch while driving.

### Deviations from the designs

* **Re-anchoring (design §2.3) is not implemented.** The fixed per-row grid ships: exact near the start tile, about 2.8 cm of error per tile column away from it (pinned by a test).
* **Lane changes** rose less than the headless estimate (31 / 123 instead of 85 / 335) because changes on lanes that come from junction objects (`.sco` paths) are blocked; without that rule 64 / 236. Mirrored splines reverse their lane direction as well as their side (the side flip alone raised dead ends from 279 to 336).
* **Terrain guard** is applied per surface contribution, not on the final minimum; a straight `[spline_h]` ramp is now drawn and carved as its cubic.
* **Density:** `L0 = 80 m` (median over 56 entry points of 12 maps), default traffic ceiling raised from 30 to 100.
* **Glow** is an additive disc, not a swapped night texture; `[nomaplighting]` is parsed only.
* **Not done:** active Chrono entry shown on the wizard map step (optional); a Bevy-app test of the live-switch handler (verified live instead); the 136+ parked vans whose mesh cannot be decoded (`MB_T1_kasten_parked.o3d`, 247 skipped objects counting every resident tile).

### Known problems outside this task

* `golden_screens` (2 tests of `opts-game`) fail whenever `OMSI_PATH` is set, also on unmodified `master`.
* The wizard fixture maps share folder names with real maps; the wizard test is now hermetic (`aed3eb9`), the golden screens are not.

### Open questions

Q-TC-01..04 ([terrain carving](../architecture/terrain-carving.md)), Q-AK-01..04 ([AI kinematics](../architecture/ai-traffic-kinematics.md)), whether OMSI re-evaluates Chrono when the clock passes midnight, the meaning of attachment fields 9 and 12, the value of the "Medium" density class (assumed 1.0), which repaint OMSI picks for a parked car (deterministic hash here).

## 9. Round 3: lighting, cones and volumetrics (2026-10-17, branch `fix/map-and-lighting-systems`)

The third brief adds symptoms 20-23 (System 6) and repeats 1-19. **Symptoms 1-19 are implemented and merged in `master`** (sections 6-8); they are not redone. One new, separate task was added: weather (below). Shots are kept locally in `~/.local/share/opts/captures/berlin-spandau-2026-10-17/lighting/` (not committed).

### Premise corrections

1. **Bevy 0.20 does have volumetric lighting.** `bevy_pbr::volumetric_fog` (`VolumetricFog` on the camera, `VolumetricLight` on lights) renders light shafts and god rays by screen-space ray marching with a Henyey-Greenstein phase function. Its documented limit: **only directional lights with shadow maps interact**, and its cost scales with the number of volumetric lights. So the sun (a shadow-casting `DirectionalLight` already, `SunLight` in `sky.rs`) can get god rays without a custom shader. Lantern and headlight cones are **not** covered (spot and point lights are unsupported) and need custom work. *(Corrected in Phase B: the 0.20 shader also ray-marches point and spot lights that have `VolumetricLight` and shadow maps, `volumetric_fog.wesl:340-445`, `light.rs:1326`; the module docs still say directional only. A fog volume is also required. See [lighting system §6](../architecture/lighting-system.md#6-sun-god-rays-with-volumetricfog).)* Bevy 0.20 also ships a physical `atmosphere` module (not used by OPTS; the sky is the installation's textures).
2. **Light rooms map to `RenderLayers`.** *(Corrected in Phase B, see [lighting system §4.1](../architecture/lighting-system.md#41-what-bevy-020-does-with-renderlayers-on-lights-verified): in Bevy 0.20 a light's layers select which views it is in and which meshes cast its shadows, not which meshes it lights; the content needs no room separation, and `[nomaplighting]` marks lamps that carry their own `[maplight]`.)* Bevy applies `RenderLayers` to lights as well as meshes (`bevy_pbr/src/render/light.rs`), so a light only affects entities on shared layers. No custom shader uniform is needed for interior/exterior separation, only a layer convention and a tool for assigning objects.
3. **Cones are in the data.** `[light_enh_2]` carries inner and outer cone angles (`ScoLight::cone_inner_deg/outer_deg`, parsed), vehicle `[spotlight]` has inner/outer cones and `Spot_Select`, `[interiorlight]` has range and colour. What is missing is rendering them.

### Reproduction (A1)

Release build of `master` (`6cdbd05`), Berlin-Spandau at 22:00 (`OPTS_DEV_HOUR`), `sim.set_time` for dusk:

| # | Symptom | Shot | Observed |
| ---: | :--- | :--- | :--- |
| 20 | Lantern cones | `l1`, `l2`, `l3` | Pools of light on the pavement (map-light pool of 48 point lights) and no visible beam: the lamp heads are dark shades with no light source or cone, the air is clear. Distant lamps are soft glow discs |
| 21 | Light rooms | code, `l1` | None: point lights and the glow ignore walls and interiors (`[nomaplighting]` is parsed but not applied; every map light lights every object in range, also behind walls and inside buildings). The pool does not distinguish inside and outside |
| 22 | Sun and moon | `s1_sun_low_*` | At dusk the sky glow and ambient change, no light shafts through trees or buildings. The moon is a textured disc at the anti-sun position: no phase, no own direction (moonlight is the sun's `DirectionalLight` with night values, `world.rs`) |
| 23 | Vehicle light cones | `v1` | AI and parked cars show glows and (nearest six) real shadowless `SpotLight`s of fixed 12°/30°; no visible cone, no per-light cone angles from `[light_enh_2]`/`[spotlight]`, no interior lights. The player vehicle uses the model's own spotlights with shadow maps (`vehicle.rs`) |
| 18 | AI headlights | done | glows and the headlight pool exist (C3); cone is the open part (23) |

### Subsystem map (A2) for 20-23

| # | Data origin | Code today | Gap |
| ---: | :--- | :--- | :--- |
| 20 | `.sco` `[light_enh_2]` (position, direction, cone angles, size, variable), `[maplight]` | `scenery_runtime/lights.rs` (glow disc), `maplights.rs` (48 point lights) | no beam geometry or scattering for spot-like lights; map lights are omni point lights, cone ignored |
| 21 | vehicle `[interiorlight]`, scenery `[nomaplighting]`, building lightmaps | `vehicle_lights.rs`, `maplights.rs`; no `RenderLayers` anywhere | no layer convention, no assignment of buildings/vehicle interiors, `[nomaplighting]` unapplied |
| 22 | `envir.cfg` sky/light colours and twilight angles, session date and time | `sky.rs` (`SunLight`, `SunDisc`, `MoonDisc`), `environment.rs` (sun position) | no `VolumetricFog`/`VolumetricLight`, no moon direction/phase from date, moonlight reuses the sun light |
| 23 | model `[light_enh_2]`, `[spotlight]` with `Spot_Select`, AI variables | `vehicle_glow.rs` (pool of six spots, fixed cone), `vehicle.rs` | cone angles and brightness variables not mapped to the spot light, no beam |

### Asked for in this round besides the brief: weather

The docs specify weather ([`environment.md`](../formats/text/environment.md): `Weather/*.owt`, `clouds.cfg`, `.dsc`; scenario `.osn.owt`) but the game has five hard-coded presets that change only sky attenuation, fallback light and temperature; fog, clouds, precipitation, ground wetness and wind are not rendered. An agent (`agent/weather`, own worktree) implements it: `.owt` parser, the installation's presets in the wizard and the Environment page, distance fog, cloud layer, precipitation, wet ground, wind and precipitation script variables, settings toggles; design note `architecture/weather.md`. It is bounded away from System 6: plain distance fog only, no volumetric fog, god rays or light cones.

### Open for Checkpoint 2 (Phase B scope)

1. How OMSI separates interior from exterior light: measure from content (`[interiorlight]` range semantics, `[nomaplighting]` objects, buildings with `[LightMapMapping]`, bus interior lights) and decide the `RenderLayers` convention.
2. Cone rendering technique for lanterns and headlights without volumetric support in Bevy: candidates are a screen-space additive cone mesh with depth fade (cheap, no shadows), per-light analytic ray-marched cone in a custom material, or Bevy's volumetric fog with a spot-light shadow proxy; each costed against the 60 fps budget.
3. Sun god rays through `VolumetricFog`/`VolumetricLight`: cost on the target GPU, step count, fallback (off) in the Environment UI and settings.
4. Moon: direction and phase from date and time (new moon to full moon from the date), moonlight as its own directional light.
5. Performance budget: maximum volumetric lights on screen and the fallback ladder.
