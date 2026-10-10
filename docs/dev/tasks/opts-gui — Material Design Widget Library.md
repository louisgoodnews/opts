# 🎨 `opts-gui` — Material Design Widget Library

You are designing and implementing a modular widget library for the
OPTS project, built on Bevy UI and following Material Design 3
principles. This library will be the foundation of OPTS's entire
user interface: menus, settings, vehicle selection, map browsing,
HUD elements, and future editor tools.

This is a two-phase task:

- **Phase A — Design Research & Specification** — produce the design
  system document that defines every token, component, and convention.
- **Phase B — Implementation** — build the `opts-gui` crate that
  implements the design system as reusable Bevy components and plugins.

**Do not proceed to Phase B until Phase A is approved.** A widget
library is a long-lived API surface; a wrong abstraction here
compounds across every screen in the application.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Target crate
`crates/opts-gui/`

If this crate does not exist, it will be created in Phase B.

### Output targets

**Phase A:**
- `docs/src/architecture/gui-design-system.md` — the design system spec

**Phase B:**
- `crates/opts-gui/` — the widget library crate
- `docs/crates/opts-gui.md` — the crate documentation

### Related context (read first)
- `docs/src/contributing/standards.md` — coding standards
- `docs/src/adr/0001-why-bevy-and-rust.md` — Bevy as the engine
- `docs/src/glossary.md` — shared vocabulary
- `CONTRIBUTING.md` — crate boundary rules
- `ROADMAP.md` — where this fits in the project

### External references (read, do not copy)
- Material Design 3 spec: https://m3.material.io/
- Bevy `bevy_ui` examples: https://bevy.org/examples/#ui
- Bevy `bevy_feathers` source (for patterns): check `docs.rs/bevy_feathers`
- `bevy_material_ui` crate (for prior art, not for copying)

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not copy source code from `bevy_material_ui` or any external
  widget library. Read for pattern inspiration only.
- Do not add Bevy as a dependency of `opts-core` or `opts-sim`.
  `opts-gui` sits alongside `opts-bevy`, not beneath it.
- Do not invent a custom rendering system. Use Bevy's `bevy_ui`.
- Do not implement every Material Design component in v1. Focus on
  the foundation and the most-used components.
- Do not write production UI screens in this task. This is the
  library; screens come later.

### ✅ Permitted

- Read the Material Design 3 spec.
- Read Bevy UI source and examples.
- Read `bevy_feathers` for patterns (styling, theming, token
  organization).
- Read `bevy_material_ui` for API shape inspiration.
- Create `crates/opts-gui/`.
- Create `docs/src/architecture/gui-design-system.md`.
- Create `docs/crates/opts-gui.md`.
- Add examples in `crates/opts-gui/examples/`.

---

## 3. Phase A — Design Research & Specification

Phase A produces a single document:
`docs/src/architecture/gui-design-system.md`.

### Stage A1 — Material Design 3 Core Concepts

Research and document the Material Design 3 system. The document
must cover:

1. **Design tokens hierarchy.**
   - Reference tokens (raw values: colors, sizes)
   - System tokens (semantic roles: `primary`, `surface`, `on-primary`)
   - Component tokens (per-component: `button.background.rest`)
   - Explain how OPTS will map this hierarchy into Rust types.

2. **Color system.**
   - 26 standard color roles across 6 groups (primary, secondary,
     tertiary, error, surface, outline).
   - Light and dark themes.
   - The `on-X` convention for contrast pairs.
   - How these map to a Rust `ColorScheme` struct.

3. **Typography scale.**
   - 15 baseline styles (Display, Headline, Title, Body, Label ×
     Large, Medium, Small).
   - Font size, line height, weight, tracking for each.
   - How these map to a `Typography` resource.

4. **Elevation.**
   - 6 levels (0–5).
   - Tonal overlay vs. shadow approach.
   - Which level applies to which component type.

5. **Shape.**
   - Radius levels (none, xs, sm, md, lg, xl, full).
   - Which components use which radius.

6. **Motion.**
   - Duration tokens (short, medium, long).
   - Easing curves (standard, emphasized, decelerated, accelerated).
   - Spring-based physics (M3 Expressive) as a forward-looking
     option.

7. **Spacing.**
   - 4dp base unit.
   - Named spacing tokens.

**Deliverable:** A "Design Tokens" section in the document,
organized as tables. Each token gets a name, a value, and a
short purpose note.

### Stage A2 — Component Inventory

List every Material Design component that OPTS will need. Sort
into three tiers:

| Tier | Criterion | Target Count |
| :--- | :--- | ---: |
| **Tier 1** | Essential for any screen; used everywhere | 8–12 |
| **Tier 2** | Common in specific screens; used often | 8–12 |
| **Tier 3** | Situational; used rarely | 10–15 |

For each component, document:

- Material Design name
- One-line description
- Tier assignment
- Key states (rest, hover, focus, pressed, disabled)
- Variants (e.g., filled/outlined/text for buttons)
- Bevy implementation notes (what components it needs, what
  layout it uses)

**Deliverable:** A "Component Inventory" section with three tiers.

### Stage A3 — Bevy Mapping

Document how the design system maps to Bevy's ECS model.

1. **Theme resource.** A `MaterialTheme` resource holding the
   color scheme, typography, and other tokens.
2. **Token components.** How individual tokens become Bevy
   components or resource fields.
3. **Widget composition.** How a widget (e.g., a button) is
   composed of Bevy entities with specific components.
4. **Plugin architecture.** How `opts-gui` exposes itself as a
   Bevy plugin.
5. **State management.** How widget states (hover, focus) are
   tracked via Bevy components.
6. **Interaction.** How Bevy's input events flow to widgets.

**Deliverable:** A "Bevy Mapping" section with entity/component
diagrams.

### Stage A4 — Theming Strategy

Document how theming works:

1. **Built-in themes.** Light and dark at minimum.
2. **Dynamic color.** Whether OPTS should support Material You's
   dynamic color extraction (from a user-selected image). Likely
   out of scope for v1, but document the path.
3. **Theme switching at runtime.** How the app switches themes
   without restarting.
4. **Custom themes.** How modders or users could supply their
   own theme.
5. **High-contrast and accessibility.** Ensuring the design system
   supports reduced-motion and high-contrast variants.

**Deliverable:** A "Theming" section.

### Stage A5 — Component Anatomy

Pick **one** Tier 1 component (recommendation: the button) and
document its full anatomy as a reference for how other components
will be structured:

1. The entity tree (parent node, child nodes, text node, icon node)
2. The components on each entity
3. The systems that operate on it
4. The states it can be in
5. The theme tokens it consumes
6. The public API (`Button::filled("Save").on_click(...)`)

**Deliverable:** A "Component Anatomy — Button" section.

### Stage A6 — Scope Decision

State clearly what Phase B will and will not implement:

**In Scope for v1:**
- The `MaterialTheme` resource with light and dark schemes
- The token system (colors, typography, spacing)
- Layout primitives (Stack, Row, Grid)
- Tier 1 components (buttons, labels, icons, cards, dividers,
  text fields, checkboxes, switches, radio buttons)
- One complete example screen demonstrating all components
- Bevy plugin that registers everything

**Out of Scope for v1:**
- Tier 2 and Tier 3 components
- Dynamic color extraction
- Material motion physics
- Accessibility features beyond Bevy defaults
- Production UI screens

**Deliverable:** A "Scope" section with In Scope and Out of Scope
lists.

### Stage A7 — Report and Pause

Produce a "Phase A Design Summary" containing:

- The token tables
- The component inventory
- The Bevy mapping summary
- The theming strategy
- The scope decision
- Any design decisions that need user approval

**STOP. Do not proceed to Phase B until the user approves Phase A.**

---

## 4. Phase B — Implementation

Phase B builds the `opts-gui` crate. Do not start until Phase A is
approved.

### Stage B1 — Crate Setup

1. Create `crates/opts-gui/` with a `Cargo.toml` following the
   workspace conventions.
2. Dependencies:
   - `bevy` (via workspace)
   - `thiserror` (for any custom errors)
   - `tracing` (for diagnostics)
   - `serde` (for theme persistence, optional)
3. Add `opts-gui` to the workspace members in the root `Cargo.toml`.
4. Add `opts-gui` to `docs/src/SUMMARY.md`.

### Stage B2 — Module Structure

Create the following module tree:

```
crates/opts-gui/src/
├── lib.rs                    # Crate root, plugin
├── plugin.rs                 # OptsGuiPlugin
├── theme/
│   ├── mod.rs
│   ├── color.rs              # ColorScheme, ColorRole
│   ├── typography.rs         # Typography, TypeStyle
│   ├── spacing.rs            # Spacing tokens
│   ├── shape.rs              # Radius tokens
│   ├── elevation.rs          # Elevation levels
│   ├── motion.rs             # Duration and easing tokens
│   └── theme.rs              # MaterialTheme resource, builder
├── tokens/
│   ├── mod.rs
│   └── colors.rs             # Named color constants
├── components/
│   ├── mod.rs
│   ├── button.rs             # Button widget
│   ├── label.rs              # Text label widget
│   ├── icon.rs               # Icon widget
│   ├── card.rs               # Card container
│   ├── divider.rs            # Divider
│   ├── text_field.rs         # Text input
│   ├── checkbox.rs           # Checkbox
│   ├── switch.rs             # Toggle switch
│   └── radio.rs              # Radio button
├── layout/
│   ├── mod.rs
│   ├── stack.rs              # Vertical stack
│   ├── row.rs                # Horizontal row
│   └── grid.rs               # Grid
├── interaction/
│   ├── mod.rs
│   ├── state.rs              # Widget state components
│   └── focus.rs              # Focus navigation helpers
└── prelude.rs                # Re-exports for ergonomic imports
```

Each module gets a `//!` header per the coding standards.

### Stage B3 — Theme System

Implement the theme system first. Every widget depends on it.

1. **`ColorRole` enum** — one variant per Material Design color role.
2. **`ColorScheme` struct** — a map from role to `Color`.
3. **`MaterialTheme` resource** — holds the color scheme, typography,
   spacing, shape, elevation, and motion tokens.
4. **Light and dark schemes** — implement both following Material
   Design 3's baseline palettes.
5. **Theme builder** — a fluent API for constructing custom themes.
6. **Theme switching** — a system that responds to a `ThemeChanged`
   event and re-applies tokens to all widgets.

**Acceptance criteria:**
- A unit test verifies the light and dark schemes have all 26 roles
  populated.
- A unit test verifies the contrast ratio between `primary` and
  `on-primary` meets WCAG AA (4.5:1).
- A unit test verifies the typography scale has all 15 styles.

### Stage B4 — Token Components

Implement components that apply theme tokens to entities:

1. **`BackgroundRole(ColorRole)`** — sets the background color from
   the theme.
2. **`ForegroundRole(ColorRole)`** — sets text/icon color.
3. **`BorderRole(ColorRole)`** — sets border color.
4. **`TypeStyleToken(TypeStyle)`** — applies a typography style.
5. **`RadiusToken(Radius)`** — applies a corner radius.
6. **`ElevationToken(Elevation)`** — applies elevation styling
   (tonal overlay or shadow).

Systems watch the `MaterialTheme` resource and update components
when it changes.

**Acceptance criteria:**
- A unit test changes the theme and verifies affected entities
  update.
- A doctest shows how to use each token component.

### Stage B5 — Layout Primitives

Implement layout helpers that wrap Bevy's `Node` and `Style`
components:

1. **`Stack`** — vertical flex container with spacing.
2. **`Row`** — horizontal flex container with spacing.
3. **`Grid`** — CSS Grid container with configurable tracks.

Each provides a fluent builder that produces a Bevy entity with
the right `Style` and children.

**Acceptance criteria:**
- A doctest shows how to build a simple layout with each primitive.
- An example binary renders three layouts side by side.

### Stage B6 — Tier 1 Components

Implement each Tier 1 component as a plugin + component bundle +
builder. The reference pattern from Stage A5 applies.

For each component:

1. **Component bundle** — the data that defines the widget.
2. **Marker component** — identifies the widget root for system
   queries.
3. **Builder** — a fluent API for constructing it.
4. **Systems** — behavior: interaction, state changes, theme
   application.
5. **Plugin** — registers the systems and observers.
6. **Doctest** — one example showing usage.
7. **Example binary** — one example demonstrating the component
   in a Bevy app.

Required components for Phase B v1:

| Component | Variants |
| :--- | :--- |
| **Button** | Filled, Outlined, Text, Elevated, Tonal, Icon |
| **Label** | All 15 type styles |
| **Icon** | Sized variants |
| **Card** | Filled, Outlined, Elevated |
| **Divider** | Horizontal, Vertical |
| **TextField** | Filled, Outlined; with/without label |
| **Checkbox** | Checked, Unchecked, Indeterminate |
| **Switch** | On, Off |
| **Radio** | Selected, Unselected |

**Acceptance criteria:**
- Each component has a doctest.
- Each component has an example binary under
  `crates/opts-gui/examples/`.
- Each component's interactive states (hover, focus, pressed,
  disabled) are visually distinct.
- Each component responds to theme changes.

### Stage B7 — Interaction & Focus

Implement the interaction layer:

1. **Widget state components** — `Hovered`, `Focused`, `Pressed`,
   `Disabled`.
2. **State transition systems** — set the state components based
   on Bevy input events.
3. **Focus navigation** — keyboard/gamepad tab order using
   `bevy_input_focus` or an equivalent.
4. **Focus indicators** — visible focus ring that respects the
   `:focus-visible` convention.

**Acceptance criteria:**
- A doctest shows keyboard navigation between two buttons.
- A doctest shows a disabled button refusing interaction.
- An example binary demonstrates focus navigation across a form.

### Stage B8 — Gallery Example

Create `crates/opts-gui/examples/gallery.rs` — a single Bevy app
that renders every implemented component in every variant and
state. This is the reference for what the library can produce.

The gallery must include:
- A theme switcher (light/dark)
- Sections for each component tier
- A working form demonstrating interactive components together

**Acceptance criteria:**
- `cargo run --example gallery` runs successfully.
- Every component is visible and interactive.
- Theme switching works at runtime.

### Stage B9 — Documentation

1. **Module docs** — every module has `//!` docs explaining its
   purpose and linking to the design system doc.
2. **Item docs** — every public item has `///` docs per the
   coding standards.
3. **Crate docs** — `docs/crates/opts-gui.md` covers:
   - The crate's purpose
   - Its relationship to the design system
   - The plugin architecture
   - A worked example of building a screen
   - How to add a new component
   - Cross-references to the design system doc
4. **Design system doc** — update
   `docs/src/architecture/gui-design-system.md` with an
   "Implementation Notes" section if the design evolved during
   coding.

### Stage B10 — Verify

Run:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test -p opts-gui --all-features
cargo run -p opts-gui --example gallery
```

All must pass. Fix every warning without `#[allow(...)]` unless
justified in a comment.

---

## 5. Design Constraints (restated for Phase B)

These are non-negotiable:

1. **No hardcoded colors.** Every color comes from a theme token.
   A raw `Color::rgb(...)` in a widget is a bug.
2. **No hardcoded sizes.** Spacing comes from spacing tokens.
   Typography comes from type style tokens.
3. **No OOP.** Follow the coding standards: data + free functions,
   not methods on god objects.
4. **Files under 500 lines.** Split modules when they grow.
5. **Functions under 80 lines.**
6. **Every public item documented.**
7. **Every widget is a plugin.** Composable, removable, testable.
8. **Theme is a resource, not a global.** Accessible everywhere,
   mutable at runtime.
9. **No `unwrap()` in library code.** Use `Result` or provide
   sane defaults.
10. **`opts-gui` depends on Bevy and `opts-core` only.** It must
    not depend on `opts-sim` or `opts-bevy` (the latter creates a
    cycle).

---

## 6. Reporting Cadence

### Phase A Checkpoints

1. Stage A1 (design tokens) — approval before A2
2. Stage A2 (component inventory) — approval before A3
3. Stage A3–A5 (Bevy mapping, theming, anatomy) — approval before A6
4. Stage A6 (scope) — approval before A7

### Phase B Checkpoints

1. Stage B1–B2 (crate and modules) — approval before B3
2. Stage B3 (theme system) — approval before B4
3. Stage B4–B5 (tokens, layout) — approval before B6
4. Stage B6a (Button, Label, Icon) — approval before B6b
5. Stage B6b (Card, Divider, TextField) — approval before B6c
6. Stage B6c (Checkbox, Switch, Radio) — approval before B7
7. Stage B7 (interaction) — approval before B8
8. Stage B8 (gallery) — approval before B9
9. Stage B9–B10 (docs, verify) — final report

Do not proceed without confirmation.

Format:

```
## opts-gui — Checkpoint N

**Phase/Stage:** <name>
**Deliverables:** <summary>
**Design decisions:** <bulleted list>
**Open questions:** <if any>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 7. Exit Criteria

### Phase A

- ✅ `docs/src/architecture/gui-design-system.md` exists with all
  sections from Stages A1–A6.
- ✅ The document is reviewed and approved.
- ✅ No code written.

### Phase B

- ✅ `crates/opts-gui/` exists with the module structure from B2.
- ✅ All Tier 1 components implemented with doctests and examples.
- ✅ Theme system supports light and dark, with WCAG AA contrast.
- ✅ `cargo test -p opts-gui` passes.
- ✅ `cargo clippy -- -D warnings` passes.
- ✅ `cargo run --example gallery` renders all components.
- ✅ `docs/crates/opts-gui.md` exists.
- ✅ Every file under 500 lines.
- ✅ Every public item documented.
- ✅ No hardcoded colors or sizes.
- ✅ No `unwrap()` in library code.
- ✅ No `unsafe`.

When finished, respond with a summary containing:

- Number of tokens defined
- Number of components implemented
- Test count and pass rate
- Lines of code added
- Gallery screenshot description (if the agent can run it)
- Recommended next steps

Then respond with the exact string:
"OPTS-GUI FOUNDATION COMPLETE — ready for P-006"