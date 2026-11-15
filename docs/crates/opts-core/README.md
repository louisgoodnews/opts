# `opts-core`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L1 · Temporary facade |
| **Kind** | lib (`lib.rs`, re-exports only) |
| **Status** | 🟢 Active — temporary re-export facade |
| **Last Updated** | 2026-11-15 |

---

`opts-core` is the **temporary re-export facade** of the workspace. It contains no code of its own: it re-exports [`opts-omsi`](../opts-omsi.md) (`formats`, `setvars`) and [`opts-l10n`](../opts-l10n.md) (`l10n`) at the paths the old `opts-core` had.

The code that used to live here (the OMSI readers) moved to `opts-omsi` under [ADR-0014](../../src/adr/0014-data-model-separation.md). The facade is removed once the last consumer is migrated to the native model.

For the authoritative description of the crate graph, layers and dependency rules, see [Crate Graph](../../src/architecture/crate-graph.md). The per-crate detail page is [`opts-core`](../opts-core.md).
