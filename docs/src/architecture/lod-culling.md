# 🔭 Level of Detail and Culling

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Design (2026-11-05): §1 describes the code on `master`; §3 is the plan of [M-0029](../roadmap/phase-5/M-0029.md), with the order of work in §4. Step 2 (per-part shadow culling) is built and measured: no gain, so it is off and hidden (§3.1). Facts name the code or the source that shows them; guesses are ❓ and collected in §5 |
| **Last Updated** | 2026-11-05 |
| **Relevant Crates** | `opts-bevy` (`scenery_runtime/{lod,draw_distance}.rs`, `terrain.rs`, `render_stats.rs`, `instancing.rs`), `opts-game` (`settings_registry/live.rs`, `in_game/omsi_world/`) |
| **Traces to** | [M-0029](../roadmap/phase-5/M-0029.md); [M-0082](../roadmap/phase-5/M-0082.md) ([instancing research](instancing-research.md) §9) |

This page collects what decides **what is drawn, at which detail and with which shadows**: the scenery's own `[LOD]` bands, the draw distance, the sun's shadows, terrain detail, Bevy's frustum and occlusion culling, and instancing. It is the design document that the M-0029 roadmap item "Document LOD, frustum culling and occlusion strategies" asks for.

The rule for every step below: **measure, then switch**. Per-mesh culling has a CPU cost of its own (a pass over thousands of entities, component changes that re-extract meshes), so a step that cannot show a gain on a real map stays off by default.

---

## 1. What exists (on `master`)

| Mechanism | What it does | Where |
| :--- | :--- | :--- |
| **`[LOD]` bands** | An object with `[LOD]` shows one exclusive band picked by its screen-size fraction from the main camera, every frame; `graphics.detail_distance` (percent, default 200) divides the distance (`SceneryLodBias`) | `scenery_lod_system`, [scenery runtime](scenery-runtime.md) §LOD |
| **Draw distance** | Object roots whose bounding sphere lies beyond `graphics.view_distance` × 150 m are hidden (top step: no limit), every 0.25 s, 20 m hysteresis. The radius is measured only for objects with `[LOD]`; every other object counts as 50 m (`DEFAULT_EXTENT_M`) | `scenery_draw_distance_system` |
| **Tile radius** | Which tiles are resident: `gameplay.tile_radius` (automatic by default) or the whole map (`gameplay.load_all_tiles`) | [map streaming](map-streaming.md) |
| **Sun shadows** | One `DirectionalLight` with four cascades up to `graphics.shadow_distance` (default 300 m) | `live::shadow_cascades` |
| **Shadow casters** | Every `Mesh3d` casts, except effects: light cones, glows, map-light flares, clouds, rain, puddles, sky (`NotShadowCaster` there only) | `grep NotShadowCaster crates/` |
| **Light distance** | Lamps within `graphics.light_distance` (default 350 m) get one of the pool's point lights; these lights cast no shadows | `MapLightSettings::radius_m` |
| **Terrain** | One mesh per tile at full resolution: 61 × 61 vertices, 7,200 triangles, on the carved grid | [terrain rendering](terrain-rendering.md) §2 |
| **Batching** | Bevy 0.20 already batches and instances: Berlin-Spandau's 9,242 mesh entities draw in 11 opaque/alpha-mask multi-draw calls; blended parts are sorted per entity | [instancing research](instancing-research.md) §1 |
| **Instancing prototype** | `OPTS_INSTANCING=off\|dedupe\|merge` (off by default): (a) de-duplication of equal assets, (c) static merging | `opts-bevy/src/instancing.rs` |

**What Bevy 0.20 does on its own** (read in the `bevy_*-0.20` sources):

- **Frustum culling.** Every view, including each shadow cascade, keeps only the entities whose `Aabb` meets its frustum (`check_visibility_cpu_culling` in `bevy_camera`; for the sun `check_dir_light_mesh_visibility` in `bevy_light`, which walks every entity without `NotShadowCaster` once per cascade on the CPU). An object behind the camera or outside a cascade costs no draw.
- **`VisibilityRange`** (`bevy_camera::visibility::range`): a per-entity distance band with a dithered crossfade margin, checked against the camera every frame. It applies to the sun's shadow views too, measured from the **camera**: an entity out of its range neither draws nor casts. There is **no** separate shadow range; a part that should draw but stop casting needs `NotShadowCaster`.
- **Occlusion culling** (`bevy_render::occlusion_culling::OcclusionCulling`): two-phase, on the GPU, off by default; needs a `DepthPrepass` on the camera and is incompatible with the deferred prepass. OPTS adds a `DepthPrepass` only while light cones shine at night (`light_cones.rs`).

## 2. What it costs: known and unknown

| Quantity | Value | Source |
| :--- | :--- | :--- |
| Terrain triangles | 7,200 per tile: 25 tiles (radius 2) = 0.18 M, 49 (radius 3) = 0.35 M; the whole of Gladbeck (1,649 tiles) = 11.9 M | §1 terrain; Gladbeck tile count from [terrain rendering](terrain-rendering.md) §6 |
| Scenery mesh entities | Berlin-Spandau 9,242; Grundorf 916 | [instancing research](instancing-research.md) §1 |
| Sorted (blended) draws | 127 of about 140 main-pass draw items in a Spandau city view | the same, §1 |
| Frame time, Berlin-Spandau | 3.7–3.9 ms (about 260 fps), **CPU-bound**: the GPU works 1.3–2.0 ms of it | [lighting cones](lighting-cones.md) §C (2026-10-17, park and overview views, not the densest street) |
| Shadow pass GPU time, Berlin-Spandau | 0.17–0.25 ms | the same |
| Effect of culling 44 % of the shadow casters | none measurable: frame time and shadow-pass GPU time unchanged within noise | §3.1 *Measured* (2026-11-05) |
| Terrain share of GPU time | **not measured** ❓ (Q-LC-02) | — |

The tools to measure exist: `render.stats` counts the `Shadow` phase's batch sets, bins and entities per view, and with `OPTS_RENDER_DIAG=1` it reports the GPU time of every pass (`gpu_diagnostics_ms`); `scripts/instancing-bench.sh` drives the game through fixed camera poses on a real map and records both.

## 3. Design

### 3.1 Per-part shadow culling (small parts stop casting at a distance)

**Problem.** Every scenery mesh within 300 m casts into up to four cascades. A part 0.3 m across at 150 m covers about one texel of the far cascade (four 2048-texel cascades over 300 m: roughly 0.15–0.3 m per texel in the far ones ❓), so its shadow is invisible noise, but it is handled for every cascade anyway.

**Where a gain can come from.** Not mainly the GPU: on Berlin-Spandau the whole shadow pass costs 0.17–0.25 ms of GPU in a frame that is CPU-bound at 3.7–3.9 ms (§2). The candidate is the **CPU**: every caster is visibility-checked once per cascade and queued into each cascade's `Shadow` phase. `NotShadowCaster` removes a part from both. Against that stands the culling pass itself and the re-extraction of every part whose component changes. Whether the balance is positive is exactly Q-LC-01; the gate in §4 is the **frame time**, not the GPU time.

**Rule.** A scenery mesh part casts while its distance from the main camera is at most `k · r`, with `r` the radius of the part's world-space bounding box (`Aabb` half-extents times the largest scale) and `k` the *shadow detail* factor; beyond it the part gets `NotShadowCaster`. Parts never cast beyond `graphics.shadow_distance` anyway (the cascades end there), so the rule only matters for parts with `k · r` below it. With `k = 100`: a part of 0.3 m radius (a bollard) casts to 30 m, one of 1 m (a sign) to 100 m, one of 3 m or more to the end of the cascades.

- **Per part, not per object.** The draw distance works on object roots and has a measured radius only for objects with `[LOD]` (§1); here each mesh entity has its own `Aabb`, so a small door handle on a large house stops casting while the house still casts.
- **Which parts.** Only meshes of placed scenery objects, marked at spawn (`ShadowCullCandidate`). Not terrain (hills shadow valleys), not splines (a bridge deck casts a large shadow; flat road ribbons lie on the ground and cast onto nothing visible, a possible later step), not vehicles (few, and their shadows are close to the camera), not trees in the first step (billboards, M-0144 wind).
- **When.** Every 0.25 s like the draw distance, with 10 % hysteresis on `k · r` so a part at the border does not toggle; only parts whose state changes get a component change.
- **Never re-enabled by mistake.** The system inserts and removes its own marker together with `NotShadowCaster`, and never touches a part that already had `NotShadowCaster` for another reason (the pattern of `DistanceCulled`).
**Measured (2026-11-05)** with `scripts/shadow-cull-bench.sh`: one game run on Berlin-Spandau (entry *Rathaus Spandau*, MAN SD77, noon, no traffic, simulation paused), release build without LTO, five camera poses, `k` alternating between 0 and 100 three times per pose (8 s samples) so both sides share the machine's load (load average 6 to 9: the rustrim session was building; the machine was not quiet).

| Pose | Frame time, off (ms) | Frame time, k = 100 (ms) | Shadow pass GPU, off (ms) | Shadow pass GPU, k = 100 (ms) |
| :--- | :--- | :--- | :--- | :--- |
| driver | 19.55 · 20.72 · 18.74 | 20.19 · 18.86 · 20.14 | 0.394 · 0.397 · 0.407 | 0.405 · 0.392 · 0.428 |
| street | 8.16 · 8.28 · 8.31 | 8.27 · 8.29 · 8.31 | 0.371 · 0.383 · 0.397 | 0.369 · 0.354 · 0.388 |
| left | 8.59 · 9.54 · 8.54 | 8.65 · 8.50 · 8.48 | 0.465 · 0.476 · 0.489 | 0.479 · 0.462 · 0.470 |
| far | 8.19 · 8.21 · 8.16 | 8.16 · 8.15 · 8.13 | 0.353 · 0.353 · 0.350 | 0.354 · 0.341 · 0.342 |
| high | 6.07 · 5.73 · 5.40 | 5.99 · 5.36 · 5.37 | 0.310 · 0.305 · 0.309 | 0.286 · 0.307 · 0.299 |

The culling did what it should: `shadow_casters` (every `Mesh3d` without `NotShadowCaster`, `render.stats --parts`) fell from about 34,800–36,500 to 19,400–20,300 (−44 %). It saved nothing: the frame times of the two settings overlap at every pose, and the shadow pass's GPU time (timestamp queries, unaffected by the CPU load) is the same to within 0.03 ms. The `Shadow` phase was already batched (32 to 48 multi-draw calls over the 4 cascade views outside, 81 over 8 views from the driver's seat); with GPU preprocessing the small parts cost next to nothing. The process CPU time per frame (10 ms ticks) was too noisy to read. **Answer to Q-LC-01: shadow casters are not a cost worth culling on this map and machine.** Not measured: an open-country map (no reason to expect more casters than a city) and a weaker GPU (the GPU time did not move, so little to gain there either).

A side observation for later work: the driver pose costs 19–21 ms against about 8 ms for the outside poses on the same spot, with eight shadow views instead of four (the cab's mirrors add views, [lighting cones](lighting-cones.md) §E) ❓ not analysed here.

- **Setting.** `graphics.shadow_detail`: *Off* (no culling), and steps of `k`. **Outcome:** default *Off*, and the setting is hidden from the settings page (status *internal*: it stays reachable through `settings.set` for a later measurement on other hardware). The default is chosen by the measurement (§4 step 2): **on** only if it lowers the frame time on a real map beyond run-to-run noise, without a visible loss in the near field. If it does not, the code stays behind the setting with *Off* as the default (a weaker GPU may still profit), or is removed.

### 3.2 Terrain level of detail

**When it matters.** Only with many resident tiles: at the automatic radius terrain is a fraction of a million triangles; with *Load the whole map* on a large map it is millions (§2). Measure first (Q-LC-02); if terrain is not a visible share of GPU time at the radii players use, this step stays a design.

**Design.**

1. **Coarse meshes from the carved grid.** At tile build time, after `carve_terrain`, build two more meshes by taking every 2nd and every 4th grid point (31 × 31 and 16 × 16 vertices: a quarter and a sixteenth of the triangles). The build is cheap (no new data; the heights exist).
2. **Carving must survive decimation.** A coarse triangle interpolates between kept vertices and can lie *above* a lowered vertex it skips, which would push terrain through a road at a distance. Each kept vertex therefore takes the **minimum** height of the fine vertices it stands for (its 2 × 2 or 4 × 4 block). The coarse surface then lies at or below the fine one, never above it ❓ (to be checked on the carve tests, Q-LC-03).
3. **Cracks at the borders** between tiles of different detail: a skirt (a vertical strip 2 m down along the tile edge) on the coarse meshes. Its cost is small and it needs no knowledge of the neighbour.
4. **Switching**: `VisibilityRange` on the three meshes of a tile (fine to 600 m, medium to 1,500 m, coarse beyond ❓ distances to be tuned), with the crossfade margin; the ground material must support the dither (Q-LC-04), otherwise a plain switch.
5. **Physics and queries do not change**: `TileTerrain::height` and the ground layers keep reading the full grid.

### 3.3 Draw distance on `VisibilityRange` (option, not planned)

`VisibilityRange` could replace the 0.25 s CPU pass of the draw distance, with a crossfade instead of popping. It is not planned now: it needs the range on every mesh entity (not on the root: ranges do not propagate), a real radius for objects without `[LOD]` (today 50 m), dither support in every material OPTS uses (the ground material extension, blended glass ❓, Q-LC-04), and it hides shadows together with the mesh, which is the intended behaviour. Worth revisiting once 3.1 has shown how costly component changes on many entities are.

### 3.4 Occlusion culling (option, measure)

In a city street most buildings are hidden behind the first row. Bevy's GPU occlusion culling skips those without a bake. It needs a `DepthPrepass` on the main camera (an extra depth pass of everything that was visible last frame). One measurement on a city pose and an open-country pose, on and off, decides whether it is worth a setting (Q-LC-05). It does not apply to shadow cascades.

### 3.5 Instancing and asset sharing

Productising layer (a) of the instancing research (de-duplication across tiles, trees, splines; tests that equal parts share handles and repaints do not) is part of M-0029's instancing item; the research's §9 is its plan and is not repeated here. Static merging (c) stays behind a setting, off, until a tile-spawn profile justifies it. A custom instancing pipeline is rejected (research §6).

## 4. Order of work

| Step | What | Gate |
| ---: | :--- | :--- |
| 1 | This page | — |
| 2 ✅ | Per-part shadow culling (§3.1), **done 2026-11-05: no gain, off and hidden** with `graphics.shadow_detail`; measure frame time (average and 1 % low), `Shadow` phase entities and shadow-pass GPU time on Berlin-Spandau (city) and one open map at the harness poses, off and on, on an otherwise quiet machine | Default on only with a frame-time gain beyond noise and no visible near-field loss |
| 3 | De-duplication of assets (§3.5, research §9 item 1) | Tests that equal parts share handles; asset counts in `render.stats --parts` |
| 4 | Measure terrain share (Q-LC-02) and occlusion culling (Q-LC-05) | Decides whether 3.2 and 3.4 are built |
| 5 | Terrain LOD (§3.2), only if step 4 shows a need | Carve tests stay green; no terrain through roads at any distance |

## 5. Open questions

| ID | Question | Status |
| :--- | :--- | :--- |
| Q-LC-01 | How much CPU time do the shadow casters cost (visibility per cascade, queueing), and does culling small parts lower the frame time once the culling pass itself is paid? | ✅ 2026-11-05: culling 44 % of the casters changed neither frame time nor shadow-pass GPU time on Berlin-Spandau (§3.1) |
| Q-LC-02 | What share of GPU time is terrain at the automatic radius and with the whole map loaded? | ❓ |
| Q-LC-03 | Does the block-minimum decimation keep coarse terrain at or below every carved road surface, also across tile borders? | ❓ |
| Q-LC-04 | Which OPTS materials support `VisibilityRange` dithering (ground material extension, alpha-masked, blended)? | ❓ |
| Q-LC-05 | Does occlusion culling (with its depth prepass) save time in a city street, and does it cost time in open country? | ❓ |
| Q-LC-06 | The texel size of the far cascades with OPTS's cascade settings (the 0.15–0.3 m in §3.1 is an estimate) | ❓ |

## 🔗 Related Documents

[M-0029](../roadmap/phase-5/M-0029.md) · [Scenery runtime](scenery-runtime.md) · [Terrain rendering](terrain-rendering.md) · [Terrain carving](terrain-carving.md) · [Map streaming](map-streaming.md) · [Instancing research](instancing-research.md) · [Lighting cones](lighting-cones.md) · [Settings registry](settings-registry.md)
