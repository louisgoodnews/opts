# 🚌 OPTS (Open Public Transport Simulator) Roadmap

> 📌 This page is the book's copy of the repository-root [`ROADMAP.md`](../../../ROADMAP.md), which is canonical. Edit `ROADMAP.md` and regenerate this page (`python3 scripts/gen_roadmap_copy.py`; links are rewritten relative to `docs/src/roadmap/`).

**Vision:** A free, open-source public transport simulator built in Rust and Bevy, capable of natively loading and simulating OMSI 2 assets.

**Core Philosophy:** 📖 *Documentation-Driven Development.* We write the specifications, format definitions, and architecture docs before writing the implementation code. The documentation is the source of truth.

---

## 🧭 The Phases

| Phase | Goal | Status | Folder |
| :--- | :--- | :--- | :--- |
| 🏗️ **Phase 0** — Foundation & Documentation Infrastructure | skeleton, workspace, governance, docs pipeline | 🟢 complete — repository on GitHub, Pages live (https://louisgoodnews.github.io/opts/, `deploy-docs` green since 2026-10-13) | [`phase-0/`](phase-0/README.md) |
| 📝 **Phase 1** — OMSI Format Specification | document every OMSI 2 format before writing parsers | 🟢 🟡 nearly complete — 11 of 12 milestones done (M-0004: GTFS notes open); 20 format pages 🟢 Specified, 23 🟡 Draft; maintainer review and ADR-0002 provenance check open | [`phase-1/`](phase-1/README.md) |
| ⚙️ **Phase 2** — `opts-core` Implementation | parsers and data model | 🟢 complete (code lives in `opts-omsi` since Phase 13; `opts-core` is a re-export facade) | [`phase-2/`](phase-2/README.md) |
| 🎨 **Phase 3** — `opts-bevy` Integration | loaders, coordinates, rendering, GUI foundation | 🟢 complete | [`phase-3/`](phase-3/README.md) |
| 🚌 **Phase 4** — `opts-sim` & `opts-game` | simulation and the game binary | 🟡 in progress — physics, world streaming (time-budgeted), text and script textures, script VM, script-driven animation/lights/night maps/triggers, vehicle controls with cold starts (all 948 buses reach drive readiness since 2026-10-25), AI traffic with signals and scheduled AI, walking pedestrians, articulated consists, sounds (engine, scenery, AI, player), passengers (first slice) and timetables running; dashboard displays, cash desk, AI scripts driving the vehicle, world-loader extraction open | [`phase-4/`](phase-4/README.md) |
| ⚡ **Phase 5** — Tooling, Performance & Optimization | benchmarks, LOD, culling, profiling, CLI inspection | 🟡 in progress — control protocol, hot reload, Bevy 0.19 and 0.20 upgrades, scene trace, settings protocol, instancing and reflections research done; scenery benchmark Phases A, B, D done; CI runs on GitHub and is fully green since 2026-10-25 (Linux, Windows, macOS, docs; `8893c9f5`, M-0042), Pages live; benchmarks (M-0028), tile LOD (M-0029) and DLSS open | [`phase-5/`](phase-5/README.md) |
| 🌐 **Phase 6** — Modding Ecosystem & Community Content | modding guide, packaging tools, repaint system | ⏳ planned (repaint selection and AI/parked repaints shipped early; editor mode and script crates specified) | [`phase-6/`](phase-6/README.md) |
| 🚀 **Phase 7** — Release & Cross-Platform Distribution | CI matrix, packaging, v0.1.0-alpha release | 🟡 started — first slices of key rebinding (M-0142) and mouse driving (M-0143) 2026-10-20; release engineering, user manual, gamepad/wheel rebinding open | [`phase-7/`](phase-7/README.md) |
| 🖼️ **Phase 8** — Visual Quality & Realism | texture filtering, mipmaps, foliage coverage, reality-calibrated colour, Vivid default look | 🟡 first stage reviewed 2026-10-25 (sampler and mip chains accepted, looks too weak); DXT chains with a derived cache done 2026-10-25 | [`phase-8/`](phase-8/README.md) |
| 🧭 **Phase 9** — User Interface & Experience | evidence-based UI: looks, feel, flow, expandability; app shell, settings system, HUD, accessibility | 🟡 design approved 2026-10-12; shell, wizard, settings, HUD, automatic accessibility part and M-0070/71/75/76/86/88/130 done; open: usability test, gamepad/wheel rebinding, HUD sign-off, language-pack trial | [`phase-9/`](phase-9/README.md) |
| 🧩 **Phase 10** — Sessions, World Completeness & Vehicle Dynamics | water, session vehicles and save/restore, mesh fidelity, per-vehicle mass/suspension/brakes on a world physics | 🟡 in progress — M-0068, M-0069, M-0073, M-0078, M-0106 done; M-0066 tasks done (awaiting the project lead's acceptance); M-0067, M-0077, M-0090 have open items; M-0079 and M-0138 deferred, M-0104 obsolete | [`phase-10/`](phase-10/README.md) |
| 🔭 **Phase 11** — Advanced Systems, Realism & Continuous Improvement | in-game map, QoL and accessibility settings, AI traffic depth, scheduled transport, weather effects, event-system research | 🟡 approved 2026-10-20, in progress — M-0108, M-0112 done; M-0109, M-0110, M-0111, M-0113 in progress; M-0119 awaiting review of its dry run; M-0114–M-0118 not started | [`phase-11/`](phase-11/README.md) |
| 🎨 **Phase 12** — Visual Fidelity & Advanced Physics | dirt and light maps, wetness and snow, graphics overhaul, fuel and battery, passenger load | 🟡 approved 2026-10-20 — M-0124 in progress (mass, axle loads, ride height, override; save field open); M-0120–M-0123 not started | [`phase-12/`](phase-12/README.md) |
| 🏗️ **Phase 13** — Crate Architecture: Legacy vs. Native Separation | `opts-omsi` (legacy parsers, leaf adapter), `opts-model` (native TOML), shared base crate; no engine crate depends on `opts-omsi` | ✅ delivered 2026-10-19 (no direct Cargo dependency on opts-omsi); consumer migration not started — engine crates still reach OMSI types through the opts-core facade (omsi_uses.py: bevy 130, sim 31, traffic 14, catalogue 5, unchanged since 2026-10-19) | [`phase-13/`](phase-13/README.md) |
| 🛠️ **Phase 14** — Vehicle Overrides & Visual Editor | `VehicleProfile` TOML patch layer (sound, script, constant, mesh swaps with validation) and an in-game visual mesh editor | 📝 proposed 2026-10-19, design first | [`phase-14/`](phase-14/README.md) |
| ♾️ **Phase 999** — Refactoring Taskforce (continuous) | a structural cleanup without behaviour change at the end of every development cycle and before every versioned release | ♾️ continuous; set-up and cycle 1 done 2026-10-19 (63 → 0 files over 500 lines of code, still 0 on 2026-10-25); cycle 2 not started | [`phase-999/`](phase-999/README.md) |

Each phase folder holds a README with that phase's checklist (mirrored from the sections below, which are canonical) and its milestone files.

---

## 🏗️ Phase 0: Foundation & Documentation Infrastructure
*Goal: Establish the project skeleton, workspace, and documentation pipeline.*

*Status 2026-10-25: all items done. The repository is hosted (github.com/louisgoodnews/opts) and the book is live at https://louisgoodnews.github.io/opts/ (`deploy-docs.yml` green since 2026-10-13). ADR-0002 is dated 2026-10-04.*

- [x] **📦 Workspace Setup** *(`cargo check --workspace --offline` passes, verified 2026-10-04)*
  - [x] Initialize Cargo workspace with `opts-core`, `opts-bevy`, `opts-sim`, `opts-tools`, `opts-utils`, and `opts-game`. *(Annotation 2026-10-10: `opts-utils` and other placeholder crates were removed in `07de1b0`; the workspace now has bevy, catalogue, cli, core, game, gui, logging, sim, tools.) (Status sync 2026-10-25: 17 members — benchmark, bevy, catalogue, cli, core (facade), game, gui, hardware, importer, l10n, logging, model, omsi, sim, tools, traffic, types.)*
  - [x] Configure root `Cargo.toml` with shared dependencies and build profiles.
- [ ] **📚 Documentation Pipeline**
  - [x] Set up `mdBook` in the `docs/` directory. *(`docs/book.toml` was a 0-byte file until M-0007; `mdbook build docs` succeeds locally with mdbook 0.5.4, verified 2026-10-25, warnings only)*
  - [x] Configure `SUMMARY.md` to outline the entire project.
  - [x] Set up GitHub Actions to build and deploy `mdBook` to GitHub Pages on push. *(workflow `deploy-docs.yml` present; triggers on `main` and `master`; green on GitHub since 2026-10-13)*
  - [x] Configure GitHub Pages and verify the live site updates (live at https://louisgoodnews.github.io/opts/, verified 2026-10-25).
- [x] **⚖️ Project Governance**
  - [x] Add `LICENSE` (MIT/Apache-2.0 dual license).
  - [x] Add `CONTRIBUTING.md` and `CODE_OF_CONDUCT.md`.
  - [x] Create `docs/src/adr/` for Architecture Decision Records.
  - [x] Write ADR-0001: "Why Bevy and Rust?"
  - [x] Write ADR-0002: "Clean-Room Reverse Engineering Policy" (Crucial for legal safety).
- [x] **📖 Core Documentation**
  - [x] Write `docs/src/introduction.md` and `docs/src/vision.md`.
  - [X] Write `docs/src/glossary.md` (OMSI terms, Bevy terms, transit terminology).
- [x] **📋 Milestone file** — [M-0000](phase-0/M-0000.md) (🟢 complete 2026-10-25: Pages live)

---

## 📝 Phase 1: OMSI Format Specification (The "Docs-First" Phase)
*Goal: Completely document the OMSI 2 file formats and asset pipeline before writing a single Rust parser. This phase produces the specification that `opts-core` will implement.*

*Status 2026-10-25: every planned format is drafted; 20 pages are 🟢 Specified (o3d, x, sco, sli, lists, texture-cfg, org, money, humans, zug, bus, model_cfg, paths_cfg, passengercabin_cfg, sound_cfg, hof, osc, varnamelist, const, setvars) and 23 remain 🟡 Draft (among them tiles, global-cfg, timetables, textures, cfg, misc-types). Clean-room method unchanged: no executables inspected.*

- [x] **📄 Text Config Formats** — drafted *(parsers are hand-written; `nom` is declared in the workspace but unused)*
  - [x] [`formats/text/sco.md`](../formats/text/sco.md), [`sli.md`](../formats/text/sli.md) + [corpus audit](../formats/text/sco-sli-audit.md) (47 undocumented `.sco` keywords, misspelled aliases, BOM/encoding findings).
  - [x] [`formats/text/vehicles/bus.md`](../formats/text/vehicles/bus.md), [`cfg.md`](../formats/text/cfg.md), model/paths/passengercabin/sound configs.
  - [x] [`formats/text/vehicles/hof.md`](../formats/text/vehicles/hof.md), [`osc.md`](../formats/text/vehicles/osc.md), [`varnamelist.md`](../formats/text/vehicles/varnamelist.md), [`const.md`](../formats/text/vehicles/const.md) + [vehicle-side corpus audit](../formats/text/vehicles/vehicle-corpus-audit.md) (UTF-16 hofs, undocumented `$` functions, duplicate varlist names).
  - [x] [`formats/text/org.md`](../formats/text/org.md): number/registration lists.
  - [x] [`formats/text/misc-types.md`](../formats/text/misc-types.md): `.ovh`, `.hum`, `.cti`, `.oft`, `.otp`, `.owt`, `.oop`, `.olf`, `.ocu`, `.zug`, `.amf`, `.rdy`, backups.
  - [x] [`formats/text/texture-cfg.md`](../formats/text/texture-cfg.md): `*.bmp.cfg` surface/moisture/puddle properties.
- [x] **🗺️ Map & Scenario Files** — drafted
  - [x] [`formats/map/global-cfg.md`](../formats/map/global-cfg.md): `global.cfg`, `laststn.osn`, `.dsc` (UTF-16LE pitfall).
  - [x] [`formats/map/tiles.md`](../formats/map/tiles.md): `tile_<x>_<y>.map` (object/spline records), `.terrain` height grid, `.water`, `.LM.bmp`, `.prt`, `.hole`, `.surf`.
  - [x] [`formats/map/lists.md`](../formats/map/lists.md): AI lists, unscheduled traffic, parked/human/driver/registration lists, holidays, time zone, signal routes.
  - [x] [`formats/map/timetables.md`](../formats/map/timetables.md): `Chrono.cfg`, `.ttl`, `.ttp`, `.ttr`, bus stops, station links.
- [x] **🔢 Binary Formats** — 🟢 Specified *(parsers are hand-written; `binrw` is declared but unused)*
  - [x] [`formats/binary/o3d.md`](../formats/binary/o3d.md) + [chunk reference](../formats/binary/o3d-chunks.md): layout confirmed on 165,695 of 165,750 non-empty files; field semantics and coordinate space confirmed by mutation testing [M]. 🟢 Specified.
  - [x] Annotated hex breakdown of a simple `.o3d` (self-authored sample in `o3d.md`).
  - [x] [`formats/binary/x.md`](../formats/binary/x.md): legacy DirectX `.x` (text and binary variants); frame matrix, material ref, and quad triangulation confirmed by mutation testing [M]. 🟢 Specified.
- [x] **🔊 Format inventory & audio analysis** — [`formats/inventory.md`](../formats/inventory.md) (619,821 files, 122 extensions, coverage per extension), full [`.wav` analysis`](../formats/wav.md) (42,375 files: variants, chunks, robustness findings, config references) and nine small stubs (`.dat`, `.xsli`, `.itx`, `.ipr`, `.kyb`, `.kennz`, `.html`, `.off`, `.sichtbar`/`.unsichtbar`) (2026-10-04)
- [x] **🌐 Community cross-check** — [`formats/community-crosscheck.md`](../formats/community-crosscheck.md): reconciles the drafts with the public OMSI community wiki (facts only, CC BY-SA attribution); resolves several open questions (`$msg`, `[rendertype]`, surface types 0–16, `.surf`) and records conflicts (`[matl_alpha]` value distribution differs from `sco.md`, UTF-16 hofs, keyword case).
- [x] **🌐 Wiki Cross-Check & Spec Overhaul** — milestone [M-0006](phase-1/M-0006.md) (complete 2026-10-04; 31 pages, 19 specs updated)
- [x] **🕳️ Documentation Gap Closure** — from [`formats/gap-analysis.md`](../formats/gap-analysis.md) (all complete 2026-10-04)
  - [x] [M-0008](phase-1/M-0008.md) Script Reference — predefined variables and callbacks (`program/`), script sets, scenery scripts, manual review
  - [x] [M-0009](phase-1/M-0009.md) Repaint System — `.cti` / `[CTC]` chain
  - [x] [M-0010](phase-1/M-0010.md) Engine & Environment Configs — `envir.cfg`, weather, scenarios, inputs, `.opl`
  - [x] [M-0011](phase-1/M-0011.md) Passenger Content — ticket packs, money, fonts
  - [x] [M-0012](phase-1/M-0012.md) Chrono and Map Variants
- [x] **🧪 Mutation-Test Round (needs OMSI)** — binary mesh round complete (2026-10-05); map-side round complete (2026-10-08)
  - [x] Run `CHECKLIST.md` (now also covers the axis prediction in `o3d.md` Addendum 2: Y up, Z longitudinal); folded results into `o3d.md` (Q-004 material groups, Q-005 axes/winding/UV, Q-006 matrix, Q-001–003 header) and `x.md` (Q-X01 frame matrix, Q-X02, Q-X07, Q-X08). Both promoted to 🟢 Specified.
  - [x] Design and execute Mutation Kit 2 for the map & scenery side: `[spline]` field layout and axis order (Q-M02 resolved), `[object]` pitch/roll orientation (Q-M03 resolved), texture fallback chain (`.bmp` $\to$ `.dds` exclusive, Q-TX01 resolved), tile dimensions (Q-M01 confirmed), and alpha blending modes (Q-TX03 confirmed).
- [x] **🖼️ Texture Formats** — drafted
  - [x] [`formats/textures.md`](../formats/textures.md): `.dds` / `.bmp` / `.tga` / `.png` / `.jpg` usage over 202,000 files (A8 ground masks, extension-vs-content mismatches, `.dds` name fallback, RLE TGA, V4/V5 BMP headers). Alpha semantics (Q-TX03) and name resolution (Q-TX01) verified by Mutation Kit 2.
- [x] **📐 Coordinate Systems & Math**
  - [x] `docs/src/architecture/coordinates.md`: OMSI's axes/handedness mapped to Bevy (completed 2026-10-05, Q-M01 blocker noted).
- [ ] **✅ Review & Promotion**
  - [ ] Maintainer review of all 🟡 drafts; promote to 🟢 once the high-priority open questions of that spec are resolved.
  - [x] Add the missing index pages referenced by `SUMMARY.md` (`formats/overview.md`, `formats/text/README.md`, `formats/binary/README.md`) — done in M-0007.
  - [ ] Confirm the drafts' provenance sections against [ADR-0002](../adr/0002-clean-room-policy.md) (clean-room policy).
- [ ] **📚 Only shallowly covered:** `.osn` field meanings (only the `[time]` block is specified and read, M-0070), the purpose of `.rra`/`.surf`/`.prt` caches, and vehicle `.dsc` fragments. Tool files (`.pdn`, `.sfk`, `.blend`, …) and `.dll` plug-ins are intentionally not specified.
- [x] **📚 Format specification milestones** — [M-0001](phase-1/M-0001.md), [M-0002](phase-1/M-0002.md), [M-0003](phase-1/M-0003.md) and [M-0005](phase-1/M-0005.md) are complete
- [ ] **🚏 Transit data specifications** — [M-0004](phase-1/M-0004.md) (in progress; only the low-priority GTFS notes are open; timetable fields now measured in `formats/text/timetable.md`, M-0099)

---

## ⚙️ Phase 2: `opts-core` Implementation (The Parsers)
*Goal: Implement the Rust code that translates the specifications from Phase 1 into usable data structures.*

*Status sync 2026-10-25: the modules below live in `crates/opts-omsi` since the Phase 13 split (`3f22ff2e`); the `opts_core::formats::*` paths still resolve through the `opts-core` facade. Later parsers (hof, timetable, chrono, humans, envir, weather, ticketpack, trains, `.osn`, AI lists, …) were delivered under Phase 4 and later milestones.*

- [x] **💾 Binary Mesh Parsers** — [M-0013](phase-2/M-0013.md) & [M-0016](phase-2/M-0016.md)
  - [x] `.o3d` 3D mesh parser (`opts_core::formats::o3d`)
  - [x] `.x` legacy DirectX mesh parser, text and binary (`opts_core::formats::x`) — [M-0016](phase-2/M-0016.md)
  - [x] Clean-room mutation fixtures integration tests (`t01`–`t07`, `t10`–`t12`)
- [x] **📝 Vehicle & Operational Data Parsers** — [M-0014](phase-2/M-0014.md)
  - [x] `.bus` vehicle manifest parser (`opts_core::formats::vehicles::bus`)
  - [x] `paths.cfg` passenger paths parser (`opts_core::formats::vehicles::paths`)
  - [x] `.org` registration & fleet lists parser (`opts_core::formats::org`)
  - [x] `Money/<pack>/*.cfg` currency denominations parser (`opts_core::formats::money`)
- [x] **📐 Model Configuration Parser** — [M-0017](phase-2/M-0017.md)
  - [x] `model.cfg` vehicle rendering manifest parser (`opts_core::formats::vehicles::model`)
  - [x] LOD tiers, viewpoint camera masks, interactive click events, variable-driven animations, and material modifiers
- [x] **🗺️ World & Scenery Parsers** — [M-0015](phase-2/M-0015.md)
  - [x] Scenery object parser (`opts_core::formats::scenery::sco`)
  - [x] Spline profile parser (`opts_core::formats::scenery::sli`)
  - [x] Map manifest parser (`opts_core::formats::map::global`)
  - [x] Tile layout parser (`opts_core::formats::map::tile`)
  - [x] Binary height grid (`.terrain`) and water level (`.water`) parsers (`opts_core::formats::map::{terrain, water}`)


---

## 🎨 Phase 3: `opts-bevy` Integration (The Visuals)
*Goal: Bridge the gap between `opts-core` data and Bevy's ECS and rendering pipeline.*

- [x] **📥 Bevy Asset Loaders & Mesh Bridge** — [M-0018](phase-3/M-0018.md)
  - [x] Implement custom `AssetLoader` for `.o3d` files.
  - [x] Implement custom `AssetLoader` for `.x` files (discovered in M-0016).
  - [x] Implement custom `AssetLoader` for `.sco` and `.sli` files (`OptsSceneryPlugin`).
  - [x] Register the loaders with the `AssetServer` via `OptsMeshPlugin` and `OptsSceneryPlugin`.
  - [x] Mesh Conversion Bridge: convert `opts_core::formats::o3d::O3d` and `XMesh` to Bevy `Mesh` primitives with left-to-right handed coordinate conversion ($Z \to -Z$, winding inversion).
- [x] **🔄 Coordinate Conversion Systems** — [M-0020](phase-3/M-0020.md)
  - [x] Write systems to convert OMSI Z-up coordinates to Bevy Y-up coordinates.
  - [x] Implement camera preset projection bridge for vehicle view definitions (`[add_camera_driver]`, etc.) (`camera_preset_to_bevy_transform`) (M-0020)
- [x] **🌟 Rendering OMSI Assets & Material Bridge** — [M-0019](phase-3/M-0019.md)
  - [x] Implement texture format sniffing and decoding for `.dds`, `.bmp`, `.tga`, `.png`, `.jpg`.
  - [x] Implement custom `AssetLoader` for `.dds` textures in `opts-bevy`.
  - [x] Create Bevy `StandardMaterial` assets from parsed OMSI `Material` and `XMaterial` properties.
  - [x] Spawn parsed `.sco` objects as Bevy entities with `Mesh` and `Transform` components (`spawn_scenery_object`).
- [x] **🎨 Material Design 3 GUI Widget Library (`opts-gui`)** — [M-0021](phase-3/M-0021.md)
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

- [~] **🗺️ World & Map Loading** — [M-0024](phase-4/M-0024.md) & [M-0025](phase-4/M-0025.md)
  - [x] Document the architecture of the map streaming system (`docs/src/architecture/map-streaming.md`). *Done 2026-10-13: written from the code as it runs; limits L1-L8 and open questions Q-MS-01..05 listed.* 🔴 The code already exists without it (critical review 2026-10-11 F-012); write it before further world-loading work.
  - [~] Implement tile grid paging and async chunk streaming coordinator in `opts-game` and `opts-bevy` (M-0024 — traces to F-018). *Paging runs in `opts-game` (`stream_world_tiles`, time-budgeted resumable build since M-0089); async jobs and the `opts-bevy` module are open.*
  - [~] Implement terrain elevation mesh generation and water plane systems from `.map.terrain` and `.map.water` (M-0025 — traces to F-018). *Terrain mesh, carve, layered ground, water with depth tint and the specification ([terrain-rendering.md](../architecture/terrain-rendering.md), 2026-10-19) done; every painted `[groundtex]` layer blended since 2026-10-25 (`9aadcf56`); open: one height service for the game, normal stitching, wave displacement.*
- [~] **🛞 Vehicle Physics & Control** — [M-0022](phase-4/M-0022.md) & [M-0023](phase-4/M-0023.md)
  - [x] Document the vehicle physics model in `docs/src/simulation/vehicle_physics.md`.
  - [x] Implement kinematic/dynamic vehicle movement in `opts-sim`.
  - [x] Implement Bevy ECS bridge plugin in `opts-bevy` connecting `VehicleSimulation` state to entity `Transform` and wheel mesh rotation. (M-0023)
  - [x] Script VM (compiles all 6,062 corpus scripts; 1,216 scripted vehicles load and run) wired to the vehicle through a passive and an active Bevy bridge — OMSI Script Stack VM (`.osc`) and dynamic variable table system in `opts-sim` ([M-0026](phase-4/M-0026.md) — traces to F-008, F-019).
  - [~] Map OMSI script variables (`.osc`) to Bevy systems for lights, doors, and indicators — animation, `[visible]`, interior lights, `Spot_Select` headlights, light maps, `keyboard.cfg` key triggers and `[mouseevent]` hot-spots done; vehicle night maps (2026-10-25) and `[light_enh_2]` done; in-game screenshot check and `delay` open ([M-0045](phase-4/M-0045.md)).
  - [~] AI road traffic — [M-0049](phase-4/M-0049.md): research and plan ([`ai-traffic.md`](../architecture/ai-traffic.md)), AI list parsers and the `opts-traffic` crate (lane graph, car following, spawning helpers) done; traffic runs in the game (IDM lane following, speed limits and density rules, signals, conflict zones, right of way, lane changes, trailer chains); scheduled AI in M-0112, depth work in M-0111; debug overlay/`traffic.*` methods, `[splinehelper]` arms, script-driven driving and calibration open.
  - [x] Sound engine in `opts-bevy` (2026-10-20): clips, emitters, voice budget, distance fading, listener, buses with the Audio settings, `audio.*` protocol ([M-0027](phase-4/M-0027.md), [audio-system](../architecture/audio-system.md)).
  - [~] Sounds of scenery objects and splines ([M-0139](phase-4/M-0139.md)), of AI traffic ([M-0140](phase-4/M-0140.md)), then the player's vehicle: engine RPM and pneumatic pressure to spatial audio ([M-0102](phase-4/M-0102.md)); first slices 2026-10-20, AI doors/horn/second unit 2026-10-24; announcements, `Snd_*`, force feedback, brake squeal open.
- [ ] **🎛️ Vehicle Controls and Systems** — [M-0141](phase-4/M-0141.md) — requested 2026-10-20
  - [~] The driver's workplace works: electrics, engine start procedure per vehicle, lights, doors, heating, wipers and every dashboard control with a function; a start-procedure test per reference vehicle (Citybus 530, MAN NL/SD200/NG272 and BRT C2 start from cold since 2026-10-20, the Luzern VBL Citybus, Valiant 7000 and MB C2 BVG families since 2026-10-25, then the TH Überlandbus, Studio Polygon Lite, Citybus S31x and others; all 948 installed buses reach drive readiness (the last, the MB O530 GDH serial hybrid, since 2026-10-25); dashboard displays: printers, VDV display and the HH109 EFAD LCD work since 2026-10-25, NG272/HH109 destination boards open).
  - [~] Articulated buses — trailing segments follow the leader kinematically, the joint angle reaches the scripts, bellows are skinned ([spec](../architecture/articulation.md), M-0022 addendum 2026-10-11); one serialisable consist state, consist mass and β/γ in the fixed step done 2026-10-14 (M-0053 Parts 1–3), AI trailer chains done; β limits, `[couple_front]` and parked trailers open — [M-0053](phase-4/M-0053.md) (critical review 2026-10-11 F-002, F-003).
- [~] **🚶 Pedestrians** — [M-0050](phase-4/M-0050.md): walking pedestrians on footpath lanes with gait animation done ([design](../simulation/pedestrians.md)); `[trafficdensity_passenger]` applied; stops and boarding through M-0101 (first slice 2026-10-21); crossings/signals, holidays, separation and LOD open.
- [~] **🧑‍🤝‍🧑 Passenger & Routing System**
  - [x] Document the passenger simulation model in `docs/src/simulation/passengers.md` (written 2026-10-21 with M-0101) — prerequisite for M-0050's stop/boarding items (critical review 2026-10-11 F-023).
  - [~] Implement bus stop logic and passenger boarding/alighting — first slice [M-0101](phase-4/M-0101.md) 2026-10-21: `[busstop]` objects, boarding through the scripts' doors, tickets, articulated buses and trams; cash desk open. *Requirement from rail operation (2026-10-12): a stop evaluates the leading head's direction of travel and door side, so a tram arriving B-head first is recognised without a mirrored second stop cube ([rail-vehicles §8–9](../simulation/rail-vehicles.md)).*
  - [~] Implement a basic routing algorithm (e.g., RAPTOR or Dijkstra via `petgraph`). *Vehicle routing over lanes exists (`LaneGraph::shortest_route`, used by scheduled AI and the route line on the map); passenger journey routing open.*
- [~] **🎮 The Game Binary** *(status corrected 2026-10-11, critical review F-014)*
  - [x] Assemble `opts-game/src/main.rs` by adding the `opts-bevy` and `opts-sim` plugins to the Bevy `App` (`crates/opts-game/src/app.rs`).
  - [~] Implement the main menu, loading screens, and basic HUD — menu (`menu/`), new-game screen, loading pipeline and in-game HUD exist; Load game opens the saves overlay (M-0069); there is no Credits screen.
- [x] **📦 Delivered outside milestones (reconciled 2026-10-10, [M-0037](phase-4/M-0037.md))** — shipped after M-0023 without their own milestone; no earlier status changed.
  - [x] Workspace modularisation, `opts-catalogue` (map/vehicle catalogue) and `opts-logging` — `07de1b0`, [report](../meta/modularization-2026-10-09.md).
  - [x] Real OMSI asset loading, tile streaming (`stream_world_tiles`), terrain mesh, data-driven sky, layered ground shader, spline attachments, camera system, humans/ego perspective — `69e94b2`, `780cae4` (crates `opts-bevy`, `opts-game`).
  - [x] Map, scenery, humans and envir parsers; `.o3d`/`.x` robustness; scrambled-mesh decoder (partial, see [M-0038](phase-4/M-0038.md)) — `438d622` (`opts-core`).
  - [x] F1 reporting and GUI modal/text area/fonts — `f5dfd38`, [report](../meta/f1-reporting-2026-10-10.md).
  - [x] Startup OMSI-installation check with folder dialog and saved path (`a68ad63`); parked vehicles from `car_park` markers and vehicle-model scenery objects (`304856e`); repaint discovery from `.bus`/`model.cfg`/`.cti` (`c00c3fc`); scrambled-mesh decoder keeps all triangles and infers UV jitter (`c00c3fc`); map-list scrolling, back buttons, absolute-height crossings (`5c752fe`).
  - [x] Control protocol and hot reload — M-0043 (see Phase 5); spline road/terrain fix — `2816806`, [bug report](../meta/bugs/f17228d9-3bc0-4830-8f25-3bc9465f63d2_2026-10-10_16-44-31-spline-winding.md).
- [ ] **🧾 Audit-Driven Work (audit 5037f89a, 2026-10-10)**
  - [x] Reconcile roadmap, phase pages and board with work shipped since M-0023 — [M-0037](phase-4/M-0037.md) (F-019, F-023).
  - [x] Complete scrambled-mesh decoding: async decoding, deterministic parameters, cache policy, generator research — [M-0038](phase-4/M-0038.md) (F-001, F-003, F-014, F-017). (exact decoding 2026-10-16, `c3d10946`; heuristic decoder retired, `a17ac313`)
  - [x] Render `[texttexture]` text on signs, stops and displays — [M-0039](phase-4/M-0039.md) (F-002).
  - [~] Script-driven animation, lights and doors (all but the in-game screenshot check and `delay` done) — [M-0045](phase-4/M-0045.md).
  - [ ] World loader extraction — [M-0046](phase-4/M-0046.md).
  - [ ] Extract the world loader into `opts-bevy` and decouple `opts-bevy` from `opts-gui` (appended to [M-0024](phase-4/M-0024.md); F-015, F-016).
  - [~] Verify spline-attachment repeaters, fix floating objects, read weather/fog (appended to [M-0025](phase-4/M-0025.md); F-004, F-007, F-008). — weather/fog read since 2026-10-17, repeater layout measured over the corpus; Rheinhausen floating object and an OMSI check open
- [ ] **🧾 Critical-Review Work (critical review 2026-10-11, [report](../meta/critical-review-2026-10-11.md))**
  - [x] Host callbacks of the vehicle scripts — timetable, termini, bus stops, tickets, IBIS (`NrSpecRandom` done): destination displays and cabin monitors show their text — [M-0093](phase-4/M-0093.md) (complete 2026-10-14).
  - [x] Script textures (`[scripttexture]`, `ST*` callbacks) for dash TFTs, tachographs, printers — [M-0094](phase-4/M-0094.md) (complete 2026-10-14).
  - [~] Articulated vehicle completion: one articulation model with one serialised state in `opts-sim`, consist mass in the longitudinal dynamics, β/γ in the fixed step ([design](../architecture/articulation-physics.md); scope proposed 2026-10-14), `[couple_front]`, trailers for AI and parked vehicles — [M-0053](phase-4/M-0053.md) (F-002, F-003). (Parts 1–3 done 2026-10-14)
  - [ ] Move traffic, pedestrian, articulation and repaint loading out of `opts-game` with the world loader (appended to [M-0046](phase-4/M-0046.md); F-009).
- [~] **🗺️ Tile Loading Controls** — [M-0080](phase-4/M-0080.md)
  - [~] *Load all tiles on start* (force-loaded and kept resident), exposed in the wizard summary; *Tile radius* around the centre tile with a hardware-derived default (R-20, R-21). — both settings, live streaming and `scene.tiles` done 2026-10-18; wizard summary switch, memory guard and "Automatic (currently N)" open.
- [~] **🎥 Viewpoint Editor** — [M-0084](phase-4/M-0084.md)
  - [x] Shift + Tab (Tab until 2026-10-22) opens an editor in the driver's view (height, X/Y/Z, FOV), saved per vehicle or per model; `graphics.default_fov`; Shift + Space resets each view (R-26, R-33). Export/import open.
- [x] **🌀 Tile Streaming Performance** — [M-0089](phase-4/M-0089.md)
  - [x] Measure where the frame time goes at tile borders, rank and implement the improvements (R-32, R-36). (complete 2026-10-18: no frame above 50 ms, worst 36.9 ms on Berlin-Spandau)
- [x] **🚌 Script rig and callback safety net** — [M-0096](phase-4/M-0096.md)
- [x] **🚌 Route, timetable and IBIS data layer** — [M-0099](phase-4/M-0099.md)
- [x] **🚌 Consist script state: part engines, script share and control cables** — [M-0098](phase-4/M-0098.md)
- [~] **🚌 Script chain and course geometry** (proposed 2026-10-14; M-0097 implemented, M-0100/M-0101/M-0102 first slices, M-0103 started through M-0112, M-0105 still proposed) — [M-0097](phase-4/M-0097.md) physics contract, [M-0100](phase-4/M-0100.md) script textures, [M-0101](phase-4/M-0101.md) doors, passengers, tickets and cash, [M-0102](phase-4/M-0102.md) sound events and force feedback, [M-0103](phase-4/M-0103.md) AI scripts near the player, [M-0105](phase-4/M-0105.md) course geometry (research)

---

## ⚡ Phase 5: Tooling, Performance & Optimization
*Goal: Profile, benchmark, and optimize engine throughput, and provide developer CLI inspection utilities.*

- [ ] **⚡ Performance & Optimization** — [M-0028](phase-5/README.md) & [M-0029](phase-5/M-0029.md)
  - [ ] Implement Criterion microbenchmark suite in `opts-benchmark` for format parsers and vehicle physics step (M-0028 — traces to F-021).
  - [ ] Document LOD (Level of Detail), frustum culling, and occlusion strategies in `docs/src/architecture/lod-culling.md`.
  - [~] Implement distance-based dynamic LOD mesh switching and shadow culling in `opts-bevy` (M-0029). *Scenery `[LOD]` bands, view, shadow, light and detail distance settings live (2026-10-20/24); tile mesh LOD open.*
  - [~] Implement async background asset streaming to prevent frame drops during tile boundary crossings. (time-budgeted, resumable tile build, [M-0089](phase-4/M-0089.md) steps 1–2, 2026-10-18).
- [ ] **🧰 Developer CLI Tooling** — [M-0030](phase-5/README.md)
  - [ ] Document CLI command specifications in `docs/src/reference/opts-tools.md`.
  - [x] Implement `opts inspect <file>` to validate OMSI asset syntax and detect corrupt files (M-0030). — delivered as `opts-tools inspect <file>...` (`28249ea8`, 2026-10-10).
  - [ ] Implement `opts convert <file> --output <gltf>` to export legacy meshes to standard formats.
- [x] **🎛️ Remote Control & Live Editing** — [M-0043](phase-5/M-0043.md)
  - [x] Design documents and ADRs: [Control Protocol](../architecture/control-protocol.md), [Hot Reload](../architecture/hot-reload.md), ADR-0006, ADR-0007.
  - [x] `opts-cli` client, `opts-bevy::control` server and `opts-bevy::hot_reload` ([final report](../meta/control-protocol-2026-10-10.md)).
- [x] **⬆️ Engine Upgrade** — [M-0044](phase-5/M-0044.md)
  - [x] Migrate Bevy 0.14 → 0.19.1 (done in one step; see [M-0044](phase-5/M-0044.md)).
- [x] **⬆️ Bevy 0.20** — [M-0047](phase-5/M-0047.md) (plan: [strategy](../architecture/bevy-0-20-dlss.md))
- [ ] **🎞️ DLSS upscaling (optional feature)** — [M-0048](phase-5/M-0048.md)
- [ ] **🧹 Audit-Driven Hardening (audit 5037f89a, 2026-10-10)**
  - [~] Code-quality debt (rustdoc, panics, fixtures, stubs, Bevy bundle links and file splits done — 0 files over 500 lines of code; function splits and `SystemParam` grouping open): size limits, panics in library code, stubs, rustdoc — [M-0040](phase-5/M-0040.md) (F-005, F-006, F-009–F-012).
  - [x] Documentation repair and developer switch reference — [M-0041](phase-5/M-0041.md) (F-013, F-018, F-020–F-023).
  - [~] CI and toolchain hardening (running on GitHub since 2026-10-13; every job green since 2026-10-25, the Windows test failures were test assumptions about line ends, letter case, Unix paths and timing; `binrw` no longer built; broken-link proof open): link check, mdBook build, toolchain pin, Windows job — [M-0042](phase-5/M-0042.md) (F-024–F-026).
- [ ] **🧾 Critical-Review Work (critical review 2026-10-11, [report](../meta/critical-review-2026-10-11.md))**
  - [ ] 🟡 GitHub hosts the repository (2026-10-24), CI runs on every branch, commits can be gated on the staged tree (`scripts/install-hooks.sh`); Pages deploys and the site is live; `ci.yml` fully green since 2026-10-25; open: the remote broken-push proof — [M-0052](phase-5/M-0052.md) (F-001, F-018, F-019, F-020).
  - [~] Stop the size and lint-allow growth: fail CI on new size violations (file ratchet in `local-ci.sh` since 2026-10-19, M-0125; not in hosted CI), group load-job parameters (open; lint allows 37 → 144), fix stale Bevy rustdoc links (done, `3c760f73`) (appended to [M-0040](phase-5/M-0040.md); F-005–F-007).
  - [x] Decide and execute the `bevy-0.20` → `master` merge (appended to [M-0047](phase-5/M-0047.md); F-019) — fast-forwarded 2026-10-11.
  - [x] Roadmap copy generator with CI check, crate-graph table, crate guides, README and status reconciliation (done in the review pass; F-011, F-013–F-015, F-017).
- [ ] **🔬 Scenery Object Load Verification** — [M-0054](phase-5/M-0054.md) (spec: [scenery benchmark](../architecture/scenery-benchmark.md))
  - [x] Specify the `opts-benchmark` scenery harness: one-tile generated map, one object at a time, off-screen capture, automatic placement/geometry/texture checks, script-effect checks, report format (Phase A).
  - [x] Implement the harness with static checks and calibrate thresholds (Phase B). (done 2026-10-12)
  - [~] Script-effect checks; audio recorded only (Phase C: runtime, scenarios, judgement and tests done 2026-10-12; corpus calibration open).
  - [~] Full corpus run and triage report (Phase D, done 2026-10-12, [triage](../meta/scenery-benchmark-triage-2026-10-12.md)); fixes by failure category (Phase E: E1 done, E2/E5 fixed without corpus re-run).
- [x] **🛠️ Settings and Configuration through the Control Protocol** — [M-0072](phase-5/M-0072.md)
  - [x] Reproduce the reported gap; confirm/revert and restart-required handling, batch set, export/import, registry-wide coverage test (R-10).
- [ ] **🧭 Roadmap Adherence and Expansion** — [M-0074](phase-5/M-0074.md)
  - [~] Drift audit, intake rule and automated consistency checks; gap analysis proposing the next milestones (R-06). (audit and checks done 2026-10-18; regular review moved to M-0119; intake rule open)
- [~] **🖥️ `opts-hardware`** — [M-0081](phase-5/M-0081.md)
  - [x] Privacy-respecting hardware capability probe crate (no identifiers, no network, auditable field list) and recommendation functions for tile radius, filtering, look, UI scale (R-22). Linux verified; macOS and Windows compile-checked in CI, untested on real hardware.
  - [ ] Report dialog preview with checkbox; GPU benchmark.
- [x] **🧬 Mesh Instancing Research** — [M-0082](phase-5/M-0082.md)
  - [x] Research branch `research/mesh-instancing`: inventory, Bevy mechanisms, prototype, measurements, recommendation (R-23). (merged 2026-10-17; follow-up milestone of report §9 not yet added)
- [x] **🪞 Reflections Research** — [M-0085](phase-5/M-0085.md)
  - [x] Research branch `research/reflections`: mirrors, glass, later wet surfaces; prototype, measurements, recommendation (R-27). (merged 2026-10-17; mirrors, glass and wet ground implemented)

---


## 🌐 Phase 6: Modding Ecosystem & Community Content
*Goal: Empower content creators to build, package, validate, and share vehicles, scenery objects, maps, and repaints.*

- [ ] **📦 Modding Infrastructure** — [M-0031](phase-6/README.md) & [M-0032](phase-6/M-0032.md)
  - [ ] Specify mod directory layout, virtual filesystem mounting, and load priority rules in `docs/src/architecture/modding-vfs.md` (M-0031).
  - [~] Implement `.cti` repaint manifest parser and runtime livery material switcher in `opts-bevy` (M-0032). *Repaint discovery, selection in the new-game screen, texture replacement at load time and `[setvar]` equipment shipped in Phase 4 (`c00c3fc`, `7f24124`); repaints for AI, parked and depot vehicles shipped 2026-10-17/24 (`0c71c549`, `27112b8f`); switching at runtime and the `opts-bevy` home are open (critical review 2026-10-11 F-022).*
- [ ] **✏️ Editor Mode** — [M-0051](phase-6/M-0051.md): specification in [docs/src/editor](../editor/README.md); implementation phases E1–E6 open.
- [ ] **📜 Script Generator & Visual Script Editor** — specification in [docs/src/architecture/script-generator.md](../architecture/script-generator.md), derived from [vehicle-scripts-route.md](../architecture/vehicle-scripts-route.md): a typed vehicle specification (mass, wheel radius, torque curve, gear ratios, brake force, doors, lights …) becomes a default script set (`.osc`, `*_constfile.txt`, `*_varlist.txt`, `.bus` script block) written to the OPTS export folder, validated by the VM and a headless rig, and edited visually.
  - [ ] [M-0092](phase-6/M-0092.md) `opts-script_engine`: the script runtime (lexer, compiler, VM, headless rig) as a Bevy-free crate; `opts-sim::script` re-exports it. A headless rig already exists as the `opts-bevy` example `script_rig` (M-0096).
  - [ ] [M-0055](phase-6/M-0055.md) `opts-script_editor`: `VehicleSpec`, subsystem generators with original templates, export writer, validator, `opts-tools script gen|validate`, presets (city bus, articulated bus, tram).
  - [ ] [M-0056](phase-6/M-0056.md) `opts-visual_script_editor`: spec forms, generated-file viewer, live rig plots, subsystem graph, export; embedded in the editor mode's vehicle workspace.
- [ ] **📘 Creator Documentation & Tooling** — [M-0033](phase-6/README.md)
  - [ ] Author `docs/src/modding/overview.md` and `docs/src/modding/asset-pipeline.md`.
  - [ ] Author vehicle and map creation guides for content authors.
  - [ ] Implement mod package validation linter in `opts-tools` (M-0033).

---

## 🚀 Phase 7: Release & Cross-Platform Distribution
*Goal: Package, validate, and distribute production-ready binaries of OPTS across Linux and Windows platforms.*

- [ ] **⚙️ Release Engineering** — [M-0034](phase-7/README.md)
  - [ ] Configure multi-platform GitHub Actions release matrix for Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`) (M-0034 — traces to F-017).
  - [ ] Implement automated release workflow generating compressed binary packages with SHA-256 checksums.
- [ ] **📖 User Documentation & Settings UI** — [M-0035](phase-7/README.md)
  - [ ] Author end-user installation, controls, and troubleshooting manuals (`docs/src/user/`).
  - [x] In-game settings dialog for graphics preferences: delivered by the settings page of M-0063 (2026-10-12); audio volume appears with M-0027.
  - [ ] Input rebinding (M-0035): the *Controls* page shows it as "Feature not available yet!"; the keyboard part moved to [M-0142](phase-7/M-0142.md).
- [~] **🖱️ Driving with the Mouse** — [M-0143](phase-7/M-0143.md) — first slice 2026-10-20 (cursor height pedals, width steering, wheel press switches, right click frees the mouse)
- [~] **⌨️ Rebind Every Key** — [M-0142](phase-7/M-0142.md) — first slice 2026-10-20 (one registry of built-in actions, hints follow the binding, reset); rail keys and per-vehicle-kind layouts 2026-10-22 (26 actions); road driving, gears, cameras, pause still fixed
- [ ] **🏁 Milestone Release** — [M-0036](phase-7/README.md)
  - [ ] Publish `v0.1.0-alpha` public release milestone on GitHub Releases (M-0036).

---

## 🖼️ Phase 8: Visual Quality & Realism
*Goal: Make OPTS look like the real world: correctly filtered, sharp textures and colours that follow physical reality, with a vivid default look. OMSI's own appearance is not the reference. Decisions of 2026-10-12: reality as reference, Vivid default, hard limit of 2 minutes of added first-run time. Background: [Texture Quality and Colour](../architecture/texture-quality.md).*

- [ ] **🔬 Texture Filtering & Measurement Harness** — [M-0057](phase-8/M-0057.md)
  - [~] Image-metrics tool and scripted A/B run done (`scripts/visual-ab.sh`, five vehicle-relative poses, results outside the repository); reference views on three maps and VRAM figures open.
  - [x] One shared object-texture sampler (trilinear, anisotropy up to 16×, default 16×) with a settings entry (accepted by the project lead 2026-10-25).
- [ ] **🪜 Mipmaps & Derived Cache** — [M-0058](phase-8/M-0058.md)
  - [x] Mipmap generation for all uncompressed textures (sRGB-correct) at load time and in hot reload (accepted 2026-10-25).
  - [x] Chains for the DXT textures without one (48,217 in the installation) through a derived cache, re-encoded with `texpresso` (done 2026-10-25; Köln: +4 s on the first load, 0.2 s from the cache; far below the 120 s limit, so the budget machinery was dropped).
- [ ] **🌿 Foliage & Cutout Coverage** — [M-0059](phase-8/M-0059.md)
  - [~] Coverage-preserving alpha mips (in place for detected cutouts since M-0058, fixed cutoff 0.5); alpha-to-coverage with MSAA and per-class cutoffs open.
- [ ] **🌞 Realistic Colour & Looks** — [M-0060](phase-8/M-0060.md)
  - [~] Looks *Vivid* (default), *Natural*, *Bevy default*, live-switchable and persisted (implemented; reviewed 2026-10-25: the effect is too weak and needs expanding; *Natural* equals *Bevy default* until the calibration).
  - [ ] Physically calibrated sun, sky and exposure with cited reference values; reference-surface tests.
- [ ] **🌑 Shadow Quality and Realism** — [M-0091](phase-8/M-0091.md)
  - [ ] Assess and improve shadows with presets and measurements (R-40); a live *Shadow distance* setting exists since 2026-10-24 (`4d2a4955`).
- [ ] **🌳 Trees Move in the Wind** — [M-0144](phase-8/M-0144.md) — requested by the project lead 2026-10-26 (OMSI's trees are static: an OPTS extension)
  - [x] Specification: [vegetation wind](../architecture/vegetation-wind.md).
  - [x] `[tree]` billboards bend in a vertex stage (shadows included) with the weather's wind; setting `graphics.tree_wind` (2026-10-26).
  - [ ] Mesh trees (`.o3d` trunk and foliage cards).

---

## 🧭 Phase 9: User Interface & Experience
*Goal: Design and build the user interface of OPTS around evidence-based rules: how it should look, feel and flow, and how it grows without rewrites. Status: design approved 2026-10-12 (decisions D-UI-01 to D-UI-11 answered; unavailable features stay visible with the red text "Feature not available yet!"); implementation in progress. Documents: [phase page](phase-9/README.md), [ADR-0009](../adr/0009-ui-architecture.md).*

- [ ] **🧱 UI Foundations** — [M-0061](phase-9/M-0061.md)
  - [x] Theme foundations (part A, 2026-10-12): dark default with Light, System, Auto (clock) and High-contrast themes; Noto Sans type scale; UI scale 75–200 %, density, layout classes, 1024×640 minimum window; status roles, shadows, motion with reduced motion; accent presets with a contrast guard and RON theme files; one theme for loading screen, HUD and stubs; instrument tokens.
  - [x] Hover/focus/pressed/disabled/error states and keyboard operability in every widget (part B, 2026-10-12, `agent/ui-widgets`).
  - [x] Action layer, UI ids with `ui.tree`/`ui.click` in the control protocol, CI rules (target size, contrast, keyboard traversal), localisation scaffold (2026-10-12).
  - [ ] Open: layer-model section of the design-system page, single bundle-returning builders, first-run UI-scale suggestion.
- [ ] **🧭 App Shell & Flow** — [M-0062](phase-9/M-0062.md)
  - [x] Shell part (2026-10-12, `agent/ui-shell`): navigation rail and screen registry, Home with *Continue*, first run, pause overlay, end-of-session summary, Help and About with F1, notifications; stubs and developer copy removed.
  - [x] Four-step *Plan a drive*, honest loading screen, master-detail pattern (wizard part, 2026-10-12, `agent/ui-wizard`); acceptance goldens at three sizes open.
  - [x] Find OMSI 2 through Steam's library configuration (every library, Linux/macOS/Windows) and use it without asking; ask only when Steam does not know it (project lead 2026-10-25, done the same day; Windows registry path untested on real Windows).
- [ ] **⚙️ Settings System** — [M-0063](phase-9/M-0063.md)
  - [x] Declarative setting registry, generic settings page with search, *Reset* and revert prompt; Graphics, Interface, Content, Gameplay and Advanced pages with real controls; `settings.*` in the control protocol; generated settings reference (2026-10-12).
  - [x] Audio with M-0027 (2026-10-20).
  - [~] Controls: keyboard rebinding through M-0142 (first slice 2026-10-20); gamepad, wheel and per-vehicle controls open (`controls.rebind` still "Feature not available yet!").
- [ ] **🕹️ HUD & In-Game UI** — [M-0064](phase-9/M-0064.md)
  - [x] HUD from registered widgets and vehicle-type profiles (RON), widget switches in Settings, alerts, 1.5 s camera label, Details dialog, developer panel off by default (2026-10-12, `agent/ui-hud`).
  - [~] Help overlay generated from the controls registry and the player's bindings; *Pause → Vehicle* in place; open: HUD review sign-off and glance-time measurement.
- [ ] **♿ Accessibility, Localisation & Validation** — [M-0065](phase-9/M-0065.md)
  - [x] Automatic part (2026-10-12, `agent/ui-a11y`): rules over the laid-out UI tree for target size, text size, contrast in every theme and accent, text overflow in the pseudo-locale and German, UI scale 75–200 %, accessible names, gamepad traversal; colour-blind simulation; golden screenshots of fixture content; WCAG 2.2 AA audit with findings (8 fixed); German/English completeness over all tables; language choice at first run.
  - [ ] Moderated usability test (protocol written, **no participants yet**), screen-reader and hardware checks, GPU screenshots, loading screen and report dialog in the matrix (the HUD is in since M-0071).
- [x] **🕗 Start Time Defaults from the Map** — [M-0070](phase-9/M-0070.md)
  - [x] *When* step preselects the time of day from the map's `laststn.osn`, else the device's local time; the date keeps the map default date (decision 2026-10-18); the source is shown (R-07).
- [x] **🗺️ HUD Layout Rework** — [M-0071](phase-9/M-0071.md)
  - [x] Bottom-left cluster (hints, speed, gear, parking brake, alerts) and a bottom-right mini map (F7) within the HUD registry and UI standards (R-08).
- [x] **📍 HUD Place Label** — [M-0075](phase-9/M-0075.md)
  - [x] Replace the hard-coded place string by the entry point name of the active vehicle's spawn; hidden once it has moved away or is despawned (R-12).
- [x] **⏳ Loading Screen Vehicle** — [M-0076](phase-9/M-0076.md)
  - [x] Show the selected vehicle (or none) from the first frame; remove the MAN SD77 default (R-13).
- [ ] **🌍 Language Packs** — [M-0083](phase-9/M-0083.md)
  - [x] Contributors and AI agents add languages as validated files in the user folder (`<data>/lang/<code>/`); `opts-tools l10n export|check|new|import|stats`; discovery at start and on reload; language list with completeness; `l10n.*` control methods (R-25, 2026-10-13, `feat/language-packs`, [architecture](../architecture/localisation-packs.md)).
  - [ ] Trial in the running app with a real second language; right-to-left layout and fonts beyond Latin, Greek and Cyrillic.
- [x] **🔎 Searchable Dropdowns & Honest Catalogue Counts** — [M-0086](phase-9/M-0086.md)
  - [x] Dropdown menus scroll (R-28, `fix/dropdown-scroll`).
  - [x] Search field in long dropdowns (R-28); catalogue summary with a breakdown by kind (R-24) — `feat/combobox-search`.
  - [x] Scan status chip next to *Scan now*: last scan time, green / yellow / red with documented rules (R-29).
- [x] **🧊 No Blocking Work on the Main Thread** — [M-0088](phase-9/M-0088.md)
  - [x] Settings Content page freeze: move catalogue/file-system work off the main thread, audit the UI for more (R-31).
- [x] **🗺️ Per-Map Configuration Profiles and Load Profiler** — [M-0130](phase-9/M-0130.md)
  - [x] Phase A design (profile scope, TOML store, lifecycle and fallback map → Chrono → global, profiler heuristics, modal); checkpoint; phase B implementation; checkpoint; phase C verification — accepted by the project lead after testing 2026-10-22.
- [x] **🚏 Plan a Drive: Choose a Trip** — [M-0145](phase-9/M-0145.md)
  - [x] Optional wizard step *Trip* (the map's player runs departing within the hour on the plan's weekday); the bus starts at the run's first stop and follows the run; `scene.load_map {trip}` (protocol 1.28) — 2026-10-26.

---

## 🧩 Phase 10: Sessions, World Completeness & Vehicle Dynamics
*Goal: Turn the driving demo into a session-based simulator: water and correct content in the world, sessions with several user vehicles that can be saved and restored, and vehicles whose own mass, axles, suspension and brakes are respected by a shared world physics. Status: planned 2026-10-13 from the project lead's [field report](../quality/field-report-2026-10-13.md); in progress (status sync 2026-10-25: M-0068/69/73/78/106 done, M-0066 awaiting acceptance, M-0079/M-0138 deferred, M-0104 obsolete). Documents: [phase page](phase-10/README.md).*

- [ ] **🚗 Mesh Fidelity: Vehicles, Parked Cars, Buildings, Lanterns** — [M-0066](phase-10/M-0066.md) — 🟢 tasks done and merged 2026-10-19, awaiting the project lead's acceptance
  - [ ] Parked cars: no scrambled, missing or transparent meshes (R-01); BRT Citaro UV/mesh issues (R-09); flipped or misaligned building meshes (R-17); scrambled lantern posts (R-18); fixes with regression tests, remaining content defects registered.
- [ ] **🔬 UV Decoder Fleet Validation** — [M-0079](phase-10/M-0079.md) — ⏸ deferred by the project lead 2026-10-13
  - [ ] Decode every keyed `.o3d` of an installation, coverage report with the per-mesh table and the known-broken list, `opts-cli validate-uvs`, visual spot-checks, CI item. The heuristic decoder (P-1013 phases A–C) was retired 2026-10-16 (exact decoding, o3d.md Addendum 15), so this validation has no subject; candidate for retirement.
- [ ] **🧪 UV Decoder Texture Evidence** — [M-0104](phase-10/M-0104.md) — ⛔ obsolete since 2026-10-16 (exact decoding, o3d.md Addendum 14)
  - [ ] Feasibility first: does the plain texture atlas rank the true scrambler offset on the twin truth set; integrate into the smooth stage only on a go ([design](../architecture/uv-decoder-texture-evidence.md)).
- [x] **🚆 Train Formation Editor v1** — [M-0106](phase-10/M-0106.md) — ✅ done 2026-10-15
  - [x] `.zug` parser and presets, formation editor in the spawn menu, consist spawner with summed mass, TOML store ([design](../architecture/train-formation-editor.md)).
- [ ] **🌊 Water Rendering** — [M-0067](phase-10/M-0067.md) — 🟡 drawn since 2026-10-16, depth tint 2026-10-18; Q-M07 (marker rule, interpolation) and Q-M08 (water textures) open
  - [ ] Resolve the `.water` format semantics, parse the 1,485 corpus files, render water per tile without seams, reality-calibrated material (R-02).
- [x] **🚌 Session Vehicles** — [M-0068](phase-10/M-0068.md)
  - [x] Start with the map only; session vehicle registry; switch the active vehicle; despawn any session vehicle (R-03, R-04).
- [x] **💾 Session Save and Restore** — [M-0069](phase-10/M-0069.md)
  - [x] Versioned, section-based save format with a registry so weather, timetables and more can be added; Load game becomes real (R-05).
- [ ] **🧰 Bus Configurations** — [M-0077](phase-10/M-0077.md) — 🟡 all but thumbnails done 2026-10-20
  - [ ] Named, quick-loadable configurations of bus, repaint and setvars (R-15).
- [x] **🎛️ Setvars** — [M-0078](phase-10/M-0078.md)
  - [x] Per-bus setvar lists derived from `.cti`, varnamelists and the Add-on PDFs (facts only, ADR-0002), editor and apply (R-16).
- [x] **⚙️ Vehicle Dynamics** — [M-0073](phase-10/M-0073.md)
  - [x] World physics plus per-vehicle dynamics specs from vehicle data: mass, axle loads, per-wheel suspension, brake system; validated by measurable differences between vehicles (R-11).
- [ ] **🚌 BRT C2 Full Fix** — [M-0138](phase-10/M-0138.md) — requested 2026-10-20, ⏸ deferred the same day (first slice unmerged on `agent/c2-full-fix`)
  - [ ] Complete part census of `C2_2T.bus`, every part built, bound and drawn as authored or registered as a content defect; regression gate; the project lead's look.
- [ ] **🏙️ Content Fidelity Pass** — [M-0090](phase-10/M-0090.md) — 🟡 R-34/35/37–39/41 done or re-checked, R-36 partly, R-42 needs a position
  - [ ] Pedestrians, parked and driving cars (transparency, wheel animation), Hamburg partial loads, Wuppertal Schwebebahn stations and T-crossings (R-34 … R-42).

## 🔭 Phase 11: Advanced Systems, Realism & Continuous Improvement
*Goal: Build on the session-based simulator with the systems that make it feel finished: in-game map, comfort and accessibility settings, setvar and config depth, deeper and scheduled AI traffic, track switches, adaptive streaming, tunnels, real weather and its effect on driving, the OMSI event system, and a standing roadmap-review practice. Status: planned 2026-10-18; approved by the project lead 2026-10-20, open for implementation. Documents: [phase page](phase-11/README.md). Force feedback stays in [M-0102](phase-4/M-0102.md).*

- [ ] **🧬 OMSI Event System Research and Consolidation** — [M-0118](phase-11/M-0118.md) — 📝 proposed 2026-10-18
  - [ ] Event taxonomy, dispatch, base event set, parameter formats, ordering, then a consolidation design (documents).
- [ ] **🔁 Dev Agent Roadmap Review Process** — [M-0119](phase-11/M-0119.md) — 🟡 delivered (`2c7a03a`), dry run 2026-10-20, awaiting the project lead's review
  - [x] Prompt P-1010, CONTRIBUTING note, workflow reference and cadence (process).
- [ ] **🧰 QoL Suite** — [M-0109](phase-11/M-0109.md) — 🔄 UI scale presets, font settings and door camera done; smooth driver view open
  - [x] Door camera (done 2026-10-20).
  - [ ] Smooth driver view (open, waits for the jitter branch).
  - [x] UI scale presets 100/125/150/200 %, UI font and base font size.
- [x] **🗺️ Full-Size In-Game Map and Map Switching** — [M-0108](phase-11/M-0108.md) — ✅ done 2026-10-19
  - [x] Full-size map on F6 (pan, zoom, the whole map, position and heading); builds on the mini map (M-0071).
  - [x] Map switching from the Alt menu (builds on sessions M-0068/M-0069).
- [ ] **🚇 Tunnel Detection and Spatial Zones** — [M-0115](phase-11/M-0115.md) — 📝 proposed 2026-10-18
  - [ ] Tunnel detection and a `SpatialZone` abstraction as the precursor for reverb and light-room ambience.
- [ ] **🎛️ Setvar Runtime Editor and CTI Value Types** — [M-0110](phase-11/M-0110.md) — 🟡 in progress since 2026-10-20
  - [x] Alt-menu setvar editor (2026-10-20); ints, choices and numbers beyond binary exist since M-0078 and are carried by configurations; a per-option "needs respawn" marker is open (Q-SV02).
- [ ] **🚦 AI Traffic Depth** — [M-0111](phase-11/M-0111.md) — 🟡 in progress since 2026-10-24 (audit, density-group routing, yellow by rowdiness, counters, parked obstacles)
  - [ ] Audit, then obstacle braking, conservative lane change and traffic lights with `[rowdy_factor]` on top of M-0095/M-0049.
- [ ] **🧭 Hybrid Tile and Object Streaming** — [M-0114](phase-11/M-0114.md) — 📝 proposed 2026-10-18
  - [ ] Streaming strategy that adapts to the player's speed: tiles at speed, objects when slow.
- [ ] **🌦️ Real Weather Research** — [M-0116](phase-11/M-0116.md) — 📝 proposed 2026-10-18
  - [ ] Providers, licensing, offline policy, mapping to `.owt` presets, location source, go/no-go (research only).
- [ ] **🌧️ Weather Effects on Driving and Exhaust** — [M-0117](phase-11/M-0117.md) — 📝 proposed 2026-10-18
  - [ ] Wet and snow grip, fog visibility hint, exhaust particles.
- [x] **🚏 Scheduled AI Traffic** — [M-0112](phase-11/M-0112.md) — ✅ done 2026-10-23
  - [x] Buses, trams and trains that spawn at the route start, follow the timetable and despawn at the end (needs M-0099/M-0098).
- [ ] **🛤️ Track Switch Interaction** — [M-0113](phase-11/M-0113.md) — 🟡 in progress since 2026-10-24: census, points state, HUD hint, `rail.switch`, tongue animation done; scenario verification and the player's tongue click open
  - [x] Switch census from tiles, switch state, route announcement and lock rule for AI and player.

## 🎨 Phase 12: Visual Fidelity & Advanced Physics
*Goal: Close the visible gap between OPTS and a good OMSI 2 screenshot (dirt, light maps, wet and snowy surfaces, a modern lighting and anti-aliasing pipeline) and make vehicles behave under load (fuel or battery, passengers). Status: planned 2026-10-18; approved by the project lead 2026-10-20, open for implementation. Documents: [phase page](phase-12/README.md), [graphics design brief](../architecture/graphics-overhaul.md). Phase 8 keeps its own open items; M-0122 references them.*

- [ ] **🧼 Material Overlays and Light Maps** — [M-0120](phase-12/M-0120.md) — 📝 proposed 2026-10-18
  - [ ] Audit, then render dirt, rain and condensation overlays from script variables; apply `[nightmapmode]` 0-4; overlay setting.
- [ ] **🌧️ Wetness and Snow on Vehicles and Scenery** — [M-0121](phase-12/M-0121.md) — 📝 proposed 2026-10-18
  - [ ] Per-texture `[moisture]`/`[puddles]`, wet vehicles and scenery from the `Wetness` driver, `snow_visible` and winter variants.
- [ ] **🧍 Passenger Load Physics** — [M-0124](phase-12/M-0124.md) — 🔄 total mass, axle loads, ride height and override done; save field open
  - [ ] Dynamic mass from the passenger count into acceleration, braking, steering effort and ride height; Off/Realistic setting.
- [ ] **⛽ Fuel and Battery State** — [M-0123](phase-12/M-0123.md) — 📝 proposed 2026-10-18
  - [ ] Fuel/battery depletion, torque cut at empty, `tank_percent` to scripts, HUD gauge, Off/Realistic/Accelerated, session save.
- [ ] **✨ Graphics Overhaul** — [M-0122](phase-12/M-0122.md) — 📝 proposed 2026-10-18
  - [ ] PBR audit, ambient occlusion, anti-aliasing decision, restrained bloom; benchmarked on an RTX 3060 at 1080p against the reference ([design brief](../architecture/graphics-overhaul.md)).

---

## 🏗️ Phase 13: Crate Architecture: Legacy vs. Native Separation
*Goal: a strict boundary between legacy OMSI data (`opts-omsi`, a leaf adapter) and native OPTS data (`opts-model`, TOML), with shared types in a base crate so there is no circular dependency; `opts-bevy`, `opts-sim` and `opts-game` never depend on `opts-omsi`. Status: delivered 2026-10-19 (the migration of the consumers to the native model is the follow-up; not started as of 2026-10-25, no milestone yet). Documents: [phase page](phase-13/README.md).*

- [x] **🔎 Data-Model Separation: Audit, Options and ADR-0014** — [M-0131](phase-13/M-0131.md) — ✅ done 2026-10-19
  - [x] Audit of `opts-core` (math, parsers, OMSI data, domain model), three crate-graph options with a recommendation, dependency lists, importer location; checkpoint 1.
- [x] **🏗️ New Crates and Shared Types** — [M-0132](phase-13/M-0132.md) — ✅ done 2026-10-19
  - [x] Scaffolding, workspace members, shared domain types moved with `git mv`, `opts-core` re-exports.
- [x] **📦 Move the OMSI Parsers into `opts-omsi`** — [M-0133](phase-13/M-0133.md) — ✅ done 2026-10-19
  - [x] Format modules moved, corpus tests identical, no engine crate depends on `opts-omsi`.
- [x] **📚 Native Model, Importer, Workspace Wiring and Documentation** — [M-0134](phase-13/M-0134.md) — ✅ done 2026-10-19
  - [x] `opts-model` stubs, importer, workspace updated, ADR-0014 accepted, crate graph and CONTRIBUTING updated, merge `--no-ff`.

---

## 🛠️ Phase 14: Vehicle Overrides & Visual Editor
*Goal: customise vehicles without touching the OMSI installation: swap sound, script and constant sets between vehicles and edit meshes in the game, through a `VehicleProfile` TOML patch layer with compatibility validation. Status: proposed 2026-10-19 by the project lead; design first, a checkpoint before implementation. Documents: [phase page](phase-14/README.md).*

- [ ] **📐 Vehicle Profile Design: Override Schema, Validation and Editor Architecture** — [M-0135](phase-14/M-0135.md) — 📝 proposed 2026-10-19
  - [ ] `vehicle-profiles.md` and `vehicle_override.md`: schema, load sequence, validation of scripts, sounds and meshes, editor architecture; checkpoint.
- [ ] **🧩 Vehicle Profile Schema, Loader and Validation** — [M-0136](phase-14/M-0136.md) — 📝 proposed 2026-10-19
  - [ ] TOML profiles in `~/.local/share/opts/vehicles/`, sound/script/constant/mesh overrides, validation warnings, derived vehicles in the selection.
- [ ] **🛠️ In-Game Visual Mesh Editor** — [M-0137](phase-14/M-0137.md) — 📝 proposed 2026-10-19
  - [ ] Picking, gizmos, catalogue browser, live swap, undo, saving profiles.

---

## ♾️ Phase 999: Refactoring Taskforce (continuous)
*Goal: keep the code navigable by running a structural cleanup (no behaviour change) at the end of every development cycle, before each versioned public release; the phase never closes. Baseline 2026-10-19: 846 `.rs` files, 87 over 500 total lines, 19 over 1,000 ([Phase 999 README](phase-999/README.md)).*

- [x] **📏 Structure Audit Tool and Baseline** — [M-0125](phase-999/M-0125.md) — ✅ done 2026-10-19
  - [x] `scripts/structure_audit.py` (files, functions, modules, crates, standards, tests), the baseline report and a ratchet checked by `local-ci.sh`.
- [x] **🔁 Refactoring Cycle Protocol and Release Gate** — [M-0126](phase-999/M-0126.md) — ✅ done 2026-10-19
  - [x] Cycle template, process prompt P-1015, release-gate checklist in the release process, a dry run.
- [x] **✂️ Cycle 1, Batches 1 and 2: Low- and Medium-Risk Splits** — [M-0127](phase-999/M-0127.md) — ✅ done 2026-10-19
  - [x] Plan, then split the clean and the somewhat tangled oversized files along concerns, API paths kept.
- [x] **🧱 Cycle 1, Batches 3 and 4: High-Risk Splits and Structural Cleanup** — [M-0128](phase-999/M-0128.md) — ✅ done 2026-10-19
  - [x] The hot and widely used files, misplaced files, oversized test files.
- [x] **🧩 Cycle 1, Batch 5: Crate-Level Decisions** — [M-0129](phase-999/M-0129.md) — ✅ done 2026-10-19
  - [x] An ADR per crate merge, split or rename; default keep; execute only what is approved.

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
- [x] Replace bare `.unwrap()` with `?` in `parse_mesh` (`crates/opts-omsi/src/formats/x/parse.rs:388`) (F-001) — no .unwrap() left in crates/opts-omsi/src/formats/x/ (2026-10-25)
- [x] Audit and prune premature stub crates from workspace `members` in root `Cargo.toml` (F-002) — reviewed in M-0129, ADR-0015 keeps every crate
- [ ] Provide `opts_bevy::testing::TestAppBuilder` helper for headless `FixedUpdate` time stepping (F-003)
- [ ] Decompose monolithic parser (1,061 lines) and 546-line `from_text` function in `crates/opts-omsi/src/formats/scenery/sco.rs` (F-004) — file split 2026-10-19 (sco.rs 396 code lines plus 5 submodules, 2838b021); the per-family apply functions are still 100–201 code lines
- [ ] Refactor monolithic parsing functions (>80 lines) in `bus.rs`, `sli.rs`, `global.rs`, `tile.rs`, `model/parse.rs`, `paths.rs` (F-005)
- [x] Split `bus.rs` (583 lines) and `world_parse.rs` (689 lines) to comply with 500-line file limit (F-006) — bus.rs 490 code lines (58fa24ba); world_parse.rs (now crates/opts-omsi/tests/) 393 code lines; both under the 500-line-of-code rule
- [ ] Consolidate entity spawning boilerplate between `spawn` and `spawn_child` in `opts-gui` widgets (F-007)
- [x] Execute link normalization sweep across `docs/src/roadmap/README.md` and meta reports (F-011) — check_links.py: 0 broken links in 519 files
- [ ] Reconcile stale Open Questions in promoted specifications (`o3d.md`, `x.md`, `bus.md`) into Resolved Questions (F-012)
- [x] Integrate automated markdown link checker and mdBook validation step into CI workflow (F-015) — docs-links job in ci.yml (68bb7c6b); mdBook builds in deploy-docs.yml on docs pushes
- [x] Create `rust-toolchain.toml` (pinned to stable) and `.editorconfig` in repository root (F-016) — a54b900d (pinned to 1.97.1, not "stable")
- [x] Add `windows-latest` to GitHub Actions CI test matrix for cross-platform validation (F-017) — windows job in ci.yml (a54b900d, M-0042)
- [ ] Control protocol follow-ups (M-0043): `entity.*`/`resource.*`/`asset.*` via reflection or `bevy_remote`, `debug.*`, event streaming (`opts-cli watch`), vehicle-state snapshot on hot reload — sim.set_time done (opts-game/src/control_environment.rs); entity.*/resource.*/asset.*/debug.* are in the CLI table but answer "not implemented"
- [x] **🔎 Scene Trace** — [M-0087](phase-5/M-0087.md)
  - [x] Opt-in debug mode that records and logs which mesh is placed where (tile, record, file, position, materials, decode quality), with `trace.pick/tile/find` in the control protocol (R-30).

