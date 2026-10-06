# `opts-assets`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L3 · Pipeline |
| **Role** | member |
| **Kind** | lib |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | 🔁 Repurpose — Bevy-free asset catalog |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

The engine-independent **asset catalog**: identity, deduplication, load state and cache keys for every imported asset. Tools use it directly; `opts-bevy-assets` wraps it in Bevy `AssetLoader`s.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

- `AssetCatalog` — `get_or_import(path) -> Arc<T>`, load-state queries
- `AssetKey` (path + content hash)
- Hooks for `opts-cache` (on-disk) and `opts-import-*` (on miss)

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `opts-core` | Asset IDs and handles |
| `tracing` | Diagnostics |

Currently declares `image`, `ddsfile`, `tiny_d3d11tex` — texture decoding belongs to `opts-fmt-texture`. ⚠️ `tiny_d3d11tex` is not declared in `[workspace.dependencies]`, which **breaks the whole workspace**.

## 🧭 Design Decisions

- No Bevy: the catalog must work in `opts-cli` and a headless server.
- Decoding is delegated to Formats/Pipeline crates; the catalog only tracks identity and lifetime.

## 🚧 Current State & Known Issues

- Empty `lib.rs`.
- ⚠️ Undeclared workspace dependency `tiny_d3d11tex` (Audit F1).

## 🔮 Future Direction

Phase 3: backing store for `opts-bevy-assets`; Phase 4: persistent cache via `opts-cache`.

## 🔗 Cross-References

- [Texture formats](../../src/formats/textures.md)
- [Workspace Audit F1](../audit.md)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
