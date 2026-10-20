# Scenery Object Runtime (LOD, Visibility, Animation, Scripts, Lights)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Specified and implemented in part (see [Status](#status)) |
| **Milestones** | [M-0054](../roadmap/phase-5/M-0054.md) (Phase C prerequisite, Q-SB-11), [M-0049](../roadmap/phase-4/M-0049.md) (signal heads), [M-0026](../roadmap/phase-4/M-0026.md) (script VM), [M-0045](../roadmap/phase-4/M-0045.md) (script-driven model effects) |
| **Code** | `opts_core::formats::scenery` (parsing), `opts_sim::script::load_scenery_scripts` (loading), `opts_bevy::scenery_runtime` (ECS runtime), `opts_game::in_game::omsi_world::scenery_objects` (map wiring) |
| **Related** | [Script VM](script-vm.md), [Scenery benchmark](scenery-benchmark.md), [`.sco`](../formats/text/sco.md), [Scenery scripts](../formats/text/scenery-scripts.md), [Predefined variables](../formats/text/vehicles/predefined-variables.md), [Map tiles](../formats/map/tiles.md) |
| **Last Updated** | 2026-10-11 |

## Purpose

A placed scenery object must look the way its own files say: only the right level of detail, meshes shown and hidden by their `[visible]` rules, meshes moved by `[newanim]`, lights glowing when their variable says so, and, for the 2,828 objects with a `[script]`, the script running so that traffic lights change, clocks turn and season switches flip. Before this work none of that happened for scenery: all `[LOD]` groups were drawn at once, `[visible]`, `[newanim]` and light effects were ignored and no scenery script ran ([scenery-benchmark.md](scenery-benchmark.md#scenery-scripts-question-c)).

This page specifies that runtime and the small API the scenery benchmark ([M-0054](../roadmap/phase-5/M-0054.md) Phase C) uses to drive it deterministically. Audio from scenery scripts is out of scope (events are drained and dropped, as for vehicles).

## Entity layout

An object that needs any runtime behaviour gets a small hierarchy; an object that needs none keeps the flat layout it had before (root with one child per mesh part), so static scenery costs nothing extra.

```text
object root            Transform, SceneryObjectRuntime, [SceneryScripts], [SceneryLod]
├── LOD group entity   Visibility switched by LOD                  (only when the .sco has [LOD])
│   └── mesh entity    Transform + Visibility from SceneryMeshRule (only for meshes with
│       │                                                          [visible] or [newanim])
│       └── part       Mesh3d + MeshMaterial3d [+ SceneryLightMap]
└── light entity       SceneryLight (corona sprite)                (one per [light_enh]/[light_enh_2])
```

Meshes without rules hang their parts directly under their LOD group (or the root). LOD and `[visible]` act on different entities, so Bevy's visibility inheritance combines them: a part shows only when its LOD group and its mesh entity are both visible. Lights hang under the LOD group of the mesh that precedes them.

`SceneryObjectRuntime` on the root lists the mesh-rule, light and light-map entities, so the per-object update touches only that object's own entities (no global scan over all animated meshes).

## Draw distance (M-0029)

`SceneryDrawDistance` (resource, set from `graphics.view_distance` by `sync_view_distance`: 150 m per step, step 8 or `None` = unlimited) is applied by `scenery_draw_distance_system` every 0.25 s and at once when it changes: an object root (every placed object carries `SceneryDrawRoot`, set in `finish_object`) whose bounding sphere (`SceneryLod` radius, else 50 m) lies wholly beyond the limit gets `Visibility::Hidden` and the `DistanceCulled` marker; it is shown again 20 m inside the limit. Only roots carrying the marker are ever restored, so an object hidden by another rule stays hidden. Terrain tiles are not affected (tile radius), nor are splines or the sky.

## Level of detail (`[LOD]`)

**Parsing** (unchanged): `[LOD] <t>` starts a group; meshes after it carry `lod_threshold = Some(t)` until the next `[LOD]`. Meshes before the first `[LOD]` have `None`.

**Selection rule (assumption, Q-SR-01).** The value is a *screen-size* threshold, not a distance:

- The object's size on screen is `s = r / (d · tan(fov_y / 2))`: its bounding-sphere radius `r` (all meshes of the object, from their vertex bounds at spawn) over the half height of the view frustum at the sphere centre's distance `d`. `s = 1` means the object's diameter fills the screen height.
- A group with threshold `t` is drawn while `s ≥ t` **and** `s < t_up`, where `t_up` is the smallest threshold of the same object that is greater than `t` (∞ if none). The groups are therefore **exclusive bands**, and a `[LOD] 0` group is the stand-in drawn whenever no finer group applies.
- Meshes before the first `[LOD]` are always drawn.

Evidence: 1,022 objects use `[LOD]`; the common shapes are `0.15 0`, `0.1 0`, `0.02 0`, `0.3 0` (high detail first, `0` last), consistent with the `.sco` page's reading. Values from `0.01` to `0.9` rule out metres. A single `[LOD] 0.3` (tram points overlays) then means "draw only when large". 31 files list a coarse group *after* a finer one with a larger value (`0.1` then `1`, `0.9`; railway signals with a `_LOD` mesh); with sorted bands their `_LOD` mesh is drawn only above `s = 1`, i.e. almost never ❓.

**Runtime.** `scenery_lod_system` runs every frame for objects with `SceneryLod` (only those). It uses the first active 3D camera; with no camera all groups keep their state. `SceneryLodMode` (resource) switches the rule: `Camera` (default), `Finest` (the band with the largest threshold, plus the always-drawn meshes) and `All` (old behaviour). The benchmark uses `Finest` to get a deterministic image regardless of framing.

## Visibility (`[visible]`) and animation (`[newanim]`)

**`[visible] <variable> <value>`**: the mesh is drawn while the variable **equals** the value (|v − value| < 10⁻³), the rule the vehicle runtime already uses (`opts_bevy::vehicle_anim`). This settles Q-SB-04 by corpus evidence: of 3,770 `[visible]` lines, 101 use `0`, 31 use `2`, 10 use `-1`, 10 use `-2`, 4 use `3`, 4 use `4` and one `30`; "non-zero" could not express those. The parser keeps one `[visible]` per mesh (a later line replaces an earlier one).

**`[newanim]`** is now parsed with the vehicle parser into the typed `ModelAnim` (`opts_core::formats::vehicles::model::ModelAnim`); `ScoMesh::animations` holds `Vec<ModelAnim>` and the untyped `ScoAnimation` is gone. Malformed blocks are skipped with a warning in `SceneryObject::warnings` instead of failing the whole object. The motion rules are those of `opts_bevy::vehicle_anim` (frame from `origin_trans` and `origin_rot_*`, rotation about and translation along the frame's X axis, blocks applied in order, `maxspeed` limiting the follow speed); scenery reuses `AnimStep`, `MeshAnimation::follow` and `animation_matrix_from_values`.

**`origin_from_mesh`** (the second most common sub-key in scenery): the animation origin is the translation of the mesh file's own `0x79` matrix (the exporting tool's pivot), converted to OMSI axes; the orientation still comes from `origin_rot_*` ❓ (Q-SR-03). Meshes without a matrix use the origin.

**Variables without a script.** 5,353 objects have `[visible]`, `[newanim]` or lights but no script. Their variables come from the host only (the predefined values below); everything else reads `0`, as in a script that never wrote it ❓ (Q-SB-05).

## Script runtime

### Loading

`opts_sim::script::load_scenery_scripts(&SceneryObject, resolve)` reads the `.sco`'s `[script]`, `[varnamelist]`, `[stringvarnamelist]` and `[constfile]` lists exactly like `load_vehicle_scripts` (both now share one loader). Paths are resolved relative to the `.sco` folder, case-insensitively, including `..` paths into other packs (Q-SC03). Missing files become warnings, compile problems diagnostics; the object still spawns.

**One engine per placed object, one compile per `.sco`.** The VM keeps variables inside the engine and interns names into the program at run time ([Script VM](script-vm.md)), so instances cannot share one engine. The map loader compiles each `.sco`'s script set once (cache keyed by the resolved `.sco` path) and gives every instance a copy (`ScriptEngine::fork`: cloned program, constants and declarations, fresh variables). A traffic-light head copy is a few kilobytes.

The instance's string variables (`MapObjectInstance::string_vars`) are written, in order, into the string variables named by the `.sco`'s `[stringvarnamelist]` before `{init}` runs. The same name list now resolves `[texttexture_enh]` sources given by name (static text; script-changed text is not re-rendered yet, Q-SR-06).

### Host values

Every tick the host writes, before `{frame}`:

| Name | Accessor | Value |
| :--- | :--- | :--- |
| `Timegap`, `GetTime` | `L.S` | set by `ScriptEngine::run_frame` (step and accumulated seconds) |
| `Time` | `L.S` | seconds since midnight of the session clock (clock scripts compute `Time / 120` degrees for the hour hand) |
| `Day`, `Month`, `Year`, `DayOfYear` | `L.S` | the session date (`Month` 1–12, `Day` 1–31, `DayOfYear` 1–366) |
| `SunAlt` | `L.S` | sun elevation in degrees (the sky's formula) |
| `Weather_Temperature` | `L.S` | °C; 15 unless the host sets it ❓ |
| `TrafficLightPhase` | `L.L` | state number of the object's light program (below) |
| `NightlightA` | `L.L` | `1` while night lights are on (sun below 3°, the rule the traffic lights of AI vehicles use), else `0` ❓ (Q-SR-04) |
| `Switch`, `Signal`, `NextSignal`, `TrafficLightApproach`, `Colorscheme` | `L.L` | `0` (no rail signalling, points or repaint choice yet) |

Values in `SceneryVariableOverrides` (resource) are written last and win; the benchmark uses them to force `TrafficLightPhase` or any script input.

### Traffic-light phase (link to M-0049)

A signal head is linked to its junction by the tile: an `[object]` record followed by `[varparent] <junction id>` carries exactly one string variable, the **program index** in the junction's `.sco` (declaration order across groups, the numbering of `[use_traffic_light]`). Measured on Grundorf, Gladbeck and Bad Hügelsdorf: 4,231 of 4,235 such heads are `[trafficlight]` objects whose index is below the junction's program count ([tiles.md](../formats/map/tiles.md)). `MapTile::var_parents` now records these links.

The head's `TrafficLightPhase` is `opts_traffic::SignalProgram::state_at(clock)` of that program, with the clock the AI traffic uses (`start time of day + elapsed session seconds`, `move_traffic`), so heads and AI drivers always agree. A junction found on a tile loaded earlier is linked as well (job-wide index of loaded objects); a head whose junction is not loaded yet keeps phase `0` ❓ and is counted in the log. An object without `[varparent]` that has its own programs (level crossings) uses its own program 0 ❓.

### Ticking and budget

`scenery_script_system` (in `Update`, after the clock sync) ticks objects by distance from the camera:

| Band | Default | Tick |
| :--- | :--- | :--- |
| near | ≤ 250 m | every frame |
| mid | ≤ 1,000 m | every 0.25 s with the accumulated step |
| far | > 1,000 m | every 2 s with the accumulated step |

Accumulated steps are capped at 2 s. A frame ticks at most `max_ticks_per_frame` (default 1,000, about 2 ms at the measured cost) objects; the rest wait for the next frame (round-robin), so a map with thousands of heads cannot stall a frame. After a tick, the object's mesh rules, lights and light maps are updated from its variables (only for ticked objects). Objects without a script are evaluated once at spawn and again when a host value changes (night switch, every 2 s at most).

**Targets** (release build): ≤ 2 µs per scripted traffic-light head per tick including the effect update, ≤ 1 ms per frame for a busy map view. Measured numbers are in [Status](#status).

## Lights (`[light_enh]`, `[light_enh_2]`, `[matl_lightmap]`)

**Parsing.** `ScoLight` keeps position, direction, colour, size, the brightness variable and the factor; `[light_enh_2]` adds `omni`, the cone angles, `timeconst` and the optional bitmap. Layouts per [`.sco`](../formats/text/sco.md#light_enh_2): `[light_enh]` is x, y, z, r, g, b, size, *variable*, …; `[light_enh_2]` has 23 or 24 values with the variable at position 18 (1-based) and the factor at 19. A light whose numbers do not parse is skipped with a warning.

**Rendering.** A light hangs under the object's root entity, not under the `[LOD]` group it is declared in: lanterns write their `[light_enh_2]` after `[LOD] 0`, and parenting it to that band made the glow vanish within ~70 m (Berlin-Spandau report, System 4). A light is a soft glow: a shared disc whose vertex colours fall from full at the centre to transparent at the rim (no visible edge), one unlit additive material per colour, turned towards the camera every frame, of diameter `size × √level` (until 2026-10-17 a solid emissive sphere, which read as a hard white disc up close), with `level = clamp(variable × factor, 0, 1)`, hidden at 0 and beyond 1,500 m. A directional light (`omni = 0`) fades with the angle between its direction and the line to the camera: full inside the inner cone, nothing outside the outer cone. The direction is the shining direction (vehicle models: 8,788 front lights point to +y and 7,480 rear lights to −y); the cone values are read as full angles, as for Direct3D spotlights ❓ (Q-SR-08). No Bevy light source is created (thousands of heads would cost too much); objects are lit by the light only through their own light maps. `timeconst` smoothing is not applied ❓.

**Light cones.** A directional `[light_enh_2]` whose `cone` flag (parameter 22) is `1` also gets a visible cone entity (`opts_bevy::light_cones::LightCone`, a child of the object root next to its glow sprite) drawn as a ray-marched additive volume. The cone reads the brightness of the glow (`SceneryLight::level`: the same variable, factor and night rule), so it shows exactly while the lamp shines; nothing is evaluated twice. Only the nearest cones by screen coverage up to `LightQuality::cone_cap` are visible, the others are `Hidden`. The corona angles (up to 200°) are not the beam: the cone half angle is the outer angle halved, kept in 10°..40°, and it reaches from the lamp to the ground when it points down (2..14 m), else 6 m. Details and measurements: [lighting cones, implementation](lighting-cones.md#implementation).

**Light maps.** `[matl_lightmap] <texture> [<variable>]`: the texture becomes the part's emissive map, scaled by the variable. Variable-less maps (1,235 of 1,358 uses, building windows and street lamps) are switched by `NightlightA`; `[nightmapmode]` schedules (office, school, …) are not applied yet ❓ (Q-SR-05). Global maps share one material per texture; script-driven maps get a private material copy per object.

**Night maps.** `[matl_nightmap] <texture>` without a light map is treated as a variable-less light map: emissive, switched by `NightlightA` (OMSI swaps the day texture for it ❓).

## Map lights (`[maplight]`)

Each `[maplight] x y z r g b r_full` becomes a `MapLight` marker entity under its object (also for objects that are `[onlyeditor]` before their first mesh, which draw no mesh). No light source is created per lamp. `MapLightPool` (resource) owns `MapLightSettings::pool_size` (48) shadowless `PointLight`s; `map_light_pool_system` (in `SceneryRuntimeSet`) hands them every 0.25 s to the nearest lamps within 200 m of the active 3D camera (`choose_nearest`: a lamp that holds a light counts 15 m nearer, so the choice does not flicker at the edge), only while `SceneryClock::night` (`NightlightA`) is on. A slot keeps its lamp while it stays chosen; a released slot fades out (0.5 s) before it takes the next lamp, which fades in. Photometry: the lamp's illuminance at `r_full` is `E_full`, so the point light gets `4π·r_full²·E_full` lumens and a range of `8·r_full`. `E_full` (`FULL_LUX`, tunable) is 10,000 lx: OMSI's "full intensity" is the scene's full lighting level, here half the 20,000 lx midday sun, so a lamp 7 m up lights the road with about 1,300 lx; the physical 15 lx of a street lamp is invisible under the game's fixed exposure (measured live, 2026-10-17). `[nomaplighting]` is parsed and recorded as the `NoMapLighting` marker on the object root (`opts_bevy::light_rooms`, version 1: no behaviour, point lights still light everything in range). While the camera is in an interior view of the player vehicle, `map_light_pool_system` skips lamps over the vehicle's footprint and dims the pool by `MapLightSettings::interior_view_scale` (0.5) ([light rooms](lighting-system.md#45-cpu-rules-that-replace-rooms)). It marks the objects that carry their own `[maplight]` (654 of 688 files; 1,587 of 1,607 placed map-light objects in Berlin-Spandau), i.e. a lamp not lit by its own light; the planned handling is in [lighting system §4.6](lighting-system.md#46-nomaplighting-handling).

Not done: `[matl_change]` (texture switching), `[alphascale]` for scenery, the effect of `[nomaplighting]`, `[particle_emitter]`, `[smoke]`, `[mouseevent]` triggers for scenery.

## Interface for the benchmark (stage 3)

All items are in `opts_bevy::scenery_runtime` (re-exported at the crate root). The harness adds `SceneryRuntimePlugin` (the game already does), then:

| Item | Use |
| :--- | :--- |
| `SceneryClock` (resource) | `seconds_of_day`, `year`, `month`, `day`, `sun_alt_deg`, `night`, `temperature_c`; `SceneryClock::at(year, month, day, seconds_of_day)` builds one. With `follow_session = false` the game's sync system leaves it alone. |
| `SceneryRuntimeSettings` (resource) | `automatic` (tick in `Update`; set `false` to step by hand), distance bands, `max_ticks_per_frame`. |
| `SceneryVariableOverrides` (resource) | Name → value written into every scripted object after the host values (e.g. `TrafficLightPhase`). |
| `SceneryLodMode` (resource) | `Camera`, `Finest` or `All`. |
| `advance_scenery(world: &mut World, dt: f32)` | One deterministic step for **every** object regardless of distance: runs `{init}` where pending, moves `SceneryClock.seconds_of_day` by `dt`, runs `{frame}`, applies `[visible]`/`[newanim]`/lights. Call it N times with `dt = 1/60` for the scenarios S0–S5. |
| `SceneryScripts` (component, on the object root) | `with_engine(|e| …)` to read or write variables (`get`, `set`, `variables()`), `diagnostics()` for load and run-time problems. |
| `SceneryObjectRuntime` (component) | The object's rule entities (`meshes`, `lights`) for the expected-silhouette computation. |
| `SceneryMeshRule` (component) | Per-mesh rules (`visible`, animation steps) and the values last shown. |

A scenario is then: insert `SceneryClock::at(…)` with `follow_session = false`, set `automatic = false`, call `advance_scenery` for the scenario's duration, render. The script-effect check reads the variables through `SceneryScripts` and the expected visibility through `SceneryMeshRule`.

## Developer aid

`OPTS_DEV_LOOK_AT=<.sco substring>` with `--camera free` puts the free camera 12 m in front of the first placed object whose `.sco` path contains the substring (the object's +y side), for screenshots of signal heads and similar objects. Combine with `OPTS_DEV_HOUR` and `OPTS_DEV_ENTRY`.

## Status

Implemented in branch `agent/scenery-runtime` (2026-10-12): everything above except what is listed as "not done". Verification:

- **Corpus** (`cargo run --release -p opts-bevy --example scenery_scripts -- "<OMSI>" 50`): 2,822 of 55,664 `.sco` have scripts (270 distinct first scripts); all compile (0 errors) and all run and change a variable. 138 referenced script, variable-list and string-list files do not exist in the install (broken pack paths such as `..\..\Verkehrszeichen_MC\script\ampel1.osc`), so 107 objects start without their script. 339 run-time diagnostics in 53 scripts: 324 are the missing bus-stop timetable macros (`getarrbusline`, `getarrbusterminus`, `getarrbustimediff`; bus-stop displays and `dfi*`), 15 are stack underflows (8 in `peopleHC.osc`, whose `{frame}` is malformed in the pack itself).
- **Cost** (release, `crates/opts-bevy/tests/scenery_runtime_budget.rs`): 3,000 scripted signal heads with three `[visible]` lamps and three lights cost 6.0 ms for a full step, 2.0 microseconds per head including the effect update; the bare host feed plus `{frame}` costs 0.22 microseconds per tick over the corpus (3.4 million ticks). Per-frame budget: `max_ticks_per_frame = 1,000` (about 2 ms worst case, round-robin); a busy view has a few hundred heads within 250 m (about 1 ms).
- **Game** (Grundorf, `OPTS_DEV_LOOK_AT=ampel --camera free`, throwaway config): the head at the first junction shows green at 12:00:04, 12:00:12 and 12:00:20, amber at 12:00:35 and green again at 12:01:30 (cycle about 90 s, the junction program); at 22:00 the head is dark except the corona of the lit lamp. Screenshots are kept outside the repository.

## Open questions

| ID | Question | Impact |
| :--- | :--- | :--- |
| Q-SR-01 | `[LOD]` measure and direction: screen-size fraction with exclusive bands is an assumption (Q-SCO-003). Which radius does OMSI use (bounding box, mesh sphere), and does `[detail_factor]` or the user's detail setting scale it? | 🟡 Objects may switch detail at other distances than in OMSI. |
| Q-SR-02 | Is the `[varparent]` string really the program index, and is a head's phase the junction program's state (not the junction script's)? 99.9 % of heads fit; not checked in OMSI. | 🟢 |
| Q-SR-03 | `origin_from_mesh`: is the pivot the full `0x79` matrix (rotation too) or its translation only? | 🟡 Wrong pivot rotates hands and barriers about the wrong axis. |
| Q-SR-04 | `NightlightA` and variable-less light maps: what exactly switches them (sun altitude threshold, `[nightmapmode]` schedules, weather)? | 🟡 |
| Q-SR-05 | `[nightmapmode]` 1–4 schedules (residential, office, school) are not applied. | 🟢 |
| Q-SR-06 | Script-driven text (`Refresh_Strings`, bus-stop displays): when does OMSI re-render a text texture? Needs a text-texture refresh path. | 🟡 74 + 42 displays stay static. |
| Q-SR-07 | `{frame_ai}`/`[mouseevent]` for scenery: when do scenery triggers fire (clicks only)? Not wired. | 🟢 |
| Q-SR-08 | `timeconst` light smoothing and `[light_enh_2]` `rotating`/`parameters` bits (star, no fog, effect only) are ignored. | 🟢 |

## Provenance

- Corpus counts: a text scan of the installed `Sceneryobjects/**/*.sco` and the tiles of Grundorf, Gladbeck and Bad Hügelsdorf (2026-10-11, keywords as whole lines). The scanner is not kept.
- Script reading: `ampel1.osc`, `clock.osc`, `visibleflag_sommerhalbjahr.osc` and `rolltreppe.osc` were read to learn which values they consume; no script text is reproduced here.
- **ADR-0002 attestation:** no OMSI executable or DLL was inspected. Only OMSI text files (`.sco`, `.osc`, `.map`) were read. No OMSI asset, script or screenshot is in the repository; the tests use self-written fixtures ([ADR-0002](../adr/0002-clean-room-policy.md)).
