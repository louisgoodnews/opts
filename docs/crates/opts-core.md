# `opts-core`

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-core` |
| **Layer** | L1 · Temporary facade |
| **Kind** | Library (`lib.rs`) |
| **Status** | 🟢 Created 2026-10-19 ([ADR-0014](../src/adr/0014-data-model-separation.md), [M-0132](../src/roadmap/phase-13/M-0132.md)) |

---

## Purpose

Re-exports `opts-omsi` (`formats`, `setvars`) and `opts-l10n` (`l10n`) at the paths the old `opts-core` had, so the code that was moved did not force a change of every `use` statement. It contains no code of its own.

## Dependencies

`opts-omsi`, `opts-l10n`.

## Notes

The facade is removed when the last consumer is migrated to the native model; `scripts/omsi_uses.py` counts the uses of OMSI types in the engine crates that still go through it (ratchet `docs/dev/omsi-uses-known.txt`). New code names the real crate.
