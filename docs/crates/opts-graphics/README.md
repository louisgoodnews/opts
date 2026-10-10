# `opts-graphics`

> ⚠️ **Deprecated (2026-10-11):** this crate does not exist in the workspace; the page is kept as design history. See the [catalog](../catalog.md).

| Metadata | Value |
| :--- | :--- |
| **Layer** | L5 · Engine |
| **Role** | member |
| **Kind** | bin ⚠️ (should be lib) |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | 🔁 Convert to library, rename → `opts-bevy-render` |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

Rendering of OMSI content in Bevy: emulation of OMSI material semantics (alpha modes, env maps, night/light maps, transparency ordering), lighting and LOD.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

- `OptsRenderPlugin`
- Custom materials (`OmsiMaterial`) and the conversion from model materials

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `bevy` | Rendering |
| `opts-bevy-core` *(planned)* | System sets and states |
| `opts-core` / `opts-model-scenery` | Material and mesh descriptions |

Currently depends on `opts-sim` and `opts-utils` (not needed for rendering) and on `anyhow`/`tracing-subscriber` (binary-only concerns).

## 🧭 Design Decisions

- A *library* exposing one plugin; applications decide whether to use it.
- Never reads files — receives assets from `opts-bevy-assets`.

## 🚧 Current State & Known Issues

- `src/main.rs` with `Hello, world!`.
- ⚠️ Declares `[[bin]] name = "opts-game"`, colliding with the `opts-game` binary (Audit F3).
- ⚠️ Depends on Bevy under the old rule (accepted under the proposed rule R2 once renamed).

## 🔮 Future Direction

Phase 3: first textured `.sco` on screen.

## 🔗 Cross-References

- [`.o3d` spec](../../src/formats/binary/o3d.md) · [Texture property files](../../src/formats/text/texture-cfg.md)
- [Workspace Audit F3, F4](../audit.md)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
