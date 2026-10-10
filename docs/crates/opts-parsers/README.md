# `opts-parsers`

> ⚠️ **Deprecated (2026-10-11):** this crate does not exist in the workspace; the page is kept as design history. See the [catalog](../catalog.md).

| Metadata | Value |
| :--- | :--- |
| **Layer** | L1 · Formats |
| **Role** | facade |
| **Kind** | lib |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | 🔁 Rename → `opts-formats` (facade over `opts-fmt-*`) |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

One import point for all OMSI (and later native) format parsers. It re-exports the `opts-fmt-*` family crates behind features and builds a default `FormatRegistry` that maps file extensions to parsers.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

```rust
let registry = opts_formats::registry();          // all enabled formats
let parsed = opts_formats::scenery::sco::parse(bytes, &Default::default());
for d in parsed.diagnostics.iter() { /* … */ }
```

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `opts-fmt-*` | The actual parsers (one crate per format family) |
| `opts-fmt-api` | `Format` trait and registry |

Currently depends on **`bevy`** and `opts-assets` — both must go: parsers are pure functions over bytes (Standards §3.4) and sit *below* the asset pipeline.

## 🧭 Design Decisions

- Facade only: no parsing code lives here (rule R4).
- Features mirror format families: `scenery`, `vehicle`, `mesh`, `texture`, `map`, `transit`, `script`, `config`, `full`.
- Each `opts-fmt-*` crate follows the three-layer parser pattern (lexer → parser → validate) from Standards §3.5.

## 🚧 Current State & Known Issues

- Empty `lib.rs`.
- ⚠️ Depends on Bevy (violates Standards §2.11 and rule R2).
- Overlaps with the parser role currently assigned to `opts-core`.

## 🔮 Future Direction

Phase 2: `opts-fmt-scenery` (`.sco`, `.sli`), `opts-fmt-mesh` (`.o3d`), `opts-fmt-transit` (`.hof`) first — the four formats blocking Phase 2.

## 🔗 Cross-References

- [Format overview](../../src/formats/overview.md)
- [`.sco` spec](../../src/formats/text/sco.md) · [`.o3d` spec](../../src/formats/binary/o3d.md)
- [Crate Catalog § L1](../catalog.md#l1--formats)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
