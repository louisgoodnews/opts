# Summary

[Introduction](introduction.md)
[Vision](vision.md)
[Glossary](glossary.md)

---

# 🗺️ Roadmap

- [Overview](roadmap/README.md)
- [Phase 0: Foundation](roadmap/phase-0/README.md)
  - [M-0000: Foundation & Documentation](roadmap/phase-0/M-0000.md)
- [Phase 1: Format Specification](roadmap/phase-1/README.md)
  - [M-0001: Vehicle Text Formats](roadmap/phase-1/M-0001.md)
  - [M-0002: Binary Mesh Formats](roadmap/phase-1/M-0002.md)
  - [M-0003: World Formats](roadmap/phase-1/M-0003.md)
  - [M-0004: Transit Data](roadmap/phase-1/M-0004.md)
  - [M-0005: Script Formats](roadmap/phase-1/M-0005.md)
  - [M-0006: Community Wiki Cross-Check](roadmap/phase-1/M-0006.md)
  - [M-0007: Documentation Drift Repair](roadmap/phase-1/M-0007.md)
  - [M-0008: Script Reference](roadmap/phase-1/M-0008.md)
  - [M-0009: Repaint System](roadmap/phase-1/M-0009.md)
  - [M-0010: Engine & Environment Configs](roadmap/phase-1/M-0010.md)
  - [M-0011: Passenger Content](roadmap/phase-1/M-0011.md)
  - [M-0012: Chrono and Map Variants](roadmap/phase-1/M-0012.md)
- [Phase 2: Core Parsers](roadmap/phase-2/README.md)
  - [M-0013: Binary Mesh Parsers](roadmap/phase-2/M-0013.md)
  - [M-0014: Vehicle & Operational Parsers](roadmap/phase-2/M-0014.md)
  - [M-0015: World & Scenery Parsers](roadmap/phase-2/M-0015.md)
  - [M-0016: DirectX .x Legacy Mesh Parser](roadmap/phase-2/M-0016.md)
  - [M-0017: Model Configuration Parser](roadmap/phase-2/M-0017.md)
- [Phase 3: Bevy Integration](roadmap/phase-3/README.md)
  - [M-0018: 3D Mesh Loaders](roadmap/phase-3/M-0018.md)
  - [M-0019: Material & Texture Bridge](roadmap/phase-3/M-0019.md)
  - [M-0020: Spatial Coordinate Systems](roadmap/phase-3/M-0020.md)
  - [M-0021: Material Design 3 GUI Widget Library](roadmap/phase-3/M-0021.md)
- [Phase 4: Simulation](roadmap/phase-4/README.md)
  - [M-0022: Vehicle Physics & Kinematics](roadmap/phase-4/M-0022.md)
  - [M-0023: Vehicle ECS Simulation Bridge](roadmap/phase-4/M-0023.md)
  - [M-0024: OMSI Map Tile Grid & Chunk Streaming](roadmap/phase-4/M-0024.md)
  - [M-0025: Terrain Heightmap & Water Mesh Generation](roadmap/phase-4/M-0025.md)
  - [M-0026: OMSI Script Stack VM & Dynamic Variables](roadmap/phase-4/M-0026.md)
  - [M-0027: Vehicle & Environmental Spatial Audio](roadmap/phase-4/M-0027.md)
  - [M-0037: Shipped-Work Reconciliation](roadmap/phase-4/M-0037.md)
  - [M-0038: Scrambled-Mesh Completion](roadmap/phase-4/M-0038.md)
  - [M-0039: Text Textures](roadmap/phase-4/M-0039.md)
  - [M-0045: Script-Driven Animation](roadmap/phase-4/M-0045.md)
  - [M-0046: World Loader Extraction](roadmap/phase-4/M-0046.md)
- [Phase 5: Tooling & Performance](roadmap/phase-5/README.md)
  - [M-0040: Code-Quality Debt](roadmap/phase-5/M-0040.md)
  - [M-0041: Documentation Repair](roadmap/phase-5/M-0041.md)
  - [M-0042: CI & Toolchain Hardening](roadmap/phase-5/M-0042.md)
  - [M-0043: CLI Control Protocol & Hot Reload](roadmap/phase-5/M-0043.md)
  - [M-0044: Bevy Engine Upgrade](roadmap/phase-5/M-0044.md)
- [Phase 6: Modding Ecosystem](roadmap/phase-6/README.md)
- [Phase 7: Release & Distribution](roadmap/phase-7/README.md)

---

# 🏗️ Architecture

- [Overview](architecture/overview.md)
- [Crate Structure](architecture/crate-structure.md)
- [Crate Dependency Graph](architecture/crate-graph.md)
- [Coordinate Systems](architecture/coordinates.md)
- [Text Textures](architecture/text-textures.md)
- [Script VM](architecture/script-vm.md)
- [Asset Pipeline](architecture/asset-pipeline.md)
- [Error Handling](architecture/error-handling.md)
- [Determinism & Replays](architecture/determinism-replays.md)
- [Path Catalogue Schema](architecture/catalogue-schema.md)
- [Vehicle Playability Rules](architecture/playability-rules.md)
- [Ego Perspective & Character Locomotion](architecture/ego-perspective.md)
- [Camera System](architecture/camera-system.md)
- [GUI Design System](architecture/gui-design-system.md)
- [Vehicle ECS Simulation Bridge](architecture/vehicle-ecs-bridge.md)
- [Main Menu Architecture](architecture/main-menu.md)
- [Application Startup & Wiring](architecture/application-startup.md)
- [F1 Reporting Feature](architecture/reporting-feature.md)
- [Control Protocol](architecture/control-protocol.md)
- [Hot Reload](architecture/hot-reload.md)


---

# 📄 OMSI Formats

- [Overview](formats/overview.md)
- [Text Formats](formats/text/README.md)
  - [`.sco` — Scenery Objects](formats/text/sco.md)
  - [`.sli` — Splines](formats/text/sli.md)
  - [`.report` — F1 bug reports](formats/text/report-format.md)
  - [`.sco` / `.sli` Corpus Audit](formats/text/sco-sli-audit.md)
  - [`.bus` — Vehicle Definitions](formats/text/vehicles/bus.md)
  - [`.cfg` — Vehicle Configuration](formats/text/cfg.md)
  - [`model.cfg`](formats/text/vehicles/model_cfg.md)
  - [Passenger Paths (`paths.cfg`)](formats/text/vehicles/paths_cfg.md)
  - [Passenger Cabin](formats/text/vehicles/passengercabin_cfg.md)
  - [Sound Configuration](formats/text/vehicles/sound_cfg.md)
  - [`.osc` — Scripts](formats/text/vehicles/osc.md)
  - [Variable Lists](formats/text/vehicles/varnamelist.md)
  - [Constant Files](formats/text/vehicles/const.md)
  - [Predefined Variables & Callbacks](formats/text/vehicles/predefined-variables.md)
  - [Vehicle Script Sets](formats/text/vehicles/script-sets.md)
  - [Scenery Scripts](formats/text/scenery-scripts.md)
  - [Repaints (`.cti`, `[CTC]`)](formats/text/repaints.md)
  - [Environment, Weather & Scenarios](formats/text/environment.md)
  - [Engine Configuration (inputs, options)](formats/text/engine-config.md)
  - [`.opl` — Plug-in Declarations](formats/text/opl.md)
  - [Ticket Packs](formats/text/ticketpacks.md)
  - [Money Systems](formats/text/money.md)
  - [Fonts](formats/text/fonts.md)
  - [`.hof` — Bus Stops & Routes](formats/text/vehicles/hof.md)
  - [`.org` — Number Lists](formats/text/org.md)
  - [`.hum` — Human Characters](formats/text/humans.md)
  - [Misc. Extension Types](formats/text/misc-types.md)
  - [Vehicle-side Corpus Audit](formats/text/vehicles/vehicle-corpus-audit.md)
- [Binary Formats](formats/binary/README.md)
  - [`.o3d` — 3D Meshes](formats/binary/o3d.md)
  - [Chunk Reference](formats/binary/o3d-chunks.md)
  - [`.x` — DirectX Meshes (Legacy)](formats/binary/x.md)
- [Map & Scenario Files](formats/map/README.md)
  - [`global.cfg`, `laststn.osn`, `.dsc`](formats/map/global-cfg.md)
  - [Map Lists & Calendars](formats/map/lists.md)
  - [Timetable Files](formats/map/timetables.md)
  - [Chrono Overlays & Map Variants](formats/map/chrono.md)
  - [Map Tiles](formats/map/tiles.md)
- [Texture Property Files (`*.bmp.cfg`)](formats/text/texture-cfg.md)
- [Texture Formats](formats/textures.md)
- [Community Documentation Cross-Check](formats/community-crosscheck.md)
- [Documentation Gap Analysis](formats/gap-analysis.md)
- [Format Inventory (P-100)](formats/inventory.md)
- [Formats README](formats/README.md)
  - [`.wav` — Sound Files](formats/wav.md)
  - [`.dat` — Star Catalogue](formats/dat.md)
  - [`.xsli` — Spline Project (XML)](formats/xsli.md)
  - [`.itx` — Internet Texture List](formats/itx.md)
  - [`.ipr` — Internet Texture Dates](formats/ipr.md)
  - [`.kyb` — Key-Name Tables](formats/kyb.md)
  - [`.kennz` — Number-Plate Patterns](formats/kennz.md)
  - [`.html` — Tutorial & Menu Pages](formats/html.md)
  - [`.off` — Disabled Texture Lists](formats/off.md)
  - [`.sichtbar` / `.unsichtbar`](formats/sichtbar.md)

---

# 🧑‍💻 Contributing

- [Setup](contributing/setup.md)
- [Coding Standards](contributing/standards.md)
- [Developer Switches](contributing/developer-switches.md)
- [Documentation Standards](contributing/documentation-standards.md)
- [Legal & Licensing](contributing/legal-licensing.md)
- [Release Process](contributing/release-process.md)

---

# 📜 Architecture Decision Records

- [ADR Template](adr/template.md)
- [ADR-0001: Why Bevy and Rust?](adr/0001-why-bevy-and-rust.md)
- [ADR-0002: Clean-Room Reverse Engineering Policy](adr/0002-clean-room-policy.md)
- [ADR-0004: Unified Logging Architecture](adr/0004-logging-architecture.md)
- [ADR-0005: Workspace Crate Architecture](adr/0005-crate-architecture.md)
- [ADR-0006: Control Protocol](adr/0006-control-protocol.md)
- [ADR-0007: Hot Reload](adr/0007-hot-reload.md)

# 🎓 Tutorials

- [Loading Your First `.sco`](tutorials/first-sco.md)
- [Loading Your First Bus Model](tutorials/first-bus.md)

---

# 🚏 Simulation

- [Vehicle Physics & Kinematics](simulation/vehicle_physics.md)

---

# 🔍 Meta & Audits

- [Control Protocol & Hot Reload Report (2026-10-10)](meta/control-protocol-2026-10-10.md)
- [Bug: Spline Winding / Road Hidden by Terrain (2026-10-10)](meta/bugs/f17228d9-3bc0-4830-8f25-3bc9465f63d2_2026-10-10_16-44-31-spline-winding.md)
- [Audit 5037f89a (2026-10-10)](meta/audits/5037f89a-28fb-4408-8196-c2c0e3b3dd8c_2026-10-10_14-59-54.md)
- [F1 Reporting Final Report (2026-10-10)](meta/f1-reporting-2026-10-10.md)
- [Modularization Final Report (2026-10-09)](meta/modularization-2026-10-09.md)
- [Critical Review (2026-10-08)](meta/critical-review-2026-10-08.md)
- [Documentation Freeze (2026-10-06)](meta/documentation-freeze-2026-10-06.md)
- [Encoding Audit (2026-10-06)](meta/encoding-audit-2026-10-06.md)
- [Encoding Repair (2026-10-06)](meta/encoding-repair-2026-10-06.md)
- [Link Integrity Audit](meta/link-integrity.md)
- [Format Status Audit](meta/status-audit.md)
- [Provenance Audit](meta/provenance-audit.md)
- [Roadmap Reconciliation](meta/roadmap-reconciliation.md)
- [Cross-Reference Graph](meta/cross-references.md)

---

<!-- Placeholder sections with no written chapters yet — commented out until implemented -->
<!--
# 🚏 Simulation (Planned)

- [Overview]()
- [Simulation Loop]()
- [Passenger Model]()
- [Routing Algorithm]()
- [Transit Network]()

---

# 🎓 Tutorials

- [Loading a Bus with Textures]()
- [Building a Custom AssetLoader]()
- [Creating a Test Fixture]()

---

# 📘 Reference

- [opts-logging API]()
- [opts-core API]()
- [opts-bevy API]()
- [opts-sim API]()
- [opts-tools CLI]()
- [Error Catalogue]()

---

# 🎮 User Guide

- [Installation]()
- [Controls]()
- [Loading a Map]()
- [Troubleshooting]()
- [FAQ]()

---

# 🛠️ Modding Guide

- [Overview]()
- [Asset Pipeline]()
- [Packaging a Map]()
- [Supported Formats]()
- [Limitations]()
-->
