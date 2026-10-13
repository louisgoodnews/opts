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
    end

    %% Layer 1: Domain Core & Parsers
    subgraph L1 ["Layer 1 — Domain Core & Pure Parsers"]
        opts_core["opts-core<br/>• OMSI text format parsers (.sco, .sli, .bus, .cfg)<br/>• Binary mesh parsers (.o3d, .x)<br/>• Map tiles, paths, transit graph models<br/>• Zero Bevy · Zero File I/O"]:::domain
    end

    %% Layer 2: Domain Simulation & Engine Bridges
    subgraph L2 ["Layer 2 — Simulation & Engine Presentation Bridges"]
        opts_sim["opts-sim<br/>• Vehicle physics & kinematics<br/>• Drivetrain, suspension, steering<br/>• Script VM bytecode interpreter<br/>• Zero Bevy · Zero File I/O"]:::bridge
        opts_gui["opts-gui<br/>• Material Design 3 tokens<br/>• Reusable UI widgets<br/>• Glassmorphism layout<br/>• Bevy UI integration"]:::bridge
        opts_bevy["opts-bevy<br/>• 3D mesh loaders & materials<br/>• Coordinate system mapping<br/>• ECS VehicleController bridge<br/>• Scenery object spawning"]:::bridge
    end

    %% Layer 3: Application Executables & Tools
    subgraph L3 ["Layer 3 — Application Executables & Tooling"]
        opts_game["opts-game (Binary)<br/>• Main simulation executable<br/>• AppState orchestrator<br/>• Main menu & UI flow"]:::app
        opts_tools["opts-tools (Binary: opts)<br/>• CLI asset inspector<br/>• Mesh & texture converter<br/>• Dependency & format validator"]:::app
    end

    %% Dependencies
    opts_core -.->|diagnostics| opts_logging
    opts_sim --> opts_core
    opts_sim -.->|diagnostics| opts_logging

    opts_gui -.->|diagnostics| opts_logging

    opts_bevy --> opts_core
    opts_bevy --> opts_sim
    opts_bevy -.->|diagnostics| opts_logging

    opts_game --> opts_core
    opts_game --> opts_sim
    opts_game --> opts_bevy
    opts_game --> opts_gui
    opts_game --> opts_logging

    opts_tools --> opts_core
    opts_tools --> opts_logging
```

---

## 📋 Crate Directory & Inventory

| Crate | Layer | Purpose | Dependencies (Internal) | Depended By | Engine Coupling |
| :--- | :---: | :--- | :--- | :--- | :---: |
| **`opts-logging`** | L0 | Structured diagnostics, subsystem log files, rotation, panic hook | *None* | `opts`, `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-cli`** | L0 | Control-protocol types and the command-line client that remote-controls a running game | *None* | `opts`, `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-core`** | L1 | OMSI format parsers and data models | *None* | `opts-sim`, `opts-traffic`, `opts-catalogue`, `opts-gui`, `opts-bevy`, `opts-game`, `opts-tools` | Pure Rust |
| **`opts-sim`** | L2 | Vehicle dynamics, articulated-segment kinematics, pedestrians, OMSI script VM | `opts-core` | `opts-bevy`, `opts-game` | Pure Rust |
| **`opts-traffic`** | L2 | AI road traffic: lane graph, car following, conflict zones, right of way | `opts-core` | `opts-game` | Pure Rust |
| **`opts-catalogue`** | L2 | Scans an installation into a map/vehicle catalogue | `opts-core` | `opts-game` | Pure Rust |
| **`opts-gui`** | L2 | Material Design 3 UI widget library and theme tokens | `opts-core` | `opts-bevy`, `opts-game` | Bevy UI |
| **`opts-hardware`** | L0 | Privacy-respecting hardware capability probe and default recommendations ([charter](hardware-probe.md)): no network, no identifiers, Linux/macOS/Windows | *None* (external: `sysinfo`) | `opts-game`, `opts-tools` | Pure Rust |
| **`opts-bevy`** | L2 | Bevy loaders, materials, coordinates, vehicle/script/articulation bridges, control server, hot reload | `opts-core`, `opts-sim`, `opts-gui`, `opts-cli`, `opts-logging` | `opts-game` | Bevy Engine |
| **`opts-game`** | L3 | Game application: menus, world loading (tiles, traffic, pedestrians), state machine | `opts-core`, `opts-sim`, `opts-traffic`, `opts-catalogue`, `opts-gui`, `opts-bevy`, `opts-cli`, `opts-logging`, `opts-hardware` | `opts` | Bevy Engine |
| **`opts`** | L3 | Workspace root binary | `opts-game`, `opts-cli`, `opts-logging` | *None (executable root)* | — |
| **`opts-tools`** | L3 | File inspection tool (`inspect`) | `opts-core` | *None (executable root)* | CLI Tooling |

*Table regenerated from `cargo metadata` (normal dependencies) on 2026-10-11 by the [critical review](../meta/critical-review-2026-10-11.md) (F-011). Two edges break the intended layering: `opts-bevy` → `opts-gui` (reporting modal) and `opts-bevy` → `opts-cli` (audit 5037f89a A-F-015, M-0046). The Mermaid diagram above has not been updated yet (M-0041).*
---

## 🔒 Invariant Boundary Rules

1. **`opts-core` Zero-Engine & Zero-IO Invariant:**
   - Must never depend on `bevy`, `wgpu`, or windowing crates.
   - Must never perform synchronous file I/O (`std::fs::File`, `std::io::Read`). All parsers operate strictly on in-memory slices (`&str`, `&[u8]`).
2. **`opts-sim` Zero-Engine & Zero-IO Invariant:**
   - Must never depend on `bevy` or rendering crates.
   - Simulation logic must remain 100% deterministic and headless-runnable.
3. **`opts-logging` Zero-OPTS Invariant:**
   - Must never depend on any OPTS internal crate (`opts-core`, `opts-sim`, etc.).
   - Provides pure tracing layers, rotation policies, and file writers.
4. **Engine Access Gating:**
   - Only `opts-bevy`, `opts-gui`, and `opts-game` are permitted to declare `bevy` as a dependency.
