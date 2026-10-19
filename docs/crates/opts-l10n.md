# `opts-l10n`

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-l10n` |
| **Layer** | L1 · Localisation |
| **Kind** | Library (`lib.rs`) |
| **Status** | 🟢 Created 2026-10-19 ([ADR-0014](../src/adr/0014-data-model-separation.md), [M-0132](../src/roadmap/phase-13/M-0132.md)) |

---

## Purpose

Localisation without the engine: the Fluent-subset parser, language packs (folder layout, validation, completeness), the checks and the JSON interchange form, and the built-in message-table list the tools check. Moved out of `opts-core`, with which it had nothing in common but the crate.

## Dependencies

`serde_json`, `directories`.

## Notes

Used by `opts-gui`, `opts-game` and `opts-tools`; see [localisation packs](../src/architecture/localisation-packs.md).
