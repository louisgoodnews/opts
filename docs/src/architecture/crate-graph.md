# 📦 Crate Architecture & Dependency Graph

This document defines the architectural layers, crate boundaries, and dependency relationships across the OPTS workspace per [ADR-0005](../adr/0005-crate-architecture.md).

---

## 🏛️ Architecture Layers

The OPTS workspace enforces a strict, unidirectional 4-tier layering model. No upward dependencies or dependency cycles are permitted.

```mermaid
graph TD
    classDef foundation fill:#2d3748,stroke:#4a5568,stroke-width:2px,color:#fff;
    classDef domain fill:#1a365d,stroke:#2b6cb0,stroke-width:2px,color:#fff;
    classDef bridge fill:#234e52,stroke:#319795,stroke-width:2px,color:#fff;
    classDef app fill:#44337a,stroke:#805ad5,stroke-width:2px,color:#fff;

    %% Layer 0: Foundations
    subgraph L0 ["Layer 0 — Foundations & Diagnostics"]
        opts_logging["opts-logging<br/>• Non-blocking tracing writers<br/>• Subsystem file routing<br/>• Panic hook & crash telemetry"]:::foundation
        opts_cli["opts-cli<br/>• Control-protocol types<br/>• Command-line client<br/>• Zero Bevy"]:::foundation
        opts_hardware["opts-hardware<br/>• Privacy-respecting hardware probe<br/>• Default recommendations<br/>• No network, no identifiers"]:::foundation
    end

    %% Layer 1: Domain Core & Parsers
    subgraph L1 ["Layer 1 — Shared Types, Native Model, Legacy Readers"]
        opts_types["opts-types<br/>• Shared geometry & tile-grid maths<br/>• No file formats<br/>• Zero Bevy"]:::domain
        opts_l10n["opts-l10n<br/>• Fluent-subset parser, language packs<br/>• Checks & JSON interchange<br/>• No OMSI content"]:::domain
        opts_omsi["opts-omsi<br/>• OMSI text format parsers (.sco, .sli, .bus, .cfg)<br/>• Binary mesh parsers (.o3d, .x)<br/>• Map tiles, paths, transit graph models<br/>• Leaf adapter · Zero Bevy · Zero File I/O"]:::domain
        opts_model["opts-model<br/>• Native content model & TOML form<br/>• ContentSource trait<br/>• No OMSI knowledge"]:::domain
        opts_importer["opts-importer<br/>• opts-omsi to opts-model<br/>• The only crate that knows both"]:::domain
        opts_core["opts-core (temporary facade)<br/>• Re-exports opts-omsi and opts-l10n at the old paths<br/>• Removed when the last consumer is migrated"]:::domain
    end

    %% Layer 2: Domain Simulation & Engine Bridges
    subgraph L2 ["Layer 2 — Simulation & Engine Presentation Bridges"]
        opts_sim["opts-sim<br/>• Vehicle physics & kinematics<br/>• Drivetrain, suspension, steering<br/>• Script VM bytecode interpreter<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_traffic["opts-traffic<br/>• Lane graph & car following<br/>• Conflict zones, right of way<br/>• Zero Bevy"]:::bridge
        opts_catalogue["opts-catalogue<br/>• Installation scan into a catalogue<br/>• Map & vehicle lists"]:::bridge
        opts_gui["opts-gui<br/>• Material Design 3 tokens<br/>• Reusable UI widgets<br/>• Glassmorphism layout<br/>• Bevy UI integration"]:::bridge
        opts_bevy["opts-bevy<br/>• 3D mesh loaders & materials<br/>• Coordinate system mapping<br/>• ECS VehicleController bridge<br/>• Scenery object spawning"]:::bridge
    end

    %% Layer 3: Application Executables & Tools
    subgraph L3 ["Layer 3 — Application Executables & Tooling"]
        opts_game["opts-game (Binary)<br/>• Main simulation executable<br/>• AppState orchestrator<br/>• Main menu & UI flow"]:::app
        opts_benchmark["opts-benchmark (Binary)<br/>• Scenery load verification<br/>• Off-screen worker & supervisor"]:::app
        opts_tools["opts-tools (Binary: opts)<br/>• CLI asset inspector<br/>• Mesh & texture converter<br/>• Dependency & format validator"]:::app
    end

    %% Dependencies
    opts_benchmark --> opts_bevy
    opts_benchmark --> opts_catalogue
    opts_benchmark --> opts_core
    opts_benchmark --> opts_game
    opts_benchmark --> opts_sim
    opts_bevy --> opts_cli
    opts_bevy --> opts_core
    opts_bevy --> opts_gui
    opts_bevy -.->|diagnostics| opts_logging
    opts_bevy --> opts_sim
    opts_bevy --> opts_traffic
    opts_catalogue --> opts_core
    opts_core --> opts_l10n
    opts_core --> opts_omsi
    opts_game --> opts_bevy
    opts_game --> opts_catalogue
    opts_game --> opts_cli
    opts_game --> opts_core
    opts_game --> opts_gui
    opts_game --> opts_hardware
    opts_game --> opts_l10n
    opts_game -.->|diagnostics| opts_logging
    opts_game --> opts_sim
    opts_game --> opts_traffic
    opts_gui --> opts_l10n
    opts_importer --> opts_model
    opts_importer --> opts_omsi
    opts_model --> opts_types
    opts_omsi --> opts_types
    opts_sim --> opts_core
    opts_tools --> opts_core
    opts_tools --> opts_hardware
    opts_tools --> opts_l10n
    opts_traffic --> opts_core
```

---

## 📋 Crate Directory & Inventory

| Crate | Layer | Purpose | Dependencies (Internal) | Depended By | Engine Coupling |
| :--- | :---: | :--- | :--- | :--- | :---: |
| **`opts-logging`** | L0 | Structured diagnostics, subsystem log files, rotation, panic hook | *None* | `opts`, `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-cli`** | L0 | Control-protocol types and the command-line client that remote-controls a running game | *None* | `opts`, `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-types`** | L1 | Shared geometry and tile-grid maths both the legacy readers and the native model need ([ADR-0014](../adr/0014-data-model-separation.md)) | *None* | `opts-model`, `opts-omsi` | Pure Rust |
| **`opts-l10n`** | L1 | Localisation without the engine: Fluent-subset parser, language packs, checks, JSON interchange | *None* | `opts-core`, `opts-game`, `opts-gui`, `opts-tools` | Pure Rust |
| **`opts-omsi`** | L1 | The legacy OMSI readers (`.bus`, `.sco`, `.sli`, `.map`, `.o3d`, `.x`, ...) and the OMSI-shaped data they fill: a leaf adapter | `opts-types` | `opts-core`, `opts-importer` | Pure Rust |
| **`opts-model`** | L1 | The native content model and its TOML form (stubs: `VehicleDefinition`, `ContentSource`) | `opts-types` | `opts-importer` | Pure Rust |
| **`opts-importer`** | L1 | Turns legacy OMSI content into the native model; the only crate that knows both | `opts-model`, `opts-omsi` | *None yet* | Pure Rust |
| **`opts-core`** | L1 | Temporary facade: re-exports `opts-omsi` and `opts-l10n` at the old paths until the last consumer is migrated | `opts-l10n`, `opts-omsi` | `opts-benchmark`, `opts-bevy`, `opts-catalogue`, `opts-game`, `opts-sim`, `opts-tools`, `opts-traffic` | Pure Rust |
| **`opts-sim`** | L2 | Vehicle dynamics, articulated-segment kinematics, pedestrians, OMSI script VM | `opts-core` | `opts-benchmark`, `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-traffic`** | L2 | AI road traffic: lane graph, car following, conflict zones, right of way | `opts-core` | `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-catalogue`** | L2 | Scans an installation into a map/vehicle catalogue | `opts-core` | `opts-benchmark`, `opts-game` | Pure Rust |
| **`opts-gui`** | L2 | Material Design 3 UI widget library and theme tokens | `opts-l10n` | `opts-bevy`, `opts-game` | Bevy UI |
| **`opts-hardware`** | L0 | Privacy-respecting hardware capability probe and default recommendations ([charter](hardware-probe.md)): no network, no identifiers, Linux/macOS/Windows | *None* (external: `sysinfo`) | `opts-game`, `opts-tools` | Pure Rust |
| **`opts-bevy`** | L2 | Bevy loaders, materials, coordinates, vehicle/script/articulation bridges, control server, hot reload | `opts-cli`, `opts-core`, `opts-gui`, `opts-logging`, `opts-sim`, `opts-traffic` | `opts-benchmark`, `opts-game` | Bevy Engine |
| **`opts-game`** | L3 | Game application: menus, world loading (tiles, traffic, pedestrians), state machine | `opts-bevy`, `opts-catalogue`, `opts-cli`, `opts-core`, `opts-gui`, `opts-hardware`, `opts-l10n`, `opts-logging`, `opts-sim`, `opts-traffic` | `opts`, `opts-benchmark` | Bevy Engine |
| **`opts`** | L3 | Workspace root binary | `opts-game`, `opts-cli`, `opts-logging` | *None (executable root)* | — |
| **`opts-tools`** | L3 | File inspection tool (`inspect`) | `opts-core`, `opts-hardware`, `opts-l10n` | *None (executable root)* | CLI Tooling |
| **`opts-benchmark`** | L3 | Scenery object load verification harness ([spec](scenery-benchmark.md)): an off-screen worker that rebuilds objects through the loader and a supervisor | `opts-bevy`, `opts-catalogue`, `opts-core`, `opts-game`, `opts-sim` | *None (executable root)* | Bevy |

*Table regenerated from `cargo metadata` (normal dependencies) on 2026-10-11 by the [critical review](../meta/critical-review-2026-10-11.md) (F-011). Two edges break the intended layering: `opts-bevy` → `opts-gui` (reporting modal) and `opts-bevy` → `opts-cli` (audit 5037f89a A-F-015, M-0046). The Mermaid diagram above has not been updated yet (M-0041).*
---

## 🔒 Invariant Boundary Rules

1. **`opts-omsi` Zero-Engine & Zero-IO Invariant, and the legacy/native boundary ([ADR-0014](../adr/0014-data-model-separation.md)):**
   - Must never depend on `bevy`, `wgpu`, or windowing crates, and never on `opts-model`.
   - Must never perform synchronous file I/O (`std::fs::File`, `std::io::Read`). All parsers operate strictly on in-memory slices (`&str`, `&[u8]`).
   - It is a **leaf adapter**: `opts-bevy`, `opts-sim`, `opts-traffic`, `opts-gui` and `opts-catalogue` must not depend on it (directly or through the `opts-core` facade) in new code. The files that still do are counted by `scripts/omsi_uses.py` against `docs/dev/omsi-uses-known.txt`; the count may only fall. `opts-game` is the composition root and may wire the OMSI source through `opts-importer`.
   - `opts-model` and `opts-omsi` share only `opts-types` and never import each other; `opts-importer` is the only crate that depends on both.
2. **`opts-sim` Zero-Engine & Zero-IO Invariant:**
   - Must never depend on `bevy` or rendering crates.
   - Simulation logic must remain 100% deterministic and headless-runnable.
3. **`opts-logging` Zero-OPTS Invariant:**
   - Must never depend on any OPTS internal crate (`opts-omsi`, `opts-sim`, etc.).
   - Provides pure tracing layers, rotation policies, and file writers.
4. **Engine Access Gating:**
   - Only `opts-bevy`, `opts-gui`, and `opts-game` are permitted to declare `bevy` as a dependency.
