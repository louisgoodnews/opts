# `opts-gui` — Material Design 3 GUI Widget Library

| Metadata | Value |
| :--- | :--- |
| **Crate** | `opts-gui` |
| **Layer** | Engine GUI / Presentation Layer |
| **Kind** | Library (`lib.rs`) |
| **Status** | 🟢 Fully Implemented (Foundation Complete) |
| **Last Updated** | 2026-10-08 |

---

## 🎯 Purpose

`opts-gui` provides the canonical Material Design 3 (M3) user interface component library and design system engine for the Open Public Transport Simulator (OPTS). It translates Material Design 3 specifications into idiomatic, performant Bevy ECS UI hierarchies.

### Architectural Commitments
1. **Strict Token Consumption:** Zero hardcoded colors (`Color::srgb(...)` is forbidden in widget declarations). All styling is driven by semantic design tokens (`BackgroundRole`, `BorderRole`, `ForegroundRole`, `RadiusToken`, `ElevationToken`, `TypeStyleToken`) that reactively re-evaluate on theme changes.
2. **Safety & Robustness:** `#![deny(unsafe_code)]`, zero bare `.unwrap()` calls across library code, all files strictly under 500 lines, functions under 80 lines.
3. **Ergonomic Dual-Spawn Pattern:** Every widget builder implements both `.spawn(&mut Commands) -> Entity` (top-level nodes) and `.spawn_child(&mut ChildBuilder) -> Entity` (nested flex hierarchies).
4. **Decoupled Architecture:** Crate depends strictly on `bevy` and `opts-core` (zero dependency on `opts-bevy` or `opts-sim` to eliminate dependency cycles).
5. **Decoupled Event Stream:** All user interactions emit semantic Bevy ECS events (`ButtonClicked`, `CheckboxToggled`, `SwitchToggled`, `RadioSelected`, `ComboboxSelected`, `NotebookTabSelected`, `ToastDismissed`, `ToplevelClosed`, `TreeNodeSelected`, `TreeNodeToggled`, `TextChanged`, `DateSelected`, `DateTimeSelected`, `ThemeChanged`).

---

## 📦 Public API Surface

### 1. Theme Engine & Tokens (`opts_gui::theme`, `opts_gui::tokens`)
- **`MaterialTheme`**: Root resource managing active theme state with light/dark runtime hot-swapping.
- **`ColorScheme`**: 25+ semantic color roles conforming to WCAG 2.1 AA relative luminance contrast ratios ($\ge 4.5:1$).
- **`TypographyTokens`**: All 15 Material 3 typography scale definitions (`DisplayLarge` through `LabelSmall`).
- **`ElevationTokens`**: Levels 0 through 5 shadow and container elevation definitions.
- **`ShapeTokens`**: Corner radius scales from `None` (0dp) to `Full` (pill/circle).
- **`SpacingTokens`**: 4dp/8dp grid step scales (`None` to `ExtraLarge`).
- **Token Components**: `BackgroundRole`, `BorderRole`, `ForegroundRole`, `RadiusToken`, `ElevationToken`, `TypeStyleToken`.

### 2. Layout Primitives (`opts_gui::layout`)
- **`StackBuilder`**: Vertical flex-column container with M3 spacing tokens.
- **`RowBuilder`**: Horizontal flex-row container with alignment and spacing tokens.
- **`GridBuilder`**: Multi-column responsive grid container with item spacing.

### 3. Core Input & Display Components (`opts_gui::components`)
- **`ButtonBuilder`**: Filled, Elevated, FilledTonal, Outlined, Text, and Icon variants with reactive `on_click` closures and `ButtonClicked` event dispatch.
- **`LabelBuilder`**: Text label bound to M3 typography scale and semantic foreground color roles.
- **`IconBuilder`**: Vector/glyph icon container supporting Small (18dp), Medium (24dp), and Large (32dp) sizes.
- **`CardBuilder`**: Elevated, Filled, and Outlined container cards with customizable padding.
- **`DividerBuilder`**: Horizontal and vertical 1dp line separators bound to `OutlineVariant`.
- **`TextFieldBuilder`**: Outlined and Filled text input fields with label, placeholder, and `TextChanged` event streaming.
- **`CheckboxBuilder`**: Tristate checkbox (Checked, Unchecked, Indeterminate) with visual indicator synchronization and `CheckboxToggled` event.
- **`SwitchBuilder`**: Pill-track toggle switch with animated thumb state and `SwitchToggled` event.
- **`RadioBuilder`**: Mutually exclusive radio option button with group ID binding and `RadioSelected` event.

### 4. Advanced Desktop & Transit Widgets
- **`ComboboxBuilder`**: Dropdown select menu with options list, selection indicator, and `ComboboxSelected` event.
- **`NotebookBuilder`**: Tabbed container with interactive tab header bar, active underline indicator, panel switching, and `NotebookTabSelected` event.
- **`ToastBuilder`**: Snackbar notification banner with auto-dismiss duration timer, action button, and `ToastDismissed` event.
- **`ToplevelBuilder`**: Floating modal window dialog with title bar, close button, body container, modal scrim overlay, and `ToplevelClosed` event.
- **`ModalBuilder`** (`ModalRoot`, `ModalContent`, `ModalEvent`): full-screen scrim (60 % black) with a centred card (70 % × 90 % of the viewport, 600–1200 px wide). The backdrop blocks pointer input beneath it and clicking it does nothing; Escape emits `ModalEvent::CancelRequested`; Tab / Shift+Tab cycle only through focusables inside the card (the global focus navigation is suspended while a modal exists).
- **`TextAreaBuilder`** (`TextArea`, `TextBuffer`, `TextAreaChanged`): multiline input. Layout-aware typing (umlauts, dead keys), Enter = newline, arrows, Home/End (Ctrl = document), PageUp/PageDown, Backspace/Delete, Shift-selection, `Ctrl+A/C/X/V` through `arboard` (falls back to an in-app clipboard with a `WARN` when no system clipboard exists), word-boundary wrapping, vertical scrolling that follows the cursor (wrapped-line count is estimated from glyph width), blinking cursor on `Time<Real>` so it keeps blinking while virtual time is paused. Tab is not consumed. Glyphs come from the active font: Bevy's built-in font is an ASCII-only Fira Mono subset, so apps must install a full-coverage TTF/OTF as the default font (`Assets<Font>::insert(Handle::<Font>::default().id(), font)`, see the `modal_text_area` example's `OPTS_EXAMPLE_FONT`). Bevy 0.14 has no font fallback or colour emoji; a single font must cover every glyph used.
- **`TreeviewBuilder` & `TreeNode`**: Hierarchical tree navigation with recursive nesting, expand/collapse chevron toggle, and `TreeNodeToggled`/`TreeNodeSelected` events.
- **`DateSelectBuilder` & `DateTimeSelectBuilder`**: Transit timetable and chronotimed event date/datetime selectors emitting `DateSelected` and `DateTimeSelected`.

### 5. Interaction & Focus Navigation (`opts_gui::interaction`)
- **`Focusable`**: Component marking an entity eligible for keyboard and gamepad focus.
- **`FocusEvent`**: Programmatic navigation events (`Next`, `Previous`, `Set(Entity)`, `Clear`).
- **`FocusPlugin`**: Keyboard listener handling `Tab` (forward) and `Shift+Tab` (backward) cycling.

---

## 🚀 Running Examples

```bash
# 1. Comprehensive Gallery (all widgets, themes, and interactive controls)
cargo run -p opts-gui --example gallery

# 2. Buttons, Text & Typography Scale
cargo run -p opts-gui --example buttons_and_text

# 3. Layout Primitives (Stacks, Rows, Grids)
cargo run -p opts-gui --example layout_demo

# 4. Modal dialog with multiline TextArea, Report/Cancel and a success toast
cargo run -p opts-gui --example modal_text_area
```

---

## 🧪 Verification & Test Metrics

- **Unit & Integration Tests:** 25 passing unit tests.
- **Doc-Tests:** 23 passing doctests (48 total in `opts-gui`).
- **Full Workspace:** All 156 workspace tests and doctests pass.
- **Linter Status:** Zero Clippy warnings (`cargo clippy --all-targets --all-features -- -D warnings`).
- **WCAG Verification:** Automated mathematical luminance verification confirming $\ge 4.5:1$ contrast ratio across all color roles.
