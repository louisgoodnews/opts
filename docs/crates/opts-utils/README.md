# `opts-utils`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L0 · Foundation |
| **Role** | member |
| **Kind** | lib |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | ✅ Keep — strictly minimal |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

Tiny helpers that are genuinely needed by several crates in different layers and have no better home.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

- *(none yet)*
- Candidates: small collection extensions, string helpers that are not text-format specific.

## 📦 Dependencies — and why

None — and it should stay that way. Any dependency here is inherited by nearly the whole workspace.

## 🧭 Design Decisions

- `utils` crates tend to become junk drawers. **Rule:** if code relates to encodings, paths, errors, units or I/O, it belongs in `opts-text`, `opts-vfs`, `opts-diagnostics` or `opts-geom` instead.
- If this crate is still empty at the end of Phase 2, delete it.

## 🚧 Current State & Known Issues

- Empty `lib.rs`; no `//!` header.

## 🔮 Future Direction

Shrinks as focused foundation crates appear (`opts-diagnostics`, `opts-text`, `opts-vfs`, `opts-geom`).

## 🔗 Cross-References

- [Proposed Architecture § L0](../architecture.md#l0--foundation)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
