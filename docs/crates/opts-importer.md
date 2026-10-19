# `opts-importer`

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-importer` |
| **Layer** | L1 · Importer |
| **Kind** | Library (`lib.rs`) |
| **Status** | 🟢 Created 2026-10-19 ([ADR-0014](../src/adr/0014-data-model-separation.md), [M-0132](../src/roadmap/phase-13/M-0132.md)) |

---

## Purpose

Reads legacy OMSI content with `opts-omsi` and produces `opts-model` values. The only crate that knows both. Skeleton: `import_vehicle(bus bytes)` and `vehicle_from_manifest`.

## Dependencies

`opts-omsi`, `opts-model`.

## Notes

`opts-game` (the composition root) plugs an OMSI-backed `ContentSource` built on it; `opts-tools` can call it for a command-line import.
