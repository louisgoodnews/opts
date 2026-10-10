# 🚌 OPTS (Open Public Transport Simulator) Roadmap

> 📌 This file is canonical. [`docs/src/roadmap/README.md`](docs/src/roadmap/README.md) is the book's generated copy (same content, links rewritten). Per-phase folders with milestones: [`docs/src/roadmap/`](docs/src/roadmap/).

**Vision:** A free, open-source public transport simulator built in Rust and Bevy, capable of natively loading and simulating OMSI 2 assets.

**Core Philosophy:** 📖 *Documentation-Driven Development.* We write the specifications, format definitions, and architecture docs before writing the implementation code. The documentation is the source of truth.

---

## 🧭 The Phases

| Phase | Goal | Status | Folder |
| :--- | :--- | :--- | :--- |
| 🏗️ **Phase 0** — Foundation & Documentation Infrastructure | skeleton, workspace, governance, docs pipeline | 🟡 nearly complete (hosted Pages deployment unverified) | [`phase-0/`](docs/src/roadmap/phase-0/README.md) |
| 📝 **Phase 1** — OMSI Format Specification | document every OMSI 2 format before writing parsers | 🟢 specs drafted & mutation tested; review ongoing | [`phase-1/`](docs/src/roadmap/phase-1/README.md) |
| ⚙️ **Phase 2** — `opts-core` Implementation | parsers and data model | 🟢 complete | [`phase-2/`](docs/src/roadmap/phase-2/README.md) |
| 🎨 **Phase 3** — `opts-bevy` Integration | loaders, coordinates, rendering, GUI foundation | 🟢 complete | [`phase-3/`](docs/src/roadmap/phase-3/README.md) |
| 🚌 **Phase 4** — `opts-sim` & `opts-game` | simulation and the game binary | 🟡 in progress — M-0022 and M-0023 complete | [`phase-4/`](docs/src/roadmap/phase-4/README.md) |
| ⚡ **Phase 5** — Tooling, Performance & Optimization | benchmarks, LOD, culling, profiling, CLI inspection | ⏳ planned | [`phase-5/`](docs/src/roadmap/phase-5/README.md) |
| 🌐 **Phase 6** — Modding Ecosystem & Community Content | modding guide, packaging tools, repaint system | ⏳ planned | [`phase-6/`](docs/src/roadmap/phase-6/README.md) |
| 🚀 **Phase 7** — Release & Cross-Platform Distribution | CI matrix, packaging, v0.1.0-alpha release | ⏳ planned | [`phase-7/`](docs/src/roadmap/phase-7/README.md) |

Each phase folder holds a README with that phase's checklist (mirrored from the sections below, which are canonical) and its milestone files.

---

## 🏗️ Phase 0: Foundation & Documentation Infrastructure
*Goal: Establish the project skeleton, workspace, and documentation pipeline.*

*Status 2026-10-04: all items done except the hosted GitHub Pages deployment, which cannot be verified from the working tree. ADR-0002 still carries a `YYYY-MM-DD` placeholder date.*

- [x] **📦 Workspace Setup** *(`cargo check --workspace --offline` passes, verified 2026-10-04)*
  - [x] Initialize Cargo workspace with `opts-core`, `opts-bevy`, `opts-sim`, `opts-tools`, `opts-utils`, and `opts-game`. *(Annotation 2026-10-10: `opts-utils` and other placeholder crates were removed in `07de1b0`; the workspace now has bevy, catalogue, cli, core, game, gui, logging, sim, tools.)*
  - [x] Configure root `Cargo.toml` with shared dependencies and build profiles.
- [ ] **📚 Documentation Pipeline**
  - [x] Set up `mdBook` in the `docs/` directory. *(`docs/book.toml` was a 0-byte file until M-0007; `mdbook build` has not been run locally — the tool is not installed)*
  - [x] Configure `SUMMARY.md` to outline the entire project.
  - [x] Set up GitHub Actions to build and deploy `mdBook` to GitHub Pages on push. *(workflow `deploy-docs.yml` present; it triggers on `main` while the local branch is `master`; first run unverified)*
  - [ ] Configure GitHub Pages and verify the live site updates (needs the hosted repository).
- [x] **⚖️ Project Governance**
  - [x] Add `LICENSE` (MIT/Apache-2.0 dual license).
  - [x] Add `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md`.
  - [x] Create `docs/src/adr/` for Architecture Decision Records.
  - [x] Write ADR-0001: "Why Bevy and Rust?"
  - [x] Write ADR-0002: "Clean-Room Reverse Engineering Policy" (Crucial for legal safety).
- [x] **📖 Core Documentation**
  - [x] Write `docs/src/introduction.md` and `docs/src/vision.md`.
  - [X] Write `docs/src/glossary.md` (OMSI terms, Bevy terms, transit terminology).

---

## 📝 Phase 1: OMSI Format Specification (The "Docs-First" Phase) (Current)
*Goal: Completely document the OMSI 2 file formats and asset pipeline before writing a single Rust parser. This phase produces the specification that `opts-core` will implement.*

*Status 2026-10-04: first drafts of every planned format exist. All are 🟡 **Draft** — structure verified against the full installed corpus by clean-room analysis (hex dumps, corpus-wide structural walks, statistics; no executables inspected), but many field **meanings** still need mutation tests in OMSI and a maintainer review before they can be marked 🟢.*

- [x] **📄 Text Config Formats (Parser: `nom`)** — drafted
  - [x] [`formats/text/sco.md`](docs/src/formats/text/sco.md), [`sli.md`](docs/src/formats/text/sli.md) + [corpus audit](docs/src/formats/text/sco-sli-audit.md) (47 undocumented `.sco` keywords, misspelled aliases, BOM/encoding findings).
  - [x] [`formats/text/vehicles/bus.md`](docs/src/formats/text/vehicles/bus.md), [`cfg.md`](docs/src/formats/text/cfg.md), model/paths/passengercabin/sound configs.
  - [x] [`formats/text/vehicles/hof.md`](docs/src/formats/text/vehicles/hof.md), [`osc.md`](docs/src/formats/text/vehicles/osc.md), [`varnamelist.md`](docs/src/formats/text/vehicles/varnamelist.md), [`const.md`](docs/src/formats/text/vehicles/const.md) + [vehicle-side corpus audit](docs/src/formats/text/vehicles/vehicle-corpus-audit.md) (UTF-16 hofs, undocumented `$` functions, duplicate varlist names).
  - [x] [`formats/text/org.md`](docs/src/formats/text/org.md): number/registration lists.
  - [x] [`formats/text/misc-types.md`](docs/src/formats/text/misc-types.md): `.ovh`, `.hum`, `.cti`, `.oft`, `.otp`, `.owt`, `.oop`, `.olf`, `.ocu`, `.zug`, `.amf`, `.rdy`, backups.
  - [x] [`formats/text/texture-cfg.md`](docs/src/formats/text/texture-cfg.md): `*.bmp.cfg` surface/moisture/puddle properties.
- [x] **🗺️ Map & Scenario Files** — drafted
  - [x] [`formats/map/global-cfg.md`](docs/src/formats/map/global-cfg.md): `global.cfg`, `laststn.osn`, `.dsc` (UTF-16LE pitfall).
  - [x] [`formats/map/tiles.md`](docs/src/formats/map/tiles.md): `tile_<x>_<y>.map` (object/spline records), `.terrain` height grid, `.water`, `.LM.bmp`, `.prt`, `.hole`, `.surf`.
  - [x] [`formats/map/lists.md`](docs/src/formats/map/lists.md): AI lists, unscheduled traffic, parked/human/driver/registration lists, holidays, time zone, signal routes.
  - [x] [`formats/map/timetables.md`](docs/src/formats/map/timetables.md): `Chrono.cfg`, `.ttl`, `.ttp`, `.ttr`, bus stops, station links.
- [x] **🔢 Binary Formats (Parser: `binrw`)** — 🟢 Specified
  - [x] [`formats/binary/o3d.md`](docs/src/formats/binary/o3d.md) + [chunk reference](docs/src/formats/binary/o3d-chunks.md): layout confirmed on 165,695 of 165,750 non-empty files; field semantics and coordinate space confirmed by mutation testing [M]. 🟢 Specified.
  - [x] Annotated hex breakdown of a simple `.o3d` (self-authored sample in `o3d.md`).
  - [x] [`formats/binary/x.md`](docs/src/formats/binary/x.md): legacy DirectX `.x` (text and binary variants); frame matrix, material ref, and quad triangulation confirmed by mutation testing [M]. 🟢 Specified.
- [x] **🔊 Format inventory & audio analysis** — [`formats/inventory.md`](docs/src/formats/inventory.md) (619,821 files, 122 extensions, coverage per extension), full [`.wav` analysis`](docs/src/formats/wav.md) (42,375 files: variants, chunks, robustness findings, config references) and nine small stubs (`.dat`, `.xsli`, `.itx`, `.ipr`, `.kyb`, `.kennz`, `.html`, `.off`, `.sichtbar`/`.unsichtbar`) (2026-10-04)
- [x] **🌐 Community cross-check** — [`formats/community-crosscheck.md`](docs/src/formats/community-crosscheck.md): reconciles the drafts with the public OMSI community wiki (facts only, CC BY-SA attribution); resolves several open questions (`$msg`, `[rendertype]`, surface types 0–16, `.surf`) and records conflicts (`[matl_alpha]` value distribution differs from `sco.md`, UTF-16 hofs, keyword case).
- [x] **🌐 Wiki Cross-Check & Spec Overhaul** — milestone [M-0006](docs/src/roadmap/phase-1/M-0006.md) (complete 2026-10-04; 31 pages, 19 specs updated)
- [x] **🕳️ Documentation Gap Closure** — from [`formats/gap-analysis.md`](docs/src/formats/gap-analysis.md) (all complete 2026-10-04)
  - [x] [M-0008](docs/src/roadmap/phase-1/M-0008.md) Script Reference — predefined variables and callbacks (`program/`), script sets, scenery scripts, manual review
  - [x] [M-0009](docs/src/roadmap/phase-1/M-0009.md) Repaint System — `.cti` / `[CTC]` chain
  - [x] [M-0010](docs/src/roadmap/phase-1/M-0010.md) Engine & Environment Configs — `envir.cfg`, weather, scenarios, inputs, `.opl`
  - [x] [M-0011](docs/src/roadmap/phase-1/M-0011.md) Passenger Content — ticket packs, money, fonts
  - [x] [M-0012](docs/src/roadmap/phase-1/M-0012.md) Chrono and Map Variants
- [x] **🧪 Mutation-Test Round (needs OMSI)** — binary mesh round complete (2026-10-05); map-side round complete (2026-10-08)
  - [x] Run `CHECKLIST.md` (now also covers the axis prediction in `o3d.md` Addendum 2: Y up, Z longitudinal); folded results into `o3d.md` (Q-004 material groups, Q-005 axes/winding/UV, Q-006 matrix, Q-001–003 header) and `x.md` (Q-X01 frame matrix, Q-X02, Q-X07, Q-X08). Both promoted to 🟢 Specified.
  - [x] Design and execute Mutation Kit 2 for the map & scenery side: `[spline]` field layout and axis order (Q-M02 resolved), `[object]` pitch/roll orientation (Q-M03 resolved), texture fallback chain (`.bmp` $\to$ `.dds` exclusive, Q-TX01 resolved), tile dimensions (Q-M01 confirmed), and alpha blending modes (Q-TX03 confirmed).
- [x] **🖼️ Texture Formats** — drafted
  - [x] [`formats/textures.md`](docs/src/formats/textures.md): `.dds` / `.bmp` / `.tga` / `.png` / `.jpg` usage over 202,000 files (A8 ground masks, extension-vs-content mismatches, `.dds` name fallback, RLE TGA, V4/V5 BMP headers). Alpha semantics (Q-TX03) and name resolution (Q-TX01) verified by Mutation Kit 2.
- [x] **📐 Coordinate Systems & Math**
  - [x] `docs/src/architecture/coordinates.md`: OMSI's axes/handedness mapped to Bevy (completed 2026-10-05, Q-M01 blocker noted).
- [ ] **✅ Review & Promotion**
  - [ ] Maintainer review of all 🟡 drafts; promote to 🟢 once the high-priority open questions of that spec are resolved.
  - [x] Add the missing index pages referenced by `SUMMARY.md` (`formats/overview.md`, `formats/text/README.md`, `formats/binary/README.md`) — done in M-0007.
  - [ ] Confirm the drafts' provenance sections against [ADR-0002](docs/src/adr/0002-clean-room-policy.md) (clean-room policy).
- [ ] **📚 Only shallowly covered:** `.osn` field meanings, the purpose of `.rra`/`.surf`/`.prt` caches, and vehicle `.dsc` fragments. Tool files (`.pdn`, `.sfk`, `.blend`, …) and `.dll` plug-ins are intentionally not specified.

---

## ⚙️ Phase 2: `opts-core` Implementation (The Parsers)
*Goal: Implement the Rust code that translates the specifications from Phase 1 into usable data structures.*

- [x] **💾 Binary Mesh Parsers** — [M-0013](docs/src/roadmap/phase-2/M-0013.md) & [M-0016](docs/src/roadmap/phase-2/M-0016.md)
  - [x] `.o3d` 3D mesh parser (`opts_core::formats::o3d`)
  - [x] `.x` legacy DirectX mesh parser (`opts_core::formats::x`) — [M-0016](docs/src/roadmap/phase-2/M-0016.md)
  - [x] Clean-room mutation fixtures integration tests (`t01`–`t07`, `t10`–`t12`)
- [x] **📝 Vehicle & Operational Data Parsers** — [M-0014](docs/src/roadmap/phase-2/M-0014.md)
  - [x] `.bus` vehicle manifest parser (`opts_core::formats::vehicles::bus`)
  - [x] `paths.cfg` passenger paths parser (`opts_core::formats::vehicles::paths`)
  - [x] `.org` registration & fleet lists parser (`opts_core::formats::org`)
  - [x] `Money/<pack>/*.cfg` currency denominations parser (`opts_core::formats::money`)
- [x] **📐 Model Configuration Parser** — [M-0017](docs/src/roadmap/phase-2/M-0017.md)
  - [x] `model.cfg` vehicle rendering manifest parser (`opts_core::formats::vehicles::model`)
  - [x] LOD tiers, viewpoint camera masks, interactive click events, variable-driven animations, and material modifiers
- [x] **🗺️ World & Scenery Parsers** — [M-0015](docs/src/roadmap/phase-2/M-0015.md)
  - [x] Scenery object parser (`opts_core::formats::scenery::sco`)
  - [x] Spline profile parser (`opts_core::formats::scenery::sli`)
  - [x] Map manifest parser (`opts_core::formats::map::global`)
  - [x] Tile layout parser (`opts_core::formats::map::tile`)
  - [x] Binary height grid (`.terrain`) and water level (`.water`) parsers (`opts_core::formats::map::{terrain, water}`)


---

## 🎨 Phase 3: `opts-bevy` Integration (The Visuals)
*Goal: Bridge the gap between `opts-core` data and Bevy's ECS and rendering pipeline.*

- [x] **📥 Bevy Asset Loaders & Mesh Bridge** — [M-0018](docs/src/roadmap/phase-3/M-0018.md)
  - [x] Implement custom `AssetLoader` for `.o3d` files.
  - [x] Implement custom `AssetLoader` for `.x` files (discovered in M-0016).
  - [x] Implement custom `AssetLoader` for `.sco` and `.sli` files (`OptsSceneryPlugin`).
  - [x] Register the loaders with the `AssetServer` via `OptsMeshPlugin` and `OptsSceneryPlugin`.
  - [x] Mesh Conversion Bridge: convert `opts_core::formats::o3d::O3d` and `XMesh` to Bevy `Mesh` primitives with left-to-right handed coordinate conversion ($Z \to -Z$, winding inversion).
- [x] **🔄 Coordinate Conversion Systems** — [M-0020](docs/src/roadmap/phase-3/M-0020.md)
  - [x] Write systems to convert OMSI Z-up coordinates to Bevy Y-up coordinates.
  - [x] Implement camera preset projection bridge for vehicle view definitions (`[add_camera_driver]`, etc.) (`camera_preset_to_bevy_transform`) (M-0020)
- [x] **🌟 Rendering OMSI Assets & Material Bridge** — [M-0019](docs/src/roadmap/phase-3/M-0019.md)
  - [x] Implement texture format sniffing and decoding for `.dds`, `.bmp`, `.tga`, `.png`, `.jpg`.
  - [x] Implement custom `AssetLoader` for `.dds` textures in `opts-bevy`.
  - [x] Create Bevy `StandardMaterial` assets from parsed OMSI `Material` and `XMaterial` properties.
  - [x] Spawn parsed `.sco` objects as Bevy entities with `Mesh` and `Transform` components (`spawn_scenery_object`).
- [x] **🎨 Material Design 3 GUI Widget Library (`opts-gui`)** — [M-0021](docs/src/roadmap/phase-3/M-0021.md)
  - [x] Implement token architecture (`theme/`, `tokens/`) with WCAG 2.1 AA luminance contrast verification.
  - [x] Implement layout builders: `StackBuilder`, `RowBuilder`, `GridBuilder`.
  - [x] Implement Tier 1 components: `Button`, `Label`, `Icon`, `Card`, `Divider`, `TextField`, `Checkbox`, `Switch`, `Radio`.
  - [x] Implement advanced components: `Combobox`, `Notebook`, `Toast`, `Toplevel`, `Treeview`, `DateSelect`, `DateTimeSelect`.
  - [x] Implement interaction & focus navigation system (`Focusable`, `FocusEvent`, `FocusPlugin`).
  - [x] Build comprehensive interactive gallery application (`examples/gallery.rs`).
- [x] **🎓 Tutorials & Examples**
  - [x] Write `docs/src/tutorials/first-sco.md`: "Loading a single OMSI scenery object in Bevy."
  - [x] Write `docs/src/tutorials/first-bus.md`: "Loading a full OMSI bus model with textures."
  - [x] Create runnable `examples/` in the `opts-bevy` crate matching the tutorials (`load_sco.rs`, `load_bus.rs`).

---

## 🚌 Phase 4: `opts-sim` & `opts-game` (The Simulation)
*Goal: Build the actual public transport simulation logic and the main executable.*

- [ ] **🗺️ World & Map Loading** — [M-0024](docs/src/roadmap/phase-4/M-0024.md) & [M-0025](docs/src/roadmap/phase-4/M-0025.md)
  - [ ] Document the architecture of the map streaming system (`docs/src/architecture/map-streaming.md`).
  - [ ] Implement tile grid paging and async chunk streaming coordinator in `opts-game` and `opts-bevy` (M-0024 — traces to F-018).
  - [ ] Implement terrain elevation mesh generation and water plane systems from `.map.terrain` and `.map.water` (M-0025 — traces to F-018).
- [ ] **🛞 Vehicle Physics & Control** — [M-0022](docs/src/roadmap/phase-4/M-0022.md) & [M-0023](docs/src/roadmap/phase-4/M-0023.md)
  - [x] Document the vehicle physics model in `docs/src/simulation/vehicle_physics.md`.
  - [x] Implement kinematic/dynamic vehicle movement in `opts-sim`.
  - [x] Implement Bevy ECS bridge plugin in `opts-bevy` connecting `VehicleSimulation` state to entity `Transform` and wheel mesh rotation. (M-0023)
  - [~] Script VM core implemented (compiles all 6,062 corpus scripts); vehicle wiring open — OMSI Script Stack VM (`.osc`) and dynamic variable table system in `opts-sim` ([M-0026](docs/src/roadmap/phase-4/M-0026.md) — traces to F-008, F-019).
  - [ ] Map OMSI script variables (`.osc`) to Bevy systems for lights, doors, and indicators.
  - [ ] Implement AI vehicle spline path following and traffic controller in `opts-sim` and `opts-bevy`.
  - [ ] Implement vehicle and environmental spatial audio simulation bridge in `opts-bevy` mapping engine RPM and pneumatic pressure to spatial audio ([M-0027](docs/src/roadmap/phase-4/M-0027.md) — traces to F-009, F-020).
- [ ] **🧑‍🤝‍🧑 Passenger & Routing System**
  - [ ] Document the passenger simulation model in `docs/src/simulation/passengers.md`.
  - [ ] Implement bus stop logic and passenger boarding/alighting.
  - [ ] Implement a basic routing algorithm (e.g., RAPTOR or Dijkstra via `petgraph`).
- [ ] **🎮 The Game Binary**
  - [ ] Assemble `opts-game/src/main.rs` by adding the `opts-bevy` and `opts-sim` plugins to the Bevy `App`.
  - [ ] Implement the main menu, loading screens, and basic HUD.
- [x] **📦 Delivered outside milestones (reconciled 2026-10-10, [M-0037](docs/src/roadmap/phase-4/M-0037.md))** — shipped after M-0023 without their own milestone; no earlier status changed.
  - [x] Workspace modularisation, `opts-catalogue` (map/vehicle catalogue) and `opts-logging` — `07de1b0`, [report](docs/src/meta/modularization-2026-10-09.md).
  - [x] Real OMSI asset loading, tile streaming (`stream_world_tiles`), terrain mesh, data-driven sky, layered ground shader, spline attachments, camera system, humans/ego perspective — `69e94b2`, `780cae4` (crates `opts-bevy`, `opts-game`).
  - [x] Map, scenery, humans and envir parsers; `.o3d`/`.x` robustness; scrambled-mesh decoder (partial, see [M-0038](docs/src/roadmap/phase-4/M-0038.md)) — `438d622` (`opts-core`).
  - [x] F1 reporting and GUI modal/text area/fonts — `f5dfd38`, [report](docs/src/meta/f1-reporting-2026-10-10.md).
  - [x] Control protocol and hot reload — M-0043 (see Phase 5); spline road/terrain fix — `2816806`, [bug report](docs/src/meta/bugs/f17228d9-3bc0-4830-8f25-3bc9465f63d2_2026-10-10_16-44-31-spline-winding.md).
- [ ] **🧾 Audit-Driven Work (audit 5037f89a, 2026-10-10)**
  - [x] Reconcile roadmap, phase pages and board with work shipped since M-0023 — [M-0037](docs/src/roadmap/phase-4/M-0037.md) (F-019, F-023).
  - [ ] Complete scrambled-mesh decoding: async decoding, deterministic parameters, cache policy, generator research — [M-0038](docs/src/roadmap/phase-4/M-0038.md) (F-001, F-003, F-014, F-017).
  - [x] Render `[texttexture]` text on signs, stops and displays — [M-0039](docs/src/roadmap/phase-4/M-0039.md) (F-002).
  - [ ] Script-driven animation, lights and doors — [M-0045](docs/src/roadmap/phase-4/M-0045.md).
  - [ ] World loader extraction — [M-0046](docs/src/roadmap/phase-4/M-0046.md).
  - [ ] Extract the world loader into `opts-bevy` and decouple `opts-bevy` from `opts-gui` (appended to [M-0024](docs/src/roadmap/phase-4/M-0024.md); F-015, F-016).
  - [ ] Verify spline-attachment repeaters, fix floating objects, read weather/fog (appended to [M-0025](docs/src/roadmap/phase-4/M-0025.md); F-004, F-007, F-008).

---

## ⚡ Phase 5: Tooling, Performance & Optimization
*Goal: Profile, benchmark, and optimize engine throughput, and provide developer CLI inspection utilities.*

- [ ] **⚡ Performance & Optimization** — [M-0028](docs/src/roadmap/phase-5/README.md) & [M-0029](docs/src/roadmap/phase-5/README.md)
  - [ ] Implement Criterion microbenchmark suite in `opts-benchmark` for format parsers and vehicle physics step (M-0028 — traces to F-021).
  - [ ] Document LOD (Level of Detail), frustum culling, and occlusion strategies in `docs/src/architecture/lod-culling.md`.
  - [ ] Implement distance-based dynamic LOD mesh switching and shadow culling in `opts-bevy` (M-0029).
  - [ ] Implement async background asset streaming to prevent frame drops during tile boundary crossings.
- [ ] **🧰 Developer CLI Tooling** — [M-0030](docs/src/roadmap/phase-5/README.md)
  - [ ] Document CLI command specifications in `docs/src/reference/opts-tools.md`.
  - [ ] Implement `opts inspect <file>` to validate OMSI asset syntax and detect corrupt files (M-0030).
  - [ ] Implement `opts convert <file> --output <gltf>` to export legacy meshes to standard formats.
- [x] **🎛️ Remote Control & Live Editing** — [M-0043](docs/src/roadmap/phase-5/M-0043.md)
  - [x] Design documents and ADRs: [Control Protocol](docs/src/architecture/control-protocol.md), [Hot Reload](docs/src/architecture/hot-reload.md), ADR-0006, ADR-0007.
  - [x] `opts-cli` client, `opts-bevy::control` server and `opts-bevy::hot_reload` ([final report](docs/src/meta/control-protocol-2026-10-10.md)).
- [ ] **⬆️ Engine Upgrade** — [M-0044](docs/src/roadmap/phase-5/M-0044.md)
  - [ ] Migrate Bevy 0.14 → current in verified single-version steps.
- [ ] **🧹 Audit-Driven Hardening (audit 5037f89a, 2026-10-10)**
  - [ ] Code-quality debt: size limits, panics in library code, stubs, rustdoc — [M-0040](docs/src/roadmap/phase-5/M-0040.md) (F-005, F-006, F-009–F-012).
  - [ ] Documentation repair and developer switch reference — [M-0041](docs/src/roadmap/phase-5/M-0041.md) (F-013, F-018, F-020–F-023).
  - [ ] CI and toolchain hardening: link check, mdBook build, toolchain pin, Windows job — [M-0042](docs/src/roadmap/phase-5/M-0042.md) (F-024–F-026).

---

## 🌐 Phase 6: Modding Ecosystem & Community Content
*Goal: Empower content creators to build, package, validate, and share vehicles, scenery objects, maps, and repaints.*

- [ ] **📦 Modding Infrastructure** — [M-0031](docs/src/roadmap/phase-6/README.md) & [M-0032](docs/src/roadmap/phase-6/README.md)
  - [ ] Specify mod directory layout, virtual filesystem mounting, and load priority rules in `docs/src/architecture/modding-vfs.md` (M-0031).
  - [ ] Implement `.cti` repaint manifest parser and runtime livery material switcher in `opts-bevy` (M-0032).
- [ ] **📘 Creator Documentation & Tooling** — [M-0033](docs/src/roadmap/phase-6/README.md)
  - [ ] Author `docs/src/modding/overview.md` and `docs/src/modding/asset-pipeline.md`.
  - [ ] Author vehicle and map creation guides for content authors.
  - [ ] Implement mod package validation linter in `opts-tools` (M-0033).

---

## 🚀 Phase 7: Release & Cross-Platform Distribution
*Goal: Package, validate, and distribute production-ready binaries of OPTS across Linux and Windows platforms.*

- [ ] **⚙️ Release Engineering** — [M-0034](docs/src/roadmap/phase-7/README.md)
  - [ ] Configure multi-platform GitHub Actions release matrix for Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`) (M-0034 — traces to F-017).
  - [ ] Implement automated release workflow generating compressed binary packages with SHA-256 checksums.
- [ ] **📖 User Documentation & Settings UI** — [M-0035](docs/src/roadmap/phase-7/README.md)
  - [ ] Author end-user installation, controls, and troubleshooting manuals (`docs/src/user/`).
  - [ ] Implement in-game settings dialog in `opts-gui` for graphics preferences, audio volume, and input rebinding (M-0035).
- [ ] **🏁 Milestone Release** — [M-0036](docs/src/roadmap/phase-7/README.md)
  - [ ] Publish `v0.1.0-alpha` public release milestone on GitHub Releases (M-0036).

---

## 🧹 Deferred Cleanup

*Small items discovered during milestones and critical reviews; none warrants a full milestone.*

- [ ] Add a mutation test for `[matl_alpha]` modes 0–3 and `[light_enh_2]` field order to the kit (M-0006)
- [ ] Add a case-sensitivity probe (`[NIGHTMAPMODE]`, `$CUTEND`) to the kit (M-0006)
- [x] Create or retire the milestone IDs M-0001 to M-0005 referenced by `docs/dev/BOARD.md` but absent from `docs/src/roadmap/` (M-0006) — done in M-0007
- [x] Reconcile the BOARD's Ready/Backlog items with the specs already drafted (`.o3d`, `.x`, `.sco`, `.sli`, `.hof`, `.osc`, varlist, constfile, `org`, textures, map tiles) (M-0006) — done in M-0007
- [x] Fill the empty root `README.md` and the empty phase README stubs under `docs/src/roadmap/` (M-0006) — done in M-0007
- [x] Tick the Phase 0 items that exist on disk (ADR-0001/0002, GitHub workflows) or document why they stay open (M-0006) — done in M-0007
- [x] Rename `docs/src/roadmap/phase N/` folders to the `phase-N` form used by `PROMPTS.md` and `SUMMARY.md`, then fix the 48 broken `SUMMARY.md` links (M-0006) — done in M-0007
- [ ] Decide whether to obtain the unofficial SDK intersection manual the wiki cites (not installed; legal status unchecked) (M-0006)
- [x] Write the 36 chapters that `SUMMARY.md` lists as mdBook *draft chapters* — reconciled in freeze pass: 9 stubs created in Architecture/Contributing, 5 unwritten sections commented out (2026-10-06)
- [x] Set a real date in ADR-0002 (`2026-10-04`) (M-0007 / freeze pass)
- [x] Align the CI/Pages branch (`main`) with the repository's actual default branch (`master` locally) (F-014 — resolved 2026-10-08)
- [x] Remove or replace the 0-byte `docs/crates/book.toml` (populated in M-0007)
- [ ] Install `mdbook` and run `mdbook build docs` once to confirm the book builds with `create-missing = false` (M-0007)
- [ ] Replace bare `.unwrap()` with `?` in `parse_mesh` (`crates/opts-core/src/formats/x/parse.rs:388`) (F-001)
- [ ] Audit and prune premature stub crates from workspace `members` in root `Cargo.toml` (F-002)
- [ ] Provide `opts_bevy::testing::TestAppBuilder` helper for headless `FixedUpdate` time stepping (F-003)
- [ ] Decompose monolithic parser (1,061 lines) and 546-line `from_text` function in `crates/opts-core/src/formats/scenery/sco.rs` (F-004)
- [ ] Refactor monolithic parsing functions (>80 lines) in `bus.rs`, `sli.rs`, `global.rs`, `tile.rs`, `model/parse.rs`, `paths.rs` (F-005)
- [ ] Split `bus.rs` (583 lines) and `world_parse.rs` (689 lines) to comply with 500-line file limit (F-006)
- [ ] Consolidate entity spawning boilerplate between `spawn` and `spawn_child` in `opts-gui` widgets (F-007)
- [ ] Execute link normalization sweep across `docs/src/roadmap/README.md` and meta reports (F-011)
- [ ] Reconcile stale Open Questions in promoted specifications (`o3d.md`, `x.md`, `bus.md`) into Resolved Questions (F-012)
- [ ] Integrate automated markdown link checker and mdBook validation step into CI workflow (F-015)
- [ ] Create `rust-toolchain.toml` (pinned to stable) and `.editorconfig` in repository root (F-016)
- [ ] Add `windows-latest` to GitHub Actions CI test matrix for cross-platform validation (F-017)
- [ ] Control protocol follow-ups (M-0043): `entity.*`/`resource.*`/`asset.*` via reflection or `bevy_remote`, `debug.*`, `sim.set_time`, event streaming (`opts-cli watch`), vehicle-state snapshot on hot reload
