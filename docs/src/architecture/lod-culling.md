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

**A side observation, followed up in §3.6.** The driver pose cost 19–21 ms against about 8 ms for the outside poses at the same spot, with eight shadow views instead of four: the cost of the mirror cameras.

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

**De-duplication (layer (a)) is not made the default.** The prototype already exists and works behind `OPTS_INSTANCING=dedupe` (`AssetPool`, used for `.o3d` parts, glass, trees, spline water and vehicle overrides). The research measured what it buys on three maps ([research](instancing-research.md) §5): assets −28 to −32 %, but process CPU time per frame "within noise", resident memory "unchanged within noise" and the load time inside the run-to-run spread; draw calls do not change because Bevy already batches. Turning it on would change the material of every part in the game for a saving nobody measured, and it needs a hand-written key: the current key is the material's `Debug` text, which is robust (a field Bevy adds in an upgrade is included automatically) and a field-by-field hash is not (a forgotten field would let two different materials share one handle). So the same rule as §3.1 applies, measure then switch, and the switch stays `off` until a measurement shows a gain in load time or memory on a quiet machine. It is not a code task; it is a measurement task (Q-LC-07).

Static merging (c) stays behind a setting, off, until a tile-spawn profile justifies it. A custom instancing pipeline is rejected (research §6).

### 3.6 Mirror cameras are the largest single cost on a big map

**Measured (2026-11-05/06, Berlin-Spandau, MAN SD77 at the *Rathaus Spandau* entry, driver pose, release build without LTO, `scripts/shadow-cull-bench.sh`).** Each mirror that renders is another camera: the scene is culled and shadowed once more for it (the `Shadow` phase counts 4 views for the main camera and 4 more per active mirror camera). `render.stats --parts` lists the cameras (`parts.cameras`).

The two mirror settings, `graphics.mirror_count` and `graphics.mirror_rate`, **act only with `graphics.light_quality_mode = custom`**; in the default automatic mode the quality governor owns them ([lighting cones](lighting-cones.md), the settings page shows them disabled). An A/B of the same settings in automatic mode changes nothing, which is what my first attempt found and mis-read. With the mode set to custom (`PRE=graphics.light_quality_mode=custom`):

| `mirror_count` (rate 2) | Active cameras | Shadow views | Frame time (ms, per sample) | Process CPU per frame (ms) | GPU, all passes (ms) |
| ---: | ---: | ---: | :--- | :--- | :--- |
| 0 | 1 | 4 | 7.8 · 7.7 · 7.9 · 7.8 | 25.3 · 24.3 · 26.6 · 26.0 | 4.1–4.2 |
| 1 | 1–2 | 8 | 12.1 · 11.9 · 12.0 · 12.6 | 34.4 · 33.6 · 34.0 · 36.0 | 4.2 |
| 2 | 2 | 8 | 17.0 · 17.5 · 16.8 · 16.8 | 42.9 · 44.2 · 42.1 · 42.2 | 3.6–3.7 |

At count 2 the update rate changes the cost as the schedule says: `mirror_rate` 4 gave 12.9–13.5 ms (34 ms CPU), 2 gave 16.9–17.6 ms, 1 gave 17.1–17.3 ms with 3 cameras and 12 views (the frame did not get longer than at rate 2, although a second mirror renders every frame instead of every other; the process uses only about 2.5 of the 16 cores, so parallel work in the extra views may hide part of it, or the display rate limits the frame at about 17 ms ❓). Count 1 and rate 4 scale with count 2 in a straight line (+4.2, +5.4 and +9.2 ms over the 7.8 ms base), which is why I take the 9 ms as real; it is not a ceiling-limited figure.

- **Two mirrors at the default rate cost 9 ms of a 17 ms frame**, and 17 ms of process CPU time per frame (medians 42.5 against 25.7). The GPU total does not move (3.6–4.2 ms): the cost is on the CPU.
- **It is not the far plane.** With the mirror cameras' far plane at 250 m and at 120 m (Bevy's default is 1,000 m) the frame took 17.0–17.2 ms, as before. Bevy's cascades ignore the camera's far plane (`build_directional_light_cascades` uses the light's own bounds), and a short far plane does not shrink the per-entity passes below.
- **It scales with the scene.** On Grundorf (2,177 mesh entities) two mirrors cost +2.6 ms in the one sample not pinned at the display rate; on Spandau (34,474) +9.2 ms. With static merging (`OPTS_INSTANCING=merge`, 26,410 entities, −23 %) the extra CPU time of two mirrors fell from 17 ms to 9 ms per frame (medians of four samples, −47 %), and the base cost without mirrors from 26 ms to 22 ms.
- **Likely cause, not proven** ❓: for every active camera Bevy runs a frustum pass over all mesh entities for the camera and one per shadow cascade (`check_visibility_cpu_culling`, `check_dir_light_mesh_visibility`), five passes over 34,000 entities per view. The numbers fit (17 ms of thread time per mirror view) but I did not profile it. Bevy has no per-camera switch for shadow cascades, so a mirror view cannot skip them.
- **Why the default player does not always see 17 ms:** in automatic mode the governor steps the level down when frames miss the deadline, and the ladder (`ladder_preset`) drops the mirrors to one mirror at a quarter rate **only at level 4**, after the cones and their cap (levels 1 to 3). The ladder was priced with the synthetic 0.7 ms per view of [lighting cones](lighting-cones.md) §E; on a 34,000-entity map a mirror at rate 2 costs about 4.5 ms of every frame (half of the 9 ms for two). Nothing here shows which level the governor settles at on a given map ❓.
- **Caveat on the frame times.** In the Grundorf and merge runs, the later samples were pinned to 16.6–16.7 ms whatever the setting, even with the mirrors off: something outside the game limits the rate (display or compositor throttling ❓; the automatic-mode runs also sat at 16.6 ms, where the governor's 60 fps deadline can explain it). Such samples carry no frame-time information; the CPU time per frame and the camera counts still do. The Spandau tables above are from runs that were not pinned at count 0 (7.7–7.9 ms).

**Options, not decided (they change what the player sees):**

1. Shed mirror cost earlier on the automatic ladder: `mirror_every = 4` from level 1 or 2 instead of level 4 halves it (a mirror picture refreshed every fourth frame). The lead's call: [virtual mirrors](virtual-mirrors.md) notes a stale mirror "reads as a bug".
2. Scale the mirror budget with the scene: the extra frame time of two mirrors (one view per frame at rate 2) is about `2 ms + 0.2 µs` per mesh entity (two points, Grundorf and Spandau, Grundorf from one sample; to be checked on a third map).
3. Productise static merging (research layer (c)): it lowers the base cost and the mirror cost together, at the price of the risks of research §8 (culling granularity, memory).
4. A view that skips its shadow cascades needs a change in Bevy (or a patch); not a quick fix.

## 4. Order of work

| Step | What | Gate |
| ---: | :--- | :--- |
| 1 | This page | — |
| 2 ✅ | Per-part shadow culling (§3.1), **done 2026-11-05: no gain, off and hidden** with `graphics.shadow_detail`; measure frame time (average and 1 % low), `Shadow` phase entities and shadow-pass GPU time on Berlin-Spandau (city) and one open map at the harness poses, off and on, on an otherwise quiet machine | Default on only with a frame-time gain beyond noise and no visible near-field loss |
| 3 ⏸ | De-duplication of assets (§3.5): **not made default**, waits for a measurement of load time and memory (Q-LC-07) | A gain beyond noise on a quiet machine |
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
| Q-LC-07 | Does `OPTS_INSTANCING=dedupe` shorten tile loads or lower resident memory on a quiet machine? The research saw no CPU, memory or load-time difference beyond noise (§3.5) | ❓ |
| Q-LC-08 | What do the mirror cameras cost at the driver's seat, and why did `graphics.mirror_count` and `graphics.mirror_rate` seem not to act? | ✅ 2026-11-06: about 9 ms of a 17 ms frame for two at the default rate on Berlin-Spandau; the settings act only in custom lighting mode (§3.6) |
| Q-LC-09 | Where exactly does the CPU time of a mirror view go (the five visibility passes, extraction, queueing), and how does it scale on a third map? | ❓ §3.6 |

## 🔗 Related Documents

[M-0029](../roadmap/phase-5/M-0029.md) · [Scenery runtime](scenery-runtime.md) · [Terrain rendering](terrain-rendering.md) · [Terrain carving](terrain-carving.md) · [Map streaming](map-streaming.md) · [Instancing research](instancing-research.md) · [Lighting cones](lighting-cones.md) · [Settings registry](settings-registry.md)
