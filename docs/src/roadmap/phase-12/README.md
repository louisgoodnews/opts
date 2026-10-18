# 🎨 Phase 12: Visual Fidelity & Advanced Physics

*Goal: Close the visible gap between OPTS and a good OMSI 2 screenshot (dirt, light maps, wet and snowy surfaces, a modern post-processing chain) and add two physical effects the player feels while driving: fuel or battery state and passenger load.*

**Status:** ⏳ Planned 2026-10-18; every milestone is 📝 Proposed and awaits approval by the project lead. Nothing is started. The execution order below is a recommendation; the project lead may reorder it.

The phase is not yet listed in [`ROADMAP.md`](../../../../ROADMAP.md), the book summary or the board; those entries follow after the project lead's checkpoint on these pages.

## Milestones (recommended execution order)

1. [M-0120 — Material Overlays and Light Maps](M-0120.md) — 📝 Proposed; verify and complete dirt, rain and condensation overlays, `[matl_lightmap]` and `[matl_nightmap]` end to end.
2. [M-0121 — Wetness and Snow on Vehicles and Scenery](M-0121.md) — 📝 Proposed; texture `[moisture]`, `[visible] snow_visible` and snow textures; builds on M-0120 and the existing ground wetness.
3. [M-0124 — Passenger Load Physics](M-0124.md) — 📝 Proposed; dynamic mass; independent of the graphics work, but waits for [M-0073](../phase-10/M-0073.md) and [M-0101](../phase-4/M-0101.md).
4. [M-0123 — Fuel and Battery State](M-0123.md) — 📝 Proposed; script-driven consumption, torque cut, HUD, setting, session save.
5. [M-0122 — Graphics Overhaul](M-0122.md) — 📝 Proposed; last on purpose: the audit, ambient occlusion, anti-aliasing and bloom decisions are judged on a scene that already has dirt, light maps and wetness, and after the open Phase 8 items it depends on.

Rationale: M-0120 and M-0121 are small, concrete and mostly verification; M-0122 is the largest and is measured against everything else, so it goes last. M-0123 and M-0124 are physics and can run in parallel with the graphics track by another agent (they touch `opts-sim`, `opts-game` and the save registry, not the render path). This is a recommendation; the user may reorder.

## Dependencies on prior phases

| Milestone | Needs |
| :--- | :--- |
| M-0120 | [M-0045](../phase-4/M-0045.md) (script-driven lights and overlays; its in-game screenshot check is still open) |
| M-0121 | M-0120; the weather rendering and ground wetness merged in 2026-10 ([weather](../../architecture/weather.md)); [M-0045](../phase-4/M-0045.md) |
| M-0122 | Phase 8: [M-0057](../phase-8/M-0057.md) (filtering, harness), [M-0058](../phase-8/M-0058.md) (mipmaps), [M-0059](../phase-8/M-0059.md) (cutout coverage, alpha to coverage with MSAA), [M-0060](../phase-8/M-0060.md) (looks, calibrated sun), [M-0091](../phase-8/M-0091.md) (shadows); the [LightQuality ladder and frame-time governor](../../architecture/lighting-system.md) |
| M-0123 | [M-0069](../phase-10/M-0069.md) (session save registry, merged); [M-0096](../phase-4/M-0096.md) script host; vehicle script engine model ([vehicle functions](../../simulation/vehicle-functions.md)); settings registry ([M-0063](../phase-9/M-0063.md)) |
| M-0124 | [M-0073](../phase-10/M-0073.md) (planned: mass, axles, suspension); [M-0101](../phase-4/M-0101.md) (not started: doors, passengers); [M-0069](../phase-10/M-0069.md) for save |

The brief also named Phase 4 M-0095..M-0102 as inputs: only M-0101 (passenger count) and M-0045 (overlays and lights) are consumed here; M-0095/M-0102 give nothing this phase needs. Phase 11 inputs are optional: [M-0117](../phase-11/M-0117.md) (weather effects on driving) shares the wetness state with M-0121, and [M-0115](../phase-11/M-0115.md) (spatial zones) is where an ambient-occlusion or interior-darkening hint could later hook in; neither blocks Phase 12.

## Feature-to-milestone map (6 items)

| # | Feature (project lead's inventory) | Captured in |
| :--- | :--- | :--- |
| 1 | Dirt on vehicles and scenery | [M-0120](M-0120.md) (overlay materials, not a map type, see corrections) |
| 2 | Light maps | [M-0120](M-0120.md) |
| 3 | Wetness and snow | [M-0121](M-0121.md) |
| 4 | Graphics overhaul (PBR, ambient occlusion, anti-aliasing, bloom, texture detail) | [M-0122](M-0122.md) |
| 5 | Fuel and battery state | [M-0123](M-0123.md) |
| 6 | Passenger load physics | [M-0124](M-0124.md) |

## Corrections to the brief

The inventory was written from the player's point of view. Checked against the format documents and the code, three statements need correcting. The milestones are written on the corrected basis.

1. **OMSI has no separate dirt, rain or snow map file types.** Dirt, rain and condensation are ordinary extra **alpha overlay materials** with `[alphascale]` naming a script variable: `Dirt_Norm` (14,939 uses), `Dirt_Wiped` (794), `Rain_Window_Norm_Wetness` (18,856), `Rain_Window_Front_Wetness`, `Beschlag_Vis_NoWipe` (condensation) ([sco.md](../../formats/text/sco.md), `[alphascale]` table row and the 2026-10-14 correction; [script sets](../../formats/text/vehicles/script-sets.md), `dirt.osc`: variables `dirt_norm`, `dirt_wiped`, `dirt_reflektor_*`, trigger `veh_wash`; [M-0045](../phase-4/M-0045.md)). **Light maps** are `[matl_lightmap]` and `[matl_nightmap]` materials with `[nightmapmode]` 0-4. **Surface wetness** comes from the texture `.cfg` flag `[moisture]` (7,040 textures; [texture-cfg.md](../../formats/text/texture-cfg.md)), **snow** from winter-snow textures and `[visible]` with `snow_visible`.
   The code already holds most of the parsing and part of the runtime, so M-0120 and M-0121 **verify and complete the end-to-end result** instead of starting from zero (checked in the tree, 2026-10-18):
   * `[alphascale]`, `[matl_change]`, `[matl_lightmap]`, `[matl_nightmap]`, `[nightmapmode]` are parsed in `crates/opts-core/src/formats/vehicles/model/parse.rs` (and `ast.rs`) and `crates/opts-core/src/formats/scenery/sco.rs`.
   * Vehicle models bind light maps to a script variable (`crates/opts-bevy/src/model.rs` `TextureOverride::light_map`; `crates/opts-bevy/src/vehicle_lights.rs` `LightMapBinding`); scenery has `SceneryLightMap` (`crates/opts-bevy/src/scenery_runtime/effects.rs`, `spawn.rs`) and live `Envir_Brightness` alpha (`scenery_runtime/alpha.rs`).
   * **Gap, and why [sco.md](../../formats/text/sco.md) still says "OPTS hides them":** `is_static_overlay` in `crates/opts-bevy/src/model.rs` treats every variable starting with `rain_window_`, `dirt_` or `precip_` as always 0 and builds no entity for those overlays; `scenery_runtime/alpha.rs` states the same. The script side already writes `dirt_norm` (`crates/opts-sim/src/script/inputs.rs`, no consumer). So dirt, rain and condensation overlays are parsed but not rendered.
   * `[moisture]` and `[puddles]` are **not** parsed per texture: the only `puddles` matches are the ground shader and weather surfaces (`crates/opts-game/src/in_game/weather/surfaces.rs`, `crates/opts-bevy/src/ground_material.rs`, which carries a `wet` vector with wetness, puddle coverage and snow cover for the terrain only).
2. **Much of the graphics list already exists.** HDR cameras, tonemapping with the Vivid/Natural/Bevy looks (`crates/opts-game/src/look.rs`), MSAA 4x (Bevy default, no setting), the LightQuality ladder with its frame-time governor, cascaded shadows and weather rendering are implemented ([lighting system](../../architecture/lighting-system.md), [camera system](../../architecture/camera-system.md), [weather](../../architecture/weather.md), [texture quality](../../architecture/texture-quality.md)). Texture filtering and mipmaps are Phase 8 work (partly done). **Bloom, TAA and ambient occlusion do not run in the game:** grep over `crates/` finds no `ScreenSpaceAmbientOcclusion`, `TemporalAntiAliasing`, SMAA or FXAA; Bevy's `Bloom` appears only in the benchmark example `crates/opts-bevy/examples/lighting_bench.rs`. The `bloom` flag of `LightQuality` (`crates/opts-bevy/src/light_quality.rs`) and the setting `graphics.bloom` exist as plumbing, off in every ladder row, but nothing consumes the flag to add a camera component. M-0122 therefore plans new work only where it is really missing and leaves the rest to Phase 8.
3. **The milestone ID range starts at M-0120**, because M-0108..M-0119 (Phase 11) already exist.

## Reference image for the graphics target

The project lead supplied one reference screenshot, which is **not committed** (it shows OMSI content). Described in words, it is a daytime shot at a bus stop of a yellow Mercedes Citaro G articulated bus in Berlin livery with the front door open on a lit interior (seats, yellow handrails, the driver's cab). The sky is blue with soft, volumetric-looking cumulus. A strong, fairly low sun throws crisp long shadows from the kerb and stop furniture. Ground textures (cobbles, concrete kerb) are sharp at high resolution with strong anisotropic filtering. Paint is glossy with soft specular highlights; windscreen and side glass reflect the environment yet let the interior show through; rubber and trim are dark and matte. There is subtle ambient occlusion and contact darkening under the bus and inside. Trees are detailed with clean foliage edges, there is slight atmospheric haze in the distance, edges are cleanly anti-aliased, and the tonemapping is filmic with moderate contrast and only restrained bloom. [M-0122](M-0122.md) turns these characteristics into measurable A/B criteria.

## Already covered elsewhere

* **Filtering, mipmaps, cutout coverage, calibrated looks and shadow quality** stay in Phase 8 ([M-0057](../phase-8/M-0057.md), [M-0058](../phase-8/M-0058.md), [M-0059](../phase-8/M-0059.md), [M-0060](../phase-8/M-0060.md), [M-0091](../phase-8/M-0091.md)); M-0122 depends on them and absorbs none of their open items (the split is listed in M-0122).
* **Vehicle dynamics** (mass, axles, suspension, brakes) is [M-0073](../phase-10/M-0073.md); M-0124 only adds a dynamic mass input and does not own the solver.
* **Weather effects on grip** are [M-0117](../phase-11/M-0117.md).

---

*Proposed phase page. Execution waits for the project lead's approval; the `ROADMAP.md`, book summary and board entries follow the checkpoint.*
