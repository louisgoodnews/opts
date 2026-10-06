# `opts-discovery`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L3 · Pipeline |
| **Role** | member |
| **Kind** | lib |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | ✅ Keep |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

Scan an OMSI installation (or any `FileSource`), find every asset, and build a **dependency graph** between them: which map uses which tiles, which tile places which `.sco`, which `.sco` needs which meshes and textures. Reports missing and broken references.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

- `scan(source: &dyn FileSource, registry: &FormatRegistry) -> AssetGraph`
- `AssetGraph` (nodes = assets, edges = typed references) with queries: dependencies, dependents, missing, cycles
- Report types consumable by `opts-tools` (`opts graph`, `opts validate`)

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `petgraph` | The dependency graph |
| `opts-vfs` *(planned)* | Case-insensitive, overlay-aware file access |
| `opts-formats` *(planned)* | `Format::references` extracts outgoing references without discovery knowing any grammar |
| `serde` | Persisting scan results |
| `thiserror`, `tracing` | Errors and diagnostics |

Currently also declares `nom`, `binrw`, `glam` (should come via `opts-formats`) and **`anyhow`** (not allowed in libraries).

## 🧭 Design Decisions

- Discovery does not parse grammars itself; it asks the registry for references. New formats become discoverable automatically.
- Scans are incremental-friendly: nodes are keyed by `AssetPath` + content hash.

## 🚧 Current State & Known Issues

- Empty `lib.rs`.
- ⚠️ `anyhow` in a library (Standards §2.1).

## 🔮 Future Direction

Feeds `opts-cache` invalidation and the editor's "where is this used?" view.

## 🔗 Cross-References

- [Format Inventory](../../src/formats/inventory.md)
- [Workspace Audit F7, F8](../audit.md)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
