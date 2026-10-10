# `opts-benchmark`

> ⚠️ **Deprecated (2026-10-11):** this crate does not exist in the workspace; the page is kept as design history. See the [catalog](../catalog.md).

| Metadata | Value |
| :--- | :--- |
| **Layer** | Development |
| **Role** | dev |
| **Kind** | lib → benches |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | 🔁 Dev-only, `publish = false` |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

Performance benchmarks — parser throughput, texture decoding, import and discovery over a user-supplied OMSI corpus.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

None. Benchmarks run with `cargo bench -p opts-benchmark`; corpus benchmarks are opt-in via an environment variable pointing at a local OMSI installation (no assets are ever committed).

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `criterion` *(planned)* | Statistics-driven benchmarking |
| crates under test | e.g. `opts-formats`, `opts-assets` |

Currently declares `image`, `ddsfile`, `tiny_d3d11tex` as normal deps; ⚠️ `tiny_d3d11tex` is undeclared at workspace level (Audit F1).

## 🧭 Design Decisions

- Never a normal dependency of any crate.
- Benchmarks use clean-room fixtures from `opts-fixtures` by default ([ADR-0002](../../src/adr/0002-clean-room-policy.md)).

## 🚧 Current State & Known Issues

- Empty `lib.rs`.

## 🔮 Future Direction

CI tracks benchmark regressions on fixture corpora.

## 🔗 Cross-References

- [ADR-0002: Clean-Room Policy](../../src/adr/0002-clean-room-policy.md)
- [Workspace Audit F9](../audit.md)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
