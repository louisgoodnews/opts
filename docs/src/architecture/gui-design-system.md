# 🎨 OPTS GUI Design System — Material Design 3 Specification

## 1. Executive Summary

This document specifies the design system and architecture for `opts-gui`, the user interface widget library for the Open Public Transport Simulator (OPTS). Built natively on Bevy UI (`bevy_ui`) and adhering strictly to Google Material Design 3 (M3) principles, `opts-gui` provides a modern, accessible, modular, and themeable UI foundation for menus, vehicle selectors, route/timetable managers, in-cab HUD displays, and future visual world-editing tools.

---

## 2. Design Tokens Hierarchy

Material Design 3 organizes design decisions into a three-tier token hierarchy that decouples foundational design primitives from component implementations.

```text
┌────────────────────────────────────────────────────────┐
│  Reference Tokens (Raw Values: Hex, px, ms, cubics)    │
│  e.g. md.ref.palette.primary40 = #0061A4               │
└──────────────────────────┬─────────────────────────────┘
                           ▼
┌────────────────────────────────────────────────────────┐
│  System Tokens (Semantic Roles & Theme Context)        │
│  e.g. md.sys.color.primary = md.ref.palette.primary40  │
└──────────────────────────┬─────────────────────────────┘
                           ▼
┌────────────────────────────────────────────────────────┐
│  Component Tokens (Scoped to Widget Elements)          │
│  e.g. md.comp.filled-button.container.color = primary  │
└────────────────────────────────────────────────────────┘
```

### 2.1 Rust Token Representation

In `opts-gui`, this token hierarchy maps directly to Rust type definitions:

1. **Reference Tokens:** Low-level constants in `tokens::colors`, `tokens::motion`, etc. (e.g. `const BLUE_40: Color = Color::srgb(0.0, 0.38, 0.64);`).
2. **System Tokens:** Strongly-typed enums and resources:
   - `ColorRole`: 26 semantic color roles.
   - `ColorScheme`: Direct lookup table (`[Color; 26]`) providing $O(1)$ role-to-color mapping.
   - `TypeScale`: 15 typography categories.
   - `ElevationLevel`: Levels 0 through 5.
   - `RadiusLevel`: 7 standard border radii.
   - `SpacingScale`: 4dp base unit multiplier.
3. **Component Tokens:** Structs configuring widget defaults and component bundles consuming system tokens via marker components (e.g., `BackgroundRole(ColorRole::Primary)`).

---

## 3. Core Token Systems

### 3.1 Color System

Material Design 3 defines 26 standard color roles across 6 semantic groups. Every foreground color is paired with a matching `on-*` color ensuring high contrast and readability.

| Group | Role | Purpose / Application | Default Light | Default Dark |
| :--- | :--- | :--- | :--- | :--- |
| **Primary** | `Primary` | High-emphasis fills, primary buttons, active toggles | `#0061A4` | `#9FCAFF` |
| | `OnPrimary` | Text and icons on `Primary` containers | `#FFFFFF` | `#003258` |
| | `PrimaryContainer` | Medium-emphasis fills, tonal buttons, active chips | `#D1E4FF` | `#00497D` |
| | `OnPrimaryContainer` | Text and icons on `PrimaryContainer` | `#001D36` | `#D1E4FF` |
| **Secondary** | `Secondary` | Less prominent components, filter chips, secondary actions | `#535F70` | `#BBC7DB` |
| | `OnSecondary` | Text and icons on `Secondary` containers | `#FFFFFF` | `#253140` |
| | `SecondaryContainer` | Quiet containers, badge backgrounds, unselected toggles | `#D7E3F7` | `#3B4858` |
| | `OnSecondaryContainer` | Text and icons on `SecondaryContainer` | `#101C2B` | `#D7E3F7` |
| **Tertiary** | `Tertiary` | Accent highlights, vehicle status indicators, departure warnings | `#6B5778` | `#D6BEE4` |
| | `OnTertiary` | Text and icons on `Tertiary` containers | `#FFFFFF` | `#3B2948` |
| | `TertiaryContainer` | Tonal containers for secondary status badges | `#F2DAFF` | `#523F5F` |
| | `OnTertiaryContainer` | Text and icons on `TertiaryContainer` | `#251432` | `#F2DAFF` |
| **Error** | `Error` | Critical warnings, simulation faults, validation errors | `#BA1A1A` | `#FFB4AB` |
| | `OnError` | Text and icons on `Error` containers | `#FFFFFF` | `#690005` |
| | `ErrorContainer` | Error banner backgrounds, invalid input fills | `#FFDAD6` | `#93000A` |
| | `OnErrorContainer` | Text and icons on `ErrorContainer` | `#410002` | `#FFDAD6` |
| **Surface** | `Surface` | Base window background, canvas, viewport backdrops | `#FDF8FD` | `#121316` |
| | `OnSurface` | Primary text and icons on surface | `#1A1C1E` | `#E2E2E6` |
| | `OnSurfaceVariant` | Secondary text, placeholder text, inactive icons | `#43474E` | `#C4C6D0` |
| | `SurfaceDim` | Dimmed surface under open modal dialogs | `#DED8E1` | `#121316` |
| | `SurfaceBright` | Highlighted surface panels | `#FEF7FF` | `#38393C` |
| | `SurfaceContainerLowest` | Deepest nested surface card | `#FFFFFF` | `#0D0E11` |
| | `SurfaceContainerLow` | Standard card container fill | `#F7F2FA` | `#1D1B20` |
| | `SurfaceContainer` | Default modal / dialog fill | `#F3EDF7` | `#211F26` |
| | `SurfaceContainerHigh` | Dropdown menu background | `#ECE6F0` | `#2B2930` |
| | `SurfaceContainerHighest` | Hovered card fill, input field fill | `#E6E0E9` | `#36343B` |
| **Outline** | `Outline` | Card borders, text field outlines, dividers | `#73777F` | `#8E9099` |
| | `OutlineVariant` | Subtle card dividers, decorative rules | `#C4C6D0` | `#44474E` |
| | `Scrim` | Full-screen modal overlay backdrops | `#000000` | `#000000` |

### 3.2 Typography Scale

Material Design 3 defines 15 baseline typography styles across 5 roles (Display, Headline, Title, Body, Label) and 3 scales (Large, Medium, Small).

| Type Style | Font Size | Line Height | Tracking | Weight | Intended Purpose |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `DisplayLarge` | 57 px | 64 px | -0.25 px | Regular (400) | Fullscreen intro screens, splash titles |
| `DisplayMedium` | 45 px | 52 px | 0.00 px | Regular (400) | Primary splash headers, timetable displays |
| `DisplaySmall` | 36 px | 44 px | 0.00 px | Regular (400) | Secondary display headers |
| `HeadlineLarge` | 32 px | 40 px | 0.00 px | Regular (400) | Top-level screen titles (e.g. Map Browser) |
| `HeadlineMedium` | 28 px | 36 px | 0.00 px | Regular (400) | Major section headings, bus depot titles |
| `HeadlineSmall` | 24 px | 32 px | 0.00 px | Regular (400) | Sub-section headings, card group titles |
| `TitleLarge` | 22 px | 28 px | 0.00 px | Regular (400) | Dialog titles, drawer headings |
| `TitleMedium` | 16 px | 24 px | +0.15 px | Medium (500) | Card titles, list header items |
| `TitleSmall` | 14 px | 20 px | +0.10 px | Medium (500) | Compact card titles, table headers |
| `BodyLarge` | 16 px | 24 px | +0.50 px | Regular (400) | Long-form reading, vehicle description text |
| `BodyMedium` | 14 px | 20 px | +0.25 px | Regular (400) | Standard UI body text, list subtitles |
| `BodySmall` | 12 px | 16 px | +0.40 px | Regular (400) | Secondary metadata, timestamps, hints |
| `LabelLarge` | 14 px | 20 px | +0.10 px | Medium (500) | Button labels, tabs, chip text |
| `LabelMedium` | 12 px | 16 px | +0.50 px | Medium (500) | Secondary button labels, tooltips |
| `LabelSmall` | 11 px | 16 px | +0.50 px | Medium (500) | Badge numbers, tiny status pills |

### 3.3 Elevation & Shadows

In M3, elevation communicates hierarchy and physical separation. In dark themes and flat rendering, elevation is expressed via **tonal elevation** (overlaying a subtle percentage of the `Primary` color onto the `Surface` color) combined with Bevy's native `BoxShadow`.

| Level | Elevation (dp) | Tonal Tint (%) | Shadow Offset / Blur | Component Usage |
| :--- | :--- | :--- | :--- | :--- |
| `Level0` | 0 dp | 0% | None | Flat cards, text buttons, inline dividers |
| `Level1` | 1 dp | 5% | `(0, 1) / 3px` blur | Card containers, elevated buttons at rest |
| `Level2` | 3 dp | 8% | `(0, 2) / 6px` blur | Hovered cards, elevated buttons on hover |
| `Level3` | 6 dp | 11% | `(0, 4) / 8px` blur | Menus, popups, toolbars |
| `Level4` | 8 dp | 12% | `(0, 6) / 10px` blur | Navigation drawer, modal sheets |
| `Level5` | 12 dp | 14% | `(0, 8) / 14px` blur | Modal dialogs, date pickers |

### 3.4 Shape (Corner Radii)

| Radius Level | Value | Component Application |
| :--- | :--- | :--- |
| `None` | 0 dp | Sharp rectangles, dividers, full-bleed screen containers |
| `ExtraSmall` | 4 dp | Text fields, tooltips, small badges |
| `Small` | 8 dp | Chips, compact cards, toast notices |
| `Medium` | 12 dp | Standard cards, alert dialogs |
| `Large` | 16 dp | Expanded cards, navigation drawers |
| `ExtraLarge` | 28 dp | Floating Action Buttons (FAB), large modal sheets |
| `Full` | 9999 dp | Pill buttons, circular icon buttons, toggle switches |

### 3.5 Spacing Grid

OPTS GUI adheres to a strict 4dp base grid (`Val::Px(4.0 * n)`).

| Spacing Token | Pixels | Grid Units | Usage |
| :--- | :--- | :--- | :--- |
| `SpacingNone` | 0 px | 0 | Reset margin/padding |
| `SpacingXSmall` | 4 px | 1 | Micro spacing between icon and badge |
| `SpacingSmall` | 8 px | 2 | Icon-to-text gap, compact list padding |
| `SpacingMedium` | 12 px | 3 | Inner form field padding |
| `SpacingStandard` | 16 px | 4 | Standard screen margins, button horizontal padding |
| `SpacingLarge` | 24 px | 6 | Card padding, section gaps |
| `SpacingXLarge` | 32 px | 8 | Modal margins, header-to-content separation |
| `SpacingHuge` | 48 px | 12 | Top-level screen gutter |

### 3.6 Motion & Curves

| Duration Token | Value | Easing Curve | Application |
| :--- | :--- | :--- | :--- |
| `DurationShort` | 100 ms | Standard Decelerate | Hover state overlays, button press feedback |
| `DurationMedium` | 250 ms | Standard Easing | Menu expansion, dialog fade-in, toggle transitions |
| `DurationLong` | 400 ms | Emphasized Easing | Screen transitions, drawer sliding |

---

## 4. Component Inventory

The component inventory is partitioned into three implementation tiers based on frequency of use and UI criticality.

### 4.1 Tier 1 — Core Foundation (In Scope for Phase B v1)

| Component | Description | Variants | Key Interactive States | Bevy ECS Architecture |
| :--- | :--- | :--- | :--- | :--- |
| **`Button`** | Primary trigger for user actions | Filled, Elevated, Tonal, Outlined, Text, Icon | Rest, Hover, Focus, Press, Disabled | Composite entity: container `NodeBundle` + icon + `TextBundle`. Observers for pointer events. |
| **`Label`** | Semantic text display | All 15 `TypeScale` styles | Rest | Text entity with `TypeStyleToken` and `ForegroundRole`. |
| **`Icon`** | Scalable glyph or vector image | Small (18dp), Medium (24dp), Large (32dp) | Rest, Disabled | Image / TextureAtlas / Glyph node with `ForegroundRole`. |
| **`Card`** | Content container with elevation | Filled, Outlined, Elevated | Rest, Hover, Focus, Press | Parent `NodeBundle` with `ElevationToken`, `RadiusToken`, and `BorderRole`. |
| **`Divider`** | Structural line separator | Horizontal, Vertical | Rest | Thin `NodeBundle` (1dp thickness) with `OutlineVariant` role. |
| **`TextField`** | Text input with label & placeholder | Outlined, Filled | Rest, Focus, Error, Disabled | Container node with borders, inner text entity, cursor tick system, and IME/keyboard listener. |
| **`Checkbox`** | Multi-selection toggle | Square (checked, unchecked, indeterminate) | Rest, Hover, Focus, Disabled | Box node with `BorderRole` or `BackgroundRole` + check glyph child. |
| **`Switch`** | Binary toggle control | Pill track + circular thumb | On, Off, Hover, Disabled | Track node (Full radius) containing animated thumb node (Full radius). |
| **`Radio`** | Single-selection option control | Circular ring + inner dot | Selected, Unselected, Hover, Disabled | Outer circle node + conditional inner dot child entity. |

### 4.2 Tier 2 — Common Patterns (Future Phases)

- **`AppBar` / `TopBar`**: Screen header with title, navigation icon, and actions.
- **`NavigationDrawer`**: Collapsible side menu for screen navigation.
- **`Tabs`**: Primary and secondary tab bars for content switching.
- **`Dialog`**: Modal popup for confirmations and critical warnings.
- **`Menu` / `Dropdown`**: Contextual floating action list.
- **`Tooltip`**: Informational hover bubble.
- **`Slider`**: Continuous value selection for audio volume, graphics settings.
- **`Badge`**: Notification count or status dot attached to icons.
- **`Chip`**: Compact interactive tag for route filters and tags.
- **`ProgressBar`**: Determinate and indeterminate loading indicator.

### 4.3 Tier 3 — Situational Widgets (Future Phases)

- `BottomSheet`, `Snackbar` / `Toast`, `FAB` (Floating Action Button), `SegmentedButton`, `DatePicker`, `TimePicker`, `DataTable`, `SearchBar`, `ListTile`, `Accordion`, `Carousel`.

---

## 5. Bevy ECS Mapping Architecture

### 5.1 The Theme Resource (`MaterialTheme`)

`MaterialTheme` is registered as a Bevy `Resource` in the `App` world:

```rust
#[derive(Resource, Clone, Debug)]
pub struct MaterialTheme {
    pub scheme: ColorScheme,
    pub typography: TypographyTokens,
    pub elevation: ElevationTokens,
    pub shape: ShapeTokens,
    pub spacing: SpacingTokens,
    pub motion: MotionTokens,
}
```

When `ResMut<MaterialTheme>` is modified, Bevy's change detection triggers reactive theme sync systems.

### 5.2 Token Marker Components

UI elements consume tokens through lightweight ECS marker components:

```rust
#[derive(Component, Clone, Copy, Debug)]
pub struct BackgroundRole(pub ColorRole);

#[derive(Component, Clone, Copy, Debug)]
pub struct ForegroundRole(pub ColorRole);

#[derive(Component, Clone, Copy, Debug)]
pub struct BorderRole(pub ColorRole);

#[derive(Component, Clone, Copy, Debug)]
pub struct TypeStyleToken(pub TypeScale);

#[derive(Component, Clone, Copy, Debug)]
pub struct RadiusToken(pub RadiusLevel);

#[derive(Component, Clone, Copy, Debug)]
pub struct ElevationToken(pub ElevationLevel);
```

### 5.3 Reactive Sync Systems

Systems run whenever `Res<MaterialTheme>::is_changed()` or when new token components are added:

```rust
pub fn sync_background_roles(
    theme: Res<MaterialTheme>,
    mut query: Query<(&BackgroundRole, &mut BackgroundColor), Or<(Changed<BackgroundRole>, () )>>,
) {
    if !theme.is_changed() && query.is_empty() {
        return;
    }
    for (role, mut bg) in &mut query {
        bg.0 = theme.scheme.get(role.0);
    }
}
```

### 5.4 Plugin Composition

`opts-gui` exposes modular plugins grouped under `OptsGuiPlugin`:

```text
OptsGuiPlugin
├── MaterialThemePlugin (manages theme resource & events)
├── TokenSyncPlugin (syncs BackgroundRole, BorderRole, RadiusToken, TypeStyleToken, ElevationToken)
├── InteractionPlugin (manages hover, press, focus traversal, disabled states)
└── ComponentsPlugin
    ├── ButtonPlugin
    ├── LabelPlugin
    ├── IconPlugin
    ├── CardPlugin
    ├── DividerPlugin
    ├── TextFieldPlugin
    ├── CheckboxPlugin
    ├── SwitchPlugin
    └── RadioPlugin
```

---

## 6. Theming Strategy & Accessibility

### 6.1 Built-in Schemes & WCAG AA Compliance

`opts-gui` includes canonical `ColorScheme::light()` and `ColorScheme::dark()` implementations. Every color role pairing is validated by unit tests to meet **WCAG 2.1 Level AA** contrast standards:

- **Normal Text / Icons:** $\text{Contrast Ratio} \ge 4.5:1$ against the container background (e.g. `OnPrimary` on `Primary`, `OnSurface` on `Surface`).
- **Large Text / UI Borders:** $\text{Contrast Ratio} \ge 3.0:1$ (e.g. `Outline` on `Surface`).

### 6.2 Runtime Theme Switching

Themes can be hot-swapped dynamically without restarting or reloading UI scenes:

```rust
fn toggle_theme_system(
    input: Res<ButtonInput<KeyCode>>,
    mut theme: ResMut<MaterialTheme>,
) {
    if input.just_pressed(KeyCode::F2) {
        theme.scheme = if theme.scheme.is_dark() {
            ColorScheme::light()
        } else {
            ColorScheme::dark()
        };
    }
}
```

Because Bevy systems watch `Res<MaterialTheme>`, every active widget re-skins in a single frame.

### 6.3 Modding & Custom Theme Serialization

`MaterialTheme` derives `serde::Serialize` and `serde::Deserialize`. Modders and bus livery creators can author external RON files (e.g., `assets/themes/bvg_yellow.ron`) to customize simulator styling.

---

## 7. Component Anatomy Reference — `Button`

To establish the standard pattern for all widgets in `opts-gui`, the `Button` anatomy is specified in detail:

### 7.1 Entity Hierarchy

```text
Button Root Entity
├── Components:
│   ├── NodeBundle (layout: Row, min_height: 40dp, padding: (16dp, 24dp), align: Center, justify: Center)
│   ├── Interaction (Bevy UI interaction tracker)
│   ├── M3Button { variant: ButtonVariant }
│   ├── BackgroundRole(ColorRole::Primary)
│   ├── ForegroundRole(ColorRole::OnPrimary)
│   ├── RadiusToken(RadiusLevel::Full)
│   ├── ElevationToken(ElevationLevel::Level0)
│   └── Focusable { tab_index: 0 }
│
├── Child 1: Leading Icon (Optional)
│   ├── ImageBundle / Glyph
│   ├── ForegroundRole(ColorRole::OnPrimary)
│   └── Node (18dp × 18dp, margin_right: 8dp)
│
└── Child 2: Label Text
    ├── TextBundle
    ├── TypeStyleToken(TypeScale::LabelLarge)
    ├── ForegroundRole(ColorRole::OnPrimary)
    └── TextLayout
```

### 7.2 Interactive States & State Overlays

In accordance with M3, interactive surfaces overlay a semi-transparent state layer:

- **Rest:** Base role colors and resting elevation (Level 0 for Filled, Level 1 for Elevated).
- **Hover:** Blends 8% `on-primary` state overlay; elevates by +1 level.
- **Focus:** 10% `on-primary` state overlay + visible 2dp focus ring (`Secondary` role) with 2dp offset.
- **Pressed:** 10% `on-primary` state overlay; drops elevation back to Level 0.
- **Disabled:** Ignores input, renders 12% `on-surface` background and 38% `on-surface` foreground.

### 7.3 Public Fluent Builder API

```rust
// Spawning an action button:
commands.spawn(
    ButtonBuilder::filled("Start Simulation")
        .with_icon(asset_server.load("icons/play_arrow.png"))
        .on_click(|mut commands| {
            info!("Simulation started");
        })
);

// Spawning a secondary outlined button:
commands.spawn(
    ButtonBuilder::outlined("Settings")
);
```

---

## 8. Scope Boundaries

### 8.1 In Scope for Phase B v1

- **Core Theme System:** `MaterialTheme` resource, `ColorScheme` (light and dark palettes with all 26 roles), `TypographyTokens`, `ElevationTokens`, `ShapeTokens`, `SpacingTokens`, `MotionTokens`.
- **Token Marker Components:** `BackgroundRole`, `ForegroundRole`, `BorderRole`, `TypeStyleToken`, `RadiusToken`, `ElevationToken` and their reactive update systems.
- **Layout Primitives:** `Stack` (vertical flex), `Row` (horizontal flex), and `Grid` (track grid).
- **Tier 1 Components:**
  1. `Button` (Filled, Elevated, Tonal, Outlined, Text, Icon variants)
  2. `Label` (all 15 typography styles)
  3. `Icon` (sized glyphs)
  4. `Card` (Filled, Outlined, Elevated)
  5. `Divider` (Horizontal, Vertical)
  6. `TextField` (Outlined, Filled with cursor and typing support)
  7. `Checkbox` (Checked, Unchecked, Indeterminate)
  8. `Switch` (On, Off toggle)
  9. `Radio` (Selected, Unselected choice)
- **Interaction Layer:** Hover, press, focus navigation (Tab/Shift-Tab keyboard cycling), focus rings, disabled state.
- **Gallery Example:** `cargo run --example gallery` demonstrating all components, variants, interactive states, and runtime light/dark theme switching.
- **Tests & Verification:** Zero clippy warnings, zero unwrap in library code, WCAG AA contrast ratio unit tests, unit tests for all tokens.

### 8.2 Out of Scope for Phase B v1

- Tier 2 and Tier 3 components (Dialogs, Menus, AppBars, Drawers, Sliders, Pickers, Tables).
- Dynamic color extraction from user wallpapers / images (Material You HCT algorithm).
- Complex spring physics simulations for motion.
- Accessibility extensions beyond Bevy defaults.
- Concrete simulation UI screens (menus, vehicle selectors, in-cab HUDs).
