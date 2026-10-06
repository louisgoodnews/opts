# `opts-gui`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L5 · Engine |
| **Role** | member |
| **Kind** | lib |
| **Status** | ⬜ Skeleton |
| **Proposed fate** | 🔁 Rename → `opts-bevy-ui` |
| **Last Updated** | 2026-10-04 |

---

## 🎯 Purpose

In-game user interface: menus, HUD, dialogs, settings.

## 📤 Public API (high level)

> [!NOTE]
> Planned surface — nothing is implemented yet.

- `OptsUiPlugin`
- UI state reacting to `opts-bevy-core` states (`Menu`, `Loading`, `Driving`, …)

## 📦 Dependencies — and why

| Dependency | Why |
| :--- | :--- |
| `bevy` | UI rendering |
| `opts-core` | Shared types |
| `opts-assets` | Asset listings for menus (map/vehicle pickers) |

## 🧭 Design Decisions

- Talks to the rest of the game only through events/resources in `opts-bevy-core`.
- Editor UI (Phase 5) is a separate crate so the game does not ship editor code.

## 🚧 Current State & Known Issues

- Empty `lib.rs`.
- ⚠️ Depends on Bevy under the old rule (accepted under rule R2 once renamed).

## 🔮 Future Direction

Optional `egui`-based debug UI moves to `opts-bevy-devtools`.

## 🔗 Cross-References

- [Proposed Architecture § L5](../architecture.md#l5--engine-adapters-bevy)
- [Proposed Architecture](../architecture.md) · [Workspace Audit](../audit.md)
