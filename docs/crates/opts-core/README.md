# `opts-core`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L2 · Model |
| **Role** | api (shared kernel) |
| **Kind** | lib |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | 🔁 Repurpose — domain model kernel; parsers move out |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

The **shared kernel** of the OPTS domain model: typed IDs, asset handles, units and the common error vocabulary that every model, pipeline, simulation and engine crate agrees on. It describes *what OPTS simulates*, not how OMSI stores it.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

- `AssetId`, `AssetPath` and typed IDs (`StopId`, `LineId`, `VehicleId`, …)
- Re-exported units/frames from `opts-geom`
- `CoreError`
- Larger domain types live in `opts-model-*` crates built on top of this one.

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `serde` | Model values are saved, cached and sent over the wire |
| `glam` | Math types (via `opts-geom` once it exists) |
| `thiserror` | Library error type |
| `tracing` | Diagnostics |

Currently also declares `nom`, `binrw`, `petgraph` — these belong to parsers (`opts-fmt-*`) and the pipeline (`opts-discovery`) and should be removed.

## 🧭 Design Decisions

- **No parsers.** Format ASTs live in Formats-layer crates; translation lives in `opts-import-*`. See [ADR-0003](../adr/0003-layered-crate-architecture.md).
- **No Bevy, no I/O.** Engine adapters wrap model types in newtypes to derive Bevy traits.
- Small and stable: almost every crate depends on it, so every change here rebuilds the world.

## 🚧 Current State & Known Issues

- Empty `lib.rs`.
- Description still says *"Core data structures and parsers for OMSI 2 assets"*.
- Coding Standards §3.4–3.6 and the format specs (`Parser Module: opts_core::formats::…`) still assume parsers live here.

## 🔮 Future Direction

Split by bounded context as code arrives: `opts-model-scenery`, `opts-model-vehicle`, `opts-model-transit`, `opts-model-world`.

## 🔗 Cross-References

- [Coding Standards §2.11, §3.4–3.6](https://github.com/louisgoodnews/opts/blob/main/docs/src/contributing/standards.md)
- [Phase 2 roadmap](../../src/roadmap/phase-2/README.md)
- [Proposed Architecture § L2](../architecture.md#l2--model)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
