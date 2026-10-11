# Bevy 0.20 Upgrade and DLSS — Strategy

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Plan (nothing implemented) |
| **Date** | 2026-10-11 |
| **Milestones** | [M-0047](../roadmap/phase-5/M-0047.md) (Bevy 0.20), [M-0048](../roadmap/phase-5/M-0048.md) (DLSS) |
| **Sources** | [Bevy 0.20 release post](https://bevy.org/news/bevy-0-20/) (2026-10-08), [0.19 → 0.20 migration guide](https://bevy.org/learn/migration-guides/0-19-to-0-20/), [bevyengine/dlss_wgpu](https://github.com/bevyengine/dlss_wgpu), [Bevy 0.17 post](https://bevy.org/news/bevy-0-17/) (DLSS introduction) |

## 1. What changes for OPTS in 0.20

Read from the migration guide (the last ~10 k characters of the guide were not read; re-check them when the branch is cut).

| Area | Change | OPTS impact |
| :--- | :--- | :--- |
| **Shaders** | naga_oil preprocessor removed; shaders are WESL (`.wesl`); `#import` → `import …;`; `#ifdef` → `@if(NAME)`; `#{MATERIAL_BIND_GROUP}` → `constants::MATERIAL_BIND_GROUP`; module paths moved (`bevy_pbr::mesh_view_bindings` → `bevy_pbr::render::mesh_view_bindings`); `#define_import_path` removed. Plain WGSL without directives still works. | One shader: `crates/opts-bevy/src/shaders/ground_layers.wgsl` (60 lines, 3 imports, 7 `#{MATERIAL_BIND_GROUP}`, 2 `#ifdef PREPASS_PIPELINE`). **Small, but it is the only thing that fails to compile.** |
| **Materials** | `UnpreparedBindGroup` → `BindGroupBuilder`; `AsBindGroup::unprepared_bind_group` → `build_bind_group`; OIT opt-out `enable_oit`. | `GroundMaterial` (`ground_material.rs`) implements `AsBindGroup`; rename. |
| **UI** | `Node` requires `EmSize`; `Val::resolve*`, `UiTransform::compute_affine` take `em_size`/`rem_size`; `ui::widgets::Button` and `Interaction` **deprecated**; UI rendering is retained; `TextFont::default()` is `FontSize::Rem(1.)`; `CalculatedClip` is an enum. | The whole menu stack uses `Button`/`Interaction` (`opts-gui` M3 components, `ButtonInteractionFilter`). Deprecated, not removed: keep compiling with warnings allowed for the module, then migrate to the pointer-event based widgets in a follow-up. Our own scroll hit-test uses `ComputedNode::contains_point` and `UiGlobalTransform` — re-check signatures. |
| **Observers / pointer** | `On<E, B>` loses `B`; `Pointer<Press>` → `PointerPress`. | Few observers (screenshot, hot reload); mechanical. |
| **Camera/render** | `ScreenSpaceTransmission` must be added explicitly; `Tonemapping`/`DebandDither` moved to `bevy_render::view`; `Tonemapping::None` is a pure passthrough, `Camera3d` enables `DebandDither` by default; `DepthAttachment` → `DepthStencilViewAttachment`. | Camera setup in `opts-bevy`/`opts-game`: pick `Tonemapping::Linear` to keep the look, decide on dither. |
| **Assets** | `AssetId::invalid()` deprecated; `ShaderBuffer` API; `compressed_image_saver` now BCn/ASTC; JPEG processed by default. | Grep `AssetId::invalid`. Texture compression is an opportunity (see §4). |
| **bevy_remote** | reflected type paths of `Tonemapping`, `DebandDither` changed. | Not used yet (evaluation open in M-0044). |
| **Reflection** | `FromType` → `CreateTypeData`, `ReflectFromPtr` renames. | Only if we implement custom reflection (we do not). |

Not applicable: BSN scene syntax changes (we spawn with required components), Solari (path tracing, needs hardware RT; not a target), sprites.

## 1a. What the upgrade actually needed (2026-10-11, branch `bevy-0.20`)
Result of the first pass; details in [M-0047](../roadmap/phase-5/M-0047.md). Compared with the estimate below: the shader port was two lines of API (`alpha_discard` arguments) beyond the syntax; the surprise was `Button` (deprecated alias of a private struct → cannot be spawned; solved by a local marker with the same requirements) and two toolchain/dependency pins (Rust ≥ 1.97.1, `glam` 0.33). `GroundMaterial`, observers, cameras, tonemapping and assets compiled unchanged.

## 2. Upgrade strategy (M-0047)

1. **Branch `bevy-0.20`** from `master`; bump `bevy = "0.20"`; keep the pin policy of M-0044 (exact minor, patch via `cargo update -p bevy`).
2. **Compile-fix order** (each step ends with `cargo check --workspace --all-targets`): (a) Cargo features (`shader_format_wesl` and GLSL are gone); (b) render types (`BindGroupBuilder`, depth attachment names); (c) the ground shader → `ground_layers.wesl`; (d) UI (`EmSize` is required component — `Node::default()` spawns still work; fix explicit `resolve` calls); (e) observers and pointer events; (f) leftovers.
3. **Ground shader port** (the only real risk): rename to `.wesl`; `import bevy_pbr::{…}` at the top; `@if(PREPASS_PIPELINE)` replaces both `#ifdef` blocks; `@group(constants::MATERIAL_BIND_GROUP)`. Verify visually against the reference screenshots of the painted-ground tiles (Bad Hügelsdorf (6,16), Ruhrau V2 (14,1)) **and** in the prepass (shadow cascades: the second `#ifdef` guards the prepass entry point).
4. **Verification gate** (same as M-0044): `cargo fmt`, clippy `-D warnings`, `cargo test --workspace`, `scripts/check_links.py`, then the screenshot set from `scratchpad/shots.sh` (Dinopark, NG272 external, Ruhrau parked cars, menu screens with the control protocol `input.mouse.click`).
5. **Deprecation debt**: do not migrate `Button`/`Interaction` during the upgrade (it changes behaviour of every menu). Track it as a separate task after 0.20 is green: replace `Interaction` change detection by the new pointer events in `opts-gui::interaction`.
6. **Do first on a throw-away basis**: `cargo update` can pull incompatible `wgpu`/`naga` versions; the lock file is part of the migration commit.

Effort estimate: small compared with 0.14 → 0.19 (one shader, one material, UI deprecation warnings). Risk is concentrated in the shader port and in UI behaviour changes caused by retained rendering (check clipping of the scroll lists and the clock picker, both rely on `Overflow::clip_y` and `UiTransform`).

## 3. DLSS (M-0048)

### What Bevy offers
Since 0.17 Bevy has an optional DLSS integration (`dlss` Cargo feature of `bevy_anti_alias`, backed by the `dlss_wgpu` crate): Super Resolution (upscaling) and, for Solari, Ray Reconstruction (0.20 supports DLSS-RR 4.5). Constraints from the sources: NVIDIA **RTX** GPU, **Vulkan backend** (Windows and Linux; no DX12/Metal/web), the **DLSS SDK cannot be redistributed** — it is downloaded by the developer, license accepted, `DLSS_SDK` and `VULKAN_SDK` set at build time, clang required; on Windows `nvngx_dlss.dll` ships next to the executable. Bevy 0.19 → 0.20 users must refresh the SDK (compile errors otherwise). Exact SDK/driver minimums were not found in the sources — read the `dlss_wgpu` README when starting.

### Target machine
The development machine has a GeForce RTX 3060 (driver 615.71.09) running Linux → supported by DLSS Super Resolution (RTX 20 series and newer); Frame Generation (RTX 40+) is out of scope.

### Integration design
* **Optional, never required.** A Cargo feature `dlss` on `opts-game`/`opts-bevy` (default off) forwards to `bevy/dlss`. CI and users without the SDK build exactly as today. The OMSI path dialog pattern applies: no hard failure, a clear message (`WARN`) when the feature is on but the GPU/driver lacks support (`DlssSupported` resource / camera component is absent).
* **Settings.** `GameSettings` gets `upscaler: Off | Dlss(quality)` (Performance / Balanced / Quality / DLAA) shown only when `DlssSupported` exists; the choice is saved in `settings.db` like `omsi_path`. The control protocol gets `render.set upscaler=…` so screenshots can compare modes.
* **Camera requirements.** DLSS needs motion vectors and depth prepass on the 3D camera, TAA/MSAA disabled, a `Camera3d` with HDR, temporal jitter provided by Bevy's `TemporalJitter`. Our custom `GroundMaterial` and the sky dome must write motion vectors/prepass correctly (the ground shader already has a prepass path; **vertex-animated or script-driven meshes** — vehicle animation, text textures — need previous-frame transforms, which Bevy tracks per mesh; verify bus doors/wheels do not ghost).
* **Alpha-tested vegetation** (trees: billboards with alpha masks) is the typical DLSS ghosting source; test on Bad Hügelsdorf and Ruhrau.
* **Fallback path.** Without DLSS: existing MSAA/none; later optionally Bevy's FSR-style or TAA. The settings enum is written so further upscalers slot in.
* **Packaging.** Linux build: SDK libraries are loaded at runtime from the SDK directory; document `DLSS_SDK`/runtime library path in `docs/src/guide/`; Windows build copies `nvngx_dlss.dll`. The license forbids committing the SDK: add `.gitignore`/CI checks that nothing under `DLSS_SDK` is tracked (same rule as OMSI assets).

### Prerequisites and ordering
1. M-0047 green on 0.20 (DLSS-RR 4.5 / newer SDK are tied to 0.20; start DLSS work there, not on 0.19).
2. Prove the toolchain first with Bevy's own `anti_aliasing` example (`cargo run --example anti_aliasing --features dlss --release`) on the RTX 3060 before touching OPTS code. If it fails, nothing else is worth doing: record the failure (driver, SDK version) in this page.
3. Add the feature + camera component behind `cfg(feature = "dlss")`; screenshot A/B (off, Quality, Performance) at the same camera pose with the control protocol; measure frame time with the F10 overlay (fps) at 1080p/1440p in Ruhrau V2 (heaviest map: 2.7 M scene vertices).
4. Settings UI toggle; persist; document.

### Acceptance
* Builds with and without `--features dlss`; CI builds without.
* On the RTX 3060: Quality mode ≥ 1.3× the fps of native 1440p on Ruhrau V2 with no visible ghosting on trees, wipers/doors and the clock/text textures; screenshots archived in `meta/`.
* No hard dependency on NVIDIA anywhere.

## 4. Side opportunities in 0.20
* **BCn/ASTC texture compression with generated mipmaps** (`compressed_image_saver`): OMSI `.dds` textures are loaded and decoded on the CPU today; an asset-processing step could cache compressed GPU textures (memory and load-time win for 2–3 M-vertex maps). Evaluate separately (M-0046 world loader extraction fits).
* Faster bulk despawn: relevant to tile streaming (despawning whole tiles).
* `Tonemapping::Linear` + debanding: re-tune the sky/exposure once during the upgrade rather than later.

## 5. Risks
| Risk | Mitigation |
| :--- | :--- |
| WESL port changes ground shading subtly | Reference screenshots before/after; keep the old shader in the 0.19 branch. |
| Retained UI breaks scroll/clip behaviour | Run the UI input scenarios (map list scroll, back buttons, clock picker) with the control protocol after the upgrade. |
| DLSS SDK/driver mismatch on Linux | Prove with Bevy's own example first; keep the feature off by default. |
| Ghosting from script-animated meshes | Per-object motion vectors; fallback: exclude those meshes from temporal accumulation via material flag if Bevy exposes one, otherwise lower DLSS preset. |
| Upstream is 3 days old (0.20.0, 2026-10-08): early patch releases | Start the branch now; merge only after 0.20.1 or when the verification gate is green. |
