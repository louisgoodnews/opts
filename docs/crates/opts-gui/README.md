# `opts-gui`

| Metadata | Value |
| :--- | :--- |
| **Layer** | L5 · Engine |
| **Role** | member |
| **Kind** | lib |
| **Status** | 🟢 Active Implementation |
| **Proposed fate** | 🔁 Foundation Complete (`opts-gui`) |
| **Last Updated** | 2026-10-08 |

---

## 🎯 Purpose

Material Design 3 GUI widget library for OPTS. See full documentation in [opts-gui.md](../opts-gui.md).

## 📤 Public API (high level)

- `OptsGuiPlugin`: Root Bevy plugin registering all theme systems, tokens, and widget interaction handlers.
- `MaterialTheme`, `ColorScheme`: Full Material Design 3 design system with WCAG 2.1 AA verified palettes.
- Widget builders: `ButtonBuilder`, `LabelBuilder`, `IconBuilder`, `CardBuilder`, `DividerBuilder`, `TextFieldBuilder`, `CheckboxBuilder`, `SwitchBuilder`, `RadioBuilder`, `ComboboxBuilder`, `NotebookBuilder`, `ToastBuilder`, `ToplevelBuilder`, `TreeviewBuilder`, `DateSelectBuilder`, `DateTimeSelectBuilder`.

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
