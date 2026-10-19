# `opts-model`

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-model` |
| **Layer** | L1 · Native model |
| **Kind** | Library (`lib.rs`) |
| **Status** | 🟢 Created 2026-10-19 ([ADR-0014](../src/adr/0014-data-model-separation.md), [M-0132](../src/roadmap/phase-13/M-0132.md)) |

---

## Purpose

The native OPTS content model and its TOML form: what the engine crates are meant to read. Stubs so far: `VehicleDefinition` (names, mass, axles) with a TOML round trip, and the `ContentSource` trait through which the engine will ask for content without knowing where it comes from.

## Dependencies

`opts-types`, `serde`, `toml`, `thiserror`. Must not depend on `opts-omsi`.

## Notes

Grows with the migration of the consumers and with the vehicle profiles of [Phase 14](../src/roadmap/phase-14/README.md).
