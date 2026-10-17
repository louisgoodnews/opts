# Mesh Instancing Research (Bevy 0.20 Automatic Batching, De-duplication, Static Merging)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Research done, prototype on branch `research/mesh-instancing` (not merged) |
| **Date** | 2026-10-13 |
| **Milestone** | [M-0082](../roadmap/phase-5/M-0082.md) (field report R-23) |
| **Code** | `opts-bevy` `instancing.rs` (`OPTS_INSTANCING`, `AssetPool`, `StaticMerger`), `render_stats.rs` (`render.stats`), `model.rs`; example `instancing_inventory`; `scripts/instancing-bench.sh` |
| **Related** | [Performance profiling](performance-profiling.md), [Scenery runtime](scenery-runtime.md), [Visual A/B](visual-ab.md), [Bevy 0.20 and DLSS](bevy-0-20-dlss.md) |

## 1. Summary (read this first)

* **Bevy 0.20 already batches and instances OPTS scenery.** Entities that share a mesh and a material handle are drawn instanced; with GPU preprocessing and *bindless* `StandardMaterial` even entities with *different* meshes and materials share one `multi_draw_indirect` call. Measured on an RTX 3060 (Vulkan): Berlin-Spandau with 9,242 mesh entities issues **11 opaque/alpha-mask multi-draw calls**, Grundorf with 916 entities 7. The renderer does not suffer from "one draw call per object".
* What is *not* batched are **blended parts** (windows, glass, lenses): the `Transparent3d` phase is sorted per entity. In Spandau 127 of ~140 main-pass draw items are transparent. This, not repeated opaque meshes, is where draw calls remain.
* **Layer (a), de-duplication by content**, cuts material and asset counts a lot (assets −28 % Spandau, −28 % Gladbeck, −32 % Grundorf; visible distinct materials 675 → 230) but does **not** change draw calls and shows no frame-time gain beyond noise. It is cheap and safe.
* **Layer (c), static merging** (the prototype stands in for manual instancing, see §6) removes 18–56 % of the mesh entities and lowers process CPU time per frame by about 16–18 % in two of three scenes; draw calls stay the same. It costs culling granularity and is the only layer with real risks.
* **Recommendation: adopt partly.** Adopt (a) as a normal loader change; offer (c) for trees and flat static scenery behind a setting only after the follow-ups of §9; **reject** a custom instancing pipeline (it would duplicate the PBR pipeline to gain nothing Bevy does not already do). Proposed milestone in §9.

Caveats up front: the machine was shared with other builds (load average 20–80 on 16 cores) during all runs, so **frame times are not reliable**; draw-phase counts, entity and asset counts are deterministic. Visual equality holds within run-to-run noise (§7), not bit-exactly.

## 2. Inventory (offline, whole maps)

`cargo run --release -p opts-bevy --example instancing_inventory -- <OMSI dir> <map>...` parses every `tile_*.map`, `.sco` and `.o3d` header; no GPU. A *part* is one `Mesh3d + MeshMaterial3d` entity (a mesh file's material slot with triangles per placed object; a tree billboard is one part). Scrambled meshes count every table slot (✓ counts for those are estimates). Numbers are for the **whole map**, not the streamed radius.

| Map | Tiles | Placed objects | Trees | Parts | Flat static / LOD-only / dynamic parts | Unique mesh files | Unique materials (content) | Unique (mesh, material) pairs | Repeat factor parts/pairs (map; median per tile) |
| :--- | ---: | ---: | ---: | ---: | :--- | ---: | ---: | ---: | :--- |
| Grundorf (rural) | 19 | 1,095 | 428 | 1,757 | 959 / 342 / 456 | 131 | 105 | 250 | 7.0 ; 2.1 |
| Waldhofen (rural) | 108 | 100,107 | 77,496 | 109,177 | 105,126 / 107 / 3,944 | 1,003 | 735 | 1,947 | 56 ; 11 |
| Bad_Huegelsdorf_2020 (town) | 208 | 171,223 | 121,014 | 189,689 | 176,874 / 1,057 / 11,758 | 3,200 | 2,727 | 5,515 | 34 ; 6.4 |
| Berlin-Spandau (dense city) | 329 | 29,450 | 8,390 | 46,245 | 28,735 / 5,306 / 12,204 | 994 | 684 | 3,234 | 14 ; 2.6 |
| Gladbeck (city, depot entry) | 1,744 | 242,440 | 114,231 | 353,272 | 283,863 / 11,877 / 57,532 | 1,630 | 1,208 | 2,934 | 120 ; 4.3 |
| TH_Wald (forest) | 191 | 299,429 | 175,821 | 415,492 | 201,426 / 109,313 / 104,753 | 1,630 | 1,219 | 2,805 | 148 ; 20 |
| Hamburg109 (city) | 301 | 21,629 | 3,668 | 26,712 | 16,285 / 0 / 10,427 | 1,430 | 874 | 3,867 | 6.9 ; 2.5 |

Findings:

* **Repetition is large over a map and small per tile** (median 2–20 per tile): a tile (≈ 300 m) holds few copies, so per-tile instancing buys less than the map-wide factor suggests; sharing must be map-wide (which the loader's job-wide cache already is).
* **Trees dominate** forest and rural maps (65–80 % of parts) and **every tree is today one mesh and one material asset** (`trees.rs`). Most repeated objects are trees, shrubs, grass, traffic lights, street signs, bus-stop fittings.
* **55–96 % of parts sit in objects that never change at run time** (no script, light, animation, `[visible]`, LOD or driven light map): 55 % Grundorf, 62 % Spandau, 80 % Gladbeck, 93–96 % Bad Hügelsdorf and Waldhofen, 49 % TH_Wald (grass and forest objects have LOD groups and scripts).
* Mesh content duplicates across add-ons are rare (994 files → 994 contents in Spandau; 3,200 → 2,947 in Bad Hügelsdorf).
* The game's loader **already shares** mesh and material handles for the same file and `[matl]` overrides (cache `ContentCache.parts`, job-wide, `model.rs`). Different mesh files with the same texture, trees, splines and vehicle parts did not share.

## 3. How Bevy 0.20 batches and instances (source: `bevy_*-0.20.0`)

* **Automatic batching**: a mesh entity without `NoAutomaticBatching` is batchable (`bevy_pbr/src/render/mesh.rs`, `should_batch`). Opaque and alpha-mask meshes are put into *bins* by `Opaque3dBatchSetKey` = {pipeline, draw function, **material bind group index**, mesh vertex/index slab, lightmap slab} and `Opaque3dBinKey` = mesh asset id (`bevy_core_pipeline/src/core_3d/mod.rs`; queued in `bevy_pbr/src/material.rs::queue_material_meshes`). Same mesh and same batch set → one instanced draw.
* **GPU preprocessing and indirect drawing**: with `GpuPreprocessingMode::Culling` (default when the device has compute, `INDIRECT_FIRST_INSTANCE` and enough storage limits, `bevy_render/src/batching/gpu_preprocessing.rs`) meshes are *multidrawable*: **one batch set = one `multi_draw_indexed_indirect(_count)`** with one indirect entry per bin (`bevy_render/src/render_phase/mod.rs`, `BinnedRenderPhaseType::mesh`). A compute pass builds per-instance data and does frustum culling on the GPU (`mesh_preprocess.wesl`, `build_indirect_params.wesl`, `unpack_bins.wesl`).
* **Bindless materials**: `StandardMaterial` is `#[bindless]` (`bevy_pbr/src/pbr_material.rs`), so many materials live in one bind-group slab; the batch set key holds the slab, not the material. Different textures and parameters therefore **do not break a batch** as long as pipeline specialisation (alpha mode, cull mode, double-sided, vertex layout, depth bias) matches.
* **Per-instance data without breaking batches**: `MeshTag` (`examples/shader/automatic_instancing.rs`: "Automatic instancing works with any Material, including the `StandardMaterial`"); per-entity transforms are instance data anyway. This is the supported route for a per-instance tint or light-map index.
* **What breaks batching**: `NoAutomaticBatching`; blended/premultiplied/additive/multiply alpha (`Transparent3d` is a *sorted* phase, one item per entity, adjacent identical items may merge); transmission; skinned and morphed meshes; different pipeline keys; different lightmap slabs; materials that need deferred or a custom draw function.
* **Custom instancing** (`examples/shader_advanced/custom_shader_instancing.rs`) is "intended for advanced users … generally recommended to try the built-in instancing before". It replaces the mesh pipeline; PBR lighting, shadows, fog and clustering would have to be re-implemented or re-bound.
* **Diagnostics**: `RenderDiagnosticsPlugin` (`bevy_render/src/diagnostic/mod.rs`) gives CPU and GPU time per pass on Vulkan/DX12 (`OPTS_RENDER_DIAG=1`). The 0.20 release post (bevy.org/news/bevy-0-20) only mentions a 132× faster GPU mesh extraction from per-column change ticks; it says nothing about new instancing features.

### What prevents batching in OPTS today (found in the code)

| Cause | Where | Effect |
| :--- | :--- | :--- |
| One `Mesh3d + MeshMaterial3d` **entity per part**; per-object hierarchy only when scripted | `scenery_runtime/spawn.rs`, `model.rs attach_model_parts` | Not a draw-call problem (batched) but entity, transform-propagation, visibility and extraction cost per part. |
| Materials created per mesh part (`materials.add`), never by content | `model.rs` | 298 materials for 336 visible pairs in Grundorf; no draw effect (bindless) but asset and prepare cost. |
| One mesh and one material per **tree** | `trees.rs` | Every tree an own bin and asset. |
| Spline ribbons: unique mesh per segment | `tile_ground.rs` | Unique by nature (world-space extrusion); materials were per (`.sli`, texture). |
| Per-object **light-map materials** (script variable → private material copy on first change) | `scenery_runtime/effects.rs`, `vehicle_lights.rs` | Splits an instance group per object; night-switch maps without variable share one material. |
| Rendered **text textures** (bus-stop displays) | `tile.rs` | Unique image and material per object (1.5–5 % of parts). |
| `AlphaMode::Blend` parts (glass, lenses, mode-2 alpha) | `model.rs` | Sorted per entity: the remaining draw-call lever (§5). |
| Script-driven transforms and `Visibility` (`[newanim]`, `[visible]`, LOD groups) | `scenery_runtime` | Correct as ordinary entities; excluded from any merging. |
| Per-tile duplicates | none | The cache is job-wide; tiles do not duplicate handles. |
| Decode cache | `descramble_cache` | Not a cause: decoded meshes enter the same part cache. |

## 4. Prototype layers (switch: `OPTS_INSTANCING=off|dedupe|merge`, read once, baseline stays `off`)

* **(a) `dedupe`**: `AssetPool` returns one handle per content hash for meshes (attribute bytes + indices) and per `Debug` form of `StandardMaterial` (texture handles, colours, alpha, culling). Used for `.o3d` parts, trees and spline materials. Safe with the runtime because the only in-place material writers either act on equal-content materials that switch together (night light maps, `NightlightA`) or copy first (`vehicle_lights.rs`, `effects.rs`).
* **(b) verify automatic batching**: the `render.stats` control command (new, `render_stats.rs`, listed in the generated [control reference](../reference/control-protocol.md)) returns frame times since the last reset (average, median, p99, 1 % low), per-phase batch sets, bins and unbatchable entities summed over views, optional distinct mesh/material/pair counts of visible entities, memory (RSS, peak) and, with `OPTS_RENDER_DIAG=1`, GPU time per pass. The phase counters run in the render world after queueing; cost is a few map lookups per frame.
* **(c) `merge`** (implies (a)): flat static objects (see §2), tree billboards and spline ribbons of a tile are baked into one mesh per (material, 64 m cell, tile) at tile load (`StaticMerger`, `OPTS_MERGE_CELL`); blended materials and meshes with other than position/normal/uv0 stay separate entities. Objects with scripts, lights, animation, `[visible]`, LOD or driven light maps are never merged, so animated and script-driven objects stay exact (their entities and rules are untouched). Merged entities hang under the tile root, so tile eviction frees them with the tile.

## 5. Measurements

Setup: RTX 3060, Vulkan, release build (thin LTO), 1920×1080-class window, MAN SD77, simulation paused at fixed vehicle-relative camera poses (six poses: driver, street, left, far, high, behind; 4 s settle + 10 s sample each), no traffic, hour 12, throw-away XDG dirs and a copy of the decoder cache. Scenes: **dense city** Berlin-Spandau entry "U-Bhf. Rathaus Spandau", **rural** Grundorf (default entry), **depot** Gladbeck entry "Betriebshof Urban Reisen". Two repetitions per mode where the run succeeded (Grundorf `off` has one run, `dedupe` one run, `merge` two: the other runs hit the start-up timeout under machine load). Raw JSON stays outside the repository (`~/.local/share/opts/benchmark/instancing/`).

| Scene / mode | Mesh entities | Assets | Opaque + mask multi-draw calls | Transparent items | Shadow calls (4 views) | Process CPU ms/frame | Wall ms/frame (noisy) | RSS MB | Load s (noisy) |
| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Grundorf off | 916 | 1,466 | 7 | 7 | 20 | 15.5 | 50.0 | 360 | 118 |
| Grundorf dedupe | 916 | 1,003 | 7 | 7 | 20 | 16.4 | 33.3 | 364 | 96 |
| Grundorf merge | 748 | 953 | 7 | 7 | 20 | 15.0 | 17.7 | 365 | 64 |
| Spandau off | 9,242 | 7,741 | 11 | 127 | 29 | 19.5 | 25.2 | 430 | 31 |
| Spandau dedupe | 9,242 | 5,540 | 11 | 128 | 29 | 19.8 | 23.3 | 453 | 33 |
| Spandau merge | 6,239 | 4,720 | 11 | 127 | 29 | 16.0 | 18.3 | 384 | 42 |
| Gladbeck off | 3,554 | 4,317 | 9.7 | 30 | 29 | 15.5 | 19.0 | 386 | 36 |
| Gladbeck dedupe | 3,554 | 3,102 | 9.7 | 30 | 29 | 14.5 | 18.3 | 360 | 49 |
| Gladbeck merge | 1,576 | 1,772 | 9.7 | 30 | 29 | 12.7 | 12.1 | 712 (one outlier run 1,105) | 42 |

Reading: **(b) holds**: 916–9,242 mesh entities collapse to 7–11 opaque multi-draw calls in every mode; de-duplication and merging do not change the draw calls because there was nothing left to batch. The transparent phase is the draw-call bulk (127 items in Spandau, per entity; adjacent identical items may merge at submission, so this is an upper bound). **Assets** fall 28–32 % with (a) and 35–59 % with (c). **Process CPU time per frame** (user + system over the sample window, less sensitive to other load than wall time but still affected) drops 16–18 % with (c) in Spandau and Gladbeck and is unchanged in Grundorf; (a) alone is within noise. Wall-clock frame time, 1 % low and load time varied by more than the effect between repetitions (load average 20–80) and are **not evidence either way**; the 1 % low figures (9–121 fps) are shown in the raw data only. GPU time per pass was 2–3 ms in total, so these scenes are CPU/main-thread bound, not draw-call bound: the earlier profile ([performance-profiling.md](performance-profiling.md)) points to tile-spawn spikes, transform propagation and extraction per entity, which fewer entities address.

Memory: resident set is unchanged within noise; merging duplicates vertex data per placement (baked world copy) while the source meshes stay cached, so it trades memory for entities; one Gladbeck run showed a transient 1.1 GB RSS that did not repeat.

## 6. Why no custom instance buffer

A manual instance buffer with per-instance transform and tint/light-map index would need its own pipeline and draw command: OPTS materials are PBR with shadows, fog and clustered lights, so it would copy the mesh pipeline (the Bevy example is unlit and calls itself advanced). The result would be what the batcher already produces (§5: 7–11 draws). Per-instance variation is available without it through `MeshTag`. Static merging captures the part that *is* costly (entity count) with ordinary Bevy components, so it replaced the "manual instancing" item of the task. **Animated and script-driven objects** stay ordinary entities in every layer; if a future milestone wants them instanced they must keep one entity per object (transform and visibility are per-entity state) and rely on automatic batching, with `MeshTag` carrying tint or light-map index.

## 7. Visual equality

Same poses, `opts-tools image-metrics --crop 400,100,1120,700` (the HUD clock differs between runs), mean absolute error / PSNR against the first `off` run. Run-to-run noise (`off` against a second `off`) is the reference:

| Scene, pose | off vs off (noise) | off vs dedupe | off vs merge (2 runs) |
| :--- | :--- | :--- | :--- |
| Spandau driver | 0.00016 / 54.0 dB | 0.00021 / 51.2 dB | 0.00021 / 53.0 dB; 0.00019 / 53.0 dB |
| Spandau left | 0.00171 / 42.4 dB | 0.00255 / 39.5 dB | 0.00245 / 39.9 dB; 0.00278 / 39.1 dB |
| Spandau high | 0.00091 / 47.0 dB | 0.00176 / 42.0 dB | 0.00166 / 41.0 dB; 0.00105 / 44.9 dB |
| Gladbeck driver | 0.00183 / 41.0 dB | 0.00181 / 41.0 dB | 0.00355 / 36.1 dB; 0.00227 / 39.7 dB |
| Gladbeck high | 0.00664 / 34.9 dB | 0.00674 / 34.9 dB | 0.00578 / 35.7 dB; 0.00661 / 35.0 dB |
| Grundorf driver | (no second off) | 0.00175 / 43.9 dB | 0.00193 / 43.2 dB; 0.00143 / 45.2 dB |

Differences between modes are of the same size as the difference between two runs of the same mode (sun and shadow cascades move with the simulation clock at the moment of the pause; tile streaming order). A side-by-side look at Gladbeck "high" showed no visible change. **Not proven**: bit-exact equality, night scenes (light maps are excluded from merging but shared in (a)), moving cameras (shimmer is invisible to these metrics).

## 8. Risks and compatibility

| Topic | Assessment |
| :--- | :--- |
| Repaints | Texture replacement is part of the part-cache key and of the material content, so repainted parts never share with originals. |
| Light maps (`.LM`, `[matl_lightmap]`) | (a) shares night-switch materials by content: they switch together by design. Script-variable maps copy on first change (unchanged). (c) never merges them. A future merged night map needs per-cell emissive control. |
| Seasonal switches, `[visible]`, alpha scale | Only dynamic objects use them; they are excluded from (c). `hidden_by_variable` parts are dropped by the loader as before. |
| Culling and LOD | (c) coarsens frustum and shadow culling to the 64 m cell; per-object `[LOD]` bands are not merged (excluded). Distance rules proposed in [performance-profiling.md](performance-profiling.md) (visibility ranges for small scenery) need per-object entities and would conflict with merging. |
| Tile eviction / hot reload | Merged entities are tile children and die with the tile; a hot-reload of one `.o3d` must rebuild its tile (already the rule for tile dependencies). |
| Collision, picking, `OPTS_DEV_LOOK_AT` | Collision is registered separately from rendering; the look-at aid disables merging. Object picking by entity (none today) would not work for merged parts. |
| Drivers, platforms | Everything depends on Bevy's GPU preprocessing (`GpuPreprocessingMode`): on adapters without compute (GL) or only preprocessing (some mobile, DX12 without culling) batching falls back to per-batch draws and merging would then help more. Not tested on Windows/macOS; measurements are one NVIDIA/Vulkan machine. |
| DLSS | Merged static meshes have static transforms (zero object motion) and use the same vertex layout; no interaction expected, untested (needs the DLSS milestone, [M-0048](../roadmap/phase-5/M-0048.md)). |
| Memory | (c) holds a baked copy per placement; large forests (TH_Wald, 176 k trees) would add tens of MB of vertex data; cap per merged mesh is 200 k vertices. |
| Prototype limits | `Debug`-string material key is slow-ish and fragile (use field hashing); merged meshes keep full `RenderAssetUsages`; no per-frame merge work, so streaming spikes are unchanged; blended parts and vehicles are not handled. |

## 9. Recommendation and milestone proposal

**Adopt partly.**

1. **Adopt (a) de-duplication** (small, safe): replace `Debug` keys by field hashing, keep the pool across tiles, add trees (billboard mesh by (height, width) rounded) and spline materials, add the stat to `scene`/`render.stats`.
2. **Keep `render.stats`** (cheap, useful beyond this research); make the draw-count figures part of regression checks.
3. **Static merging (c): decide after a profile of the tile-spawn cost**, because it removes entities but costs load-time CPU and memory; if adopted, only for trees and spline ribbons first (largest share, no dynamic state), behind a setting with a default of off.
4. **Reject** a custom instancing pipeline.
5. **Look at the transparent phase**: 127 sorted items in a city view; options are cheaper blended materials (alpha-test where the texture allows), per-material merging of window glass of one object, or visibility ranges.

Proposed milestone *M-00xx "Scenery asset sharing and static batching"* (Phase 5, medium): (1) productise the pool (field-hash keys, trees, splines, vehicles; tests that equal parts share handles and repaints do not); (2) frame-time benchmark on an otherwise idle machine (the numbers here are noisy) with `scripts/instancing-bench.sh`, including 1 % low and tile-spawn spikes; (3) static merge for trees and splines behind a setting, with per-cell culling check and hot-reload test; (4) transparent-phase study; (5) Windows/macOS and no-compute fallback check; (6) DLSS check after M-0048. Estimated effort: (1) 2 days, (2) 1 day, (3) 4 days, (4) 2 days, (5)–(6) 2 days.

## 10. Reproduce

```sh
cargo build --release -p opts-game -p opts-cli -p opts-tools -p opts-bevy --bins
# inventory (no GPU)
cargo run --release -p opts-bevy --example instancing_inventory -- "<OMSI dir>" Grundorf Berlin-Spandau
# benchmark; DECODER_CACHE must hold a COPY of ~/.cache/opts/descramble under opts/descramble
OPTS_BIN=target/release/opts-game OPTS_CLI=target/release/opts-cli DECODER_CACHE=<dir> \
  scripts/instancing-bench.sh spandau_merge merge Berlin-Spandau "Rathaus Spandau" MAN_SD200/MAN_SD77.bus
```

**ADR-0002 attestation:** only OMSI text files and `.o3d` headers/face tables through the project's own parsers were read; no OMSI binary was inspected; screenshots were written outside the repository and deleted afterwards; no OMSI asset or screenshot is committed.
