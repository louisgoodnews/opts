# 🚌 OPTS (Open Public Transport Simulator) Roadmap

> 📌 This file mirrors [`docs/src/roadmap/README.md`](docs/src/roadmap/README.md), which is canonical (and is what the book publishes). Update the canonical file first, then re-mirror. Per-phase folders with milestones: [`docs/src/roadmap/`](docs/src/roadmap/).

**Vision:** A free, open-source public transport simulator built in Rust and Bevy, capable of natively loading and simulating OMSI 2 assets.

**Core Philosophy:** 📖 *Documentation-Driven Development.* We write the specifications, format definitions, and architecture docs before writing the implementation code. The documentation is the source of truth.

---

## 🧭 The Phases

| Phase | Goal | Status | Folder |
| :--- | :--- | :--- | :--- |
| 🏗️ **Phase 0** — Foundation & Documentation Infrastructure | skeleton, workspace, governance, docs pipeline | 🟡 nearly complete (hosted Pages deployment unverified) | [`phase-0/`](docs/src/roadmap/phase-0/README.md) |
| 📝 **Phase 1** — OMSI Format Specification | document every OMSI 2 format before writing parsers | 🟡 **current** — all specs drafted; mutation tests and review pending | [`phase-1/`](docs/src/roadmap/phase-1/README.md) |
| ⚙️ **Phase 2** — `opts-core` Implementation | parsers and data model | ⏳ not started | [`phase-2/`](docs/src/roadmap/phase-2/README.md) |
| 🎨 **Phase 3** — `opts-bevy` Integration | loaders, coordinates, rendering | ⏳ not started | [`phase-3/`](docs/src/roadmap/phase-3/README.md) |
| 🚌 **Phase 4** — `opts-sim` & `opts-game` | simulation and the game binary | ⏳ not started | [`phase-4/`](docs/src/roadmap/phase-4/README.md) |
| 🚀 **Phase 5** — Tooling, Polish & Community | CLI, user/modding docs, release | ⏳ not started | [`phase-5/`](docs/src/roadmap/phase-5/README.md) |

Each phase folder holds a README with that phase's checklist (mirrored from the sections below, which are canonical) and its milestone files.

---

## 🏗️ Phase 0: Foundation & Documentation Infrastructure
*Goal: Establish the project skeleton, workspace, and documentation pipeline.*

*Status 2026-10-04: all items done except the hosted GitHub Pages deployment, which cannot be verified from the working tree. ADR-0002 still carries a `YYYY-MM-DD` placeholder date.*

- [x] **📦 Workspace Setup** *(`cargo check --workspace --offline` passes, verified 2026-10-04)*
  - [x] Initialize Cargo workspace with `opts-core`, `opts-bevy`, `opts-sim`, `opts-tools`, `opts-utils`, and `opts-game`.
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
- [~] **🧪 Mutation-Test Round (needs OMSI)** — binary mesh round complete (2026-10-05)
  - [x] Run `CHECKLIST.md` (now also covers the axis prediction in `o3d.md` Addendum 2: Y up, Z longitudinal); folded results into `o3d.md` (Q-004 material groups, Q-005 axes/winding/UV, Q-006 matrix, Q-001–003 header) and `x.md` (Q-X01 frame matrix, Q-X02, Q-X07, Q-X08). Both promoted to 🟢 Specified.
  - [ ] Design a second kit for the map side: `[spline]` / `[object]` field meanings (Q-M02, Q-M03), `[groundtex]` numbers (Q-G01), timetable profiles (Q-T01, Q-T02), misspelled collision keywords (Q-AUD-005).
- [x] **🖼️ Texture Formats** — drafted
  - [x] [`formats/textures.md`](docs/src/formats/textures.md): `.dds` / `.bmp` / `.tga` / `.png` / `.jpg` usage over 202,000 files (A8 ground masks, extension-vs-content mismatches, `.dds` name fallback, RLE TGA, V4/V5 BMP headers). Alpha semantics (Q-TX03), name resolution (Q-TX01) and UV/origin (Q-TX07) await mutation tests.
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
- [ ] **🗺️ World & Scenery Parsers (Planned)** — [M-0015](docs/src/roadmap/phase-2/M-0015.md)
  - [ ] Deferred pending spec promotion from Mutation Kit 2 (`.sco`, `.sli`, `tile_*.map`, `global.cfg`)


---

## 🎨 Phase 3: `opts-bevy` Integration (The Visuals)
*Goal: Bridge the gap between `opts-core` data and Bevy's ECS and rendering pipeline.*

- [ ] **📥 Bevy Asset Loaders**
  - [ ] Implement custom `AssetLoader` for `.o3d` files.
  - [ ] Implement custom `AssetLoader` for `.x` files (discovered in M-0016).
  - [ ] Implement custom `AssetLoader` for `.sco` and `.sli` files.
  - [ ] Register the loaders with the `AssetServer`.
- [ ] **🔄 Mesh Conversion Bridge** — Convert `opts_core::formats::o3d::O3d` and vertex streams to Bevy `Mesh` primitives (discovered in M-0013)
- [ ] **🔄 Coordinate Conversion**
  - [ ] Write systems to convert OMSI Z-up coordinates to Bevy Y-up coordinates.
- [ ] **🌟 Rendering OMSI Assets**
  - [ ] Create a Bevy `Material` from parsed OMSI texture files (`.dds`).
  - [ ] Spawn parsed `.sco` objects as Bevy entities with `Mesh` and `Transform` components.
- [ ] **🎓 Tutorials & Examples**
  - [ ] Write `docs/src/tutorials/first-sco.md`: "Loading a single OMSI scenery object in Bevy."
  - [ ] Write `docs/src/tutorials/first-bus.md`: "Loading a full OMSI bus model with textures."
  - [ ] Create runnable `examples/` in the `opts-bevy` crate matching the tutorials.

---

## 🚌 Phase 4: `opts-sim` & `opts-game` (The Simulation)
*Goal: Build the actual public transport simulation logic and the main executable.*

- [ ] **🗺️ World & Map Loading**
  - [ ] Document the architecture of the map streaming system.
  - [ ] Implement systems in `opts-game` to load a full OMSI map tile.
- [ ] **🛞 Vehicle Physics & Control**
  - [ ] Document the vehicle physics model in `docs/src/simulation/vehicle_physics.md`.
  - [ ] Implement kinematic/dynamic vehicle movement in `opts-sim`.
  - [ ] Map OMSI script variables (`.osc`) to Bevy systems for lights, doors, and indicators.
- [ ] **🧑‍🤝‍🧑 Passenger & Routing System**
  - [ ] Document the passenger simulation model in `docs/src/simulation/passengers.md`.
  - [ ] Implement bus stop logic and passenger boarding/alighting.
  - [ ] Implement a basic routing algorithm (e.g., RAPTOR or Dijkstra via `petgraph`).
- [ ] **🎮 The Game Binary**
  - [ ] Assemble `opts-game/src/main.rs` by adding the `opts-bevy` and `opts-sim` plugins to the Bevy `App`.
  - [ ] Implement the main menu, loading screens, and basic HUD.

---

## 🚀 Phase 5: Tooling, Polish & Community (The Ecosystem)
*Goal: Make OPTS accessible to users and modders, and prepare for public release.*

- [ ] **🧰 `opts-tools` CLI**
  - [ ] Document the CLI usage in `docs/src/reference/opts-tools.md`.
  - [ ] Implement `opts inspect <file>` to validate OMSI assets.
  - [ ] Implement `opts convert <file> --output <gltf>` to export OMSI assets to standard formats.
- [ ] **📘 User & Modding Docs**
  - [ ] Write `docs/src/user/installation.md` and `docs/src/user/controls.md`.
  - [ ] Write `docs/src/modding/asset_pipeline.md`: How modders can structure assets for OPTS.
- [ ] **⚡ Performance & Optimization**
  - [ ] Document LOD (Level of Detail), frustum culling, and occlusion culling strategies.
  - [ ] Implement async asset loading to prevent frame drops.
- [ ] **📦 Release**
  - [ ] Set up cross-platform CI builds (Windows, Linux, macOS).
  - [ ] Publish v0.1.0-alpha to GitHub Releases.

---

## 🧹 Deferred Cleanup

*Small items discovered while closing M-0006; none warrants a milestone.*

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
- [ ] Align the CI/Pages branch (`main`) with the repository's actual default branch (`master` locally) before the first push (M-0007)
- [x] Remove or replace the 0-byte `docs/crates/book.toml` (populated in M-0007)
- [ ] Install `mdbook` and run `mdbook build docs` once to confirm the book builds with `create-missing = false` (M-0007)
