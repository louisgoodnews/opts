# `opts-types`

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-types` |
| **Layer** | L1 · Shared types |
| **Kind** | Library (`lib.rs`) |
| **Status** | 🟢 Created 2026-10-19 ([ADR-0014](../src/adr/0014-data-model-separation.md), [M-0132](../src/roadmap/phase-13/M-0132.md)) |

---

## Purpose

The geometry and coordinate types the legacy readers and the native model both need, so neither depends on the other. Today: `TileGrid` (tile layout, the Mercator rows of `[worldcoordinates]` maps, offsets and absolute/local transforms), `GridFit` and the `BorderSpline` trait through which a grid is measured from spline records without knowing their file format.

## Dependencies

`glam` only.

## Notes

`opts-omsi` implements `BorderSpline` for its spline record. Ids and error kinds join the crate when the native model needs them.
