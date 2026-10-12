# 🎨 OPTS GUI Design System — Token Specification

| Metadata | Value |
| :--- | :--- |
| **Status** | ✅ Revised 2026-10-12 for [M-0061](../roadmap/phase-9/M-0061.md) part A (theme foundations); it describes what the code does |
| **Authority** | The Phase 9 decisions ([phase page](../roadmap/phase-9/README.md), [design language](../roadmap/phase-9/design-language.md)) are the *why*; this page is the *what*. Component anatomy (§4, §7, §8) is older and is revised with the widget work of M-0061 part B |
| **Code** | `crates/opts-gui/src/theme/**`, `tokens/**`, `fonts.rs`; settings in `crates/opts-game/src/settings*.rs` |
| **Progress** | [`docs/dev/progress/ui-theme.md`](../../dev/progress/ui-theme.md) |

## 1. Executive Summary

`opts-gui` is the user interface library of the Open Public Transport Simulator (OPTS), built natively on Bevy UI. It keeps Material Design 3's **token architecture** (reference, system and component tokens; semantic colour roles; a theme resource that re-skins every widget in one frame) but gives OPTS its **own expression** ([decision D-UI-01](../roadmap/phase-9/design-language.md#2-keep-the-token-architecture-change-its-expression)): a calm operations-room character, squarer controls (radius 8/12/16), glass only over the live 3D scene, no per-widget colour swatches.

What the foundation guarantees, each with a test:

| Guarantee | Where it is enforced |
| :--- | :--- |
| **Dark is the default**; Light, System, Auto (clock) and a High-contrast theme | `theme::mode`, `theme::runtime`, `settings_theme` |
| **Every colour pair meets its contrast threshold** in every theme and every accent preset (4.5:1 text, 3:1 control boundaries; 7:1 and 4.5:1 in high contrast) | `theme::audit` (`every_theme_and_preset_passes_every_pair`), `cargo run -p opts-gui --example theme_dump` |
| **Nothing renders below 12 px** at 100 % UI scale | the 9-style scale and the interim `MinTextSize` clamp |
| **One theme everywhere**: loading screen, HUD chrome and stub screens use role tokens, instruments use their own token set | `tests/theme_integration.rs` (no literal colours in those files) |
| **UI scale 75-200 %**, density, layout classes, window minimum 1024×640 | `theme::scale`, `theme::spacing`, `opts_cli::MIN_WINDOW_SIZE` |
| Reduced motion turns every transition into an instant change | `theme::motion` |

---

## 2. Design Tokens Hierarchy

```text
┌────────────────────────────────────────────────────────┐
│  Reference tokens: raw values (hex, px, ms, béziers)   │
│  e.g. the accent seed #0061A4, the 4 px grid           │
└──────────────────────────┬─────────────────────────────┘
                           ▼
┌────────────────────────────────────────────────────────┐
│  System tokens: semantic roles and theme context       │
│  e.g. ColorRole::Primary in the Dark theme             │
└──────────────────────────┬─────────────────────────────┘
                           ▼
┌────────────────────────────────────────────────────────┐
│  Component tokens: marker components on entities       │
│  e.g. BackgroundRole(ColorRole::SurfaceContainerLow)   │
└────────────────────────────────────────────────────────┘
```

### 2.1 Rust Token Representation

1. **Reference tokens:** the accent seeds of `AccentPreset`, the fixed neutral tables of `theme::palette`, `tokens::colors` (the legacy named M3 constants), the font files.
2. **System tokens:** `ColorRole` (**46 roles**: the 34 M3 roles plus 12 status roles), `ColorScheme` (`[Color; 46]`, O(1) lookup, carries its `ThemeKind`), `TypeScale` (**9 styles**), `ElevationLevel` (0 to 5), `RadiusLevel` (7 radii), `SpacingScale` (10 steps) with `Density`, `DurationToken` and `EasingCurve`, `InstrumentTokens`.
3. **Component tokens:** the marker components `BackgroundRole`, `ForegroundRole`, `BorderRole`, `TypeStyleToken` (+ `TypeWeight`), `RadiusToken`, `ElevationToken`, and for HUD instruments `InstrumentPanel` / `InstrumentInk`.

---

## 3. Core Token Systems

### 3.1 Color System

**46 roles in 7 groups.** Every foreground role has a matching `On…` role. The M3 groups are unchanged in meaning; the **status group** (`Success`, `Warning`, `Info`, each with `On…`, a container and `On…Container`) is new: status is always icon + text + colour, never colour alone.

The values below are the *generated* defaults (accent "Transit blue", seed `#0061A4`); they are printed by `cargo run -p opts-gui --example theme_dump`, so this table cannot drift from the code.

| Group | Role | Purpose | Dark | Light | High contrast |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Primary** | `Primary` | high-emphasis fills, active toggles, links and active labels (also readable as text on every surface) | `#86C4FF` | `#0060A3` | `#C6E1FD` |
|  | `OnPrimary` | text and icons on `Primary` | `#041C32` | `#F9FCFF` | `#041C32` |
|  | `PrimaryContainer` | medium-emphasis fills, selected states, tonal buttons | `#084573` | `#C5E2FF` | `#032F53` |
|  | `OnPrimaryContainer` | text and icons on `PrimaryContainer` | `#C9E1F9` | `#102335` | `#EEF6FF` |
| **Secondary** | `Secondary` | less prominent components (the accent hue at low chroma) | `#ADC0D4` | `#495A6B` | `#C6DAEE` |
|  | `OnSecondary` | text and icons on `Secondary` | `#161B21` | `#F9FCFF` | `#161B21` |
|  | `SecondaryContainer` | quiet containers, unselected toggles, chips | `#2C3947` | `#D2E4F5` | `#1D2A37` |
|  | `OnSecondaryContainer` | text and icons on `SecondaryContainer` | `#D8DFE6` | `#1D2227` | `#EFF6FD` |
| **Tertiary** | `Tertiary` | accent highlights, a second hue (fixed) | `#D6BEE4` | `#6B5778` | `#FFC6FF` |
|  | `OnTertiary` | text and icons on `Tertiary` | `#3B2948` | `#FFFFFF` | `#000000` |
|  | `TertiaryContainer` | tonal containers for secondary badges | `#523F5F` | `#F2DAFF` | `#4A1F5C` |
|  | `OnTertiaryContainer` | text and icons on `TertiaryContainer` | `#F2DAFF` | `#251432` | `#FFFFFF` |
| **Error** | `Error` | critical warnings, faults, validation errors; also the red "Feature not available yet!" text | `#FFB4AB` | `#BA1A1A` | `#FFA399` |
|  | `OnError` | text and icons on `Error` | `#690005` | `#FFFFFF` | `#000000` |
|  | `ErrorContainer` | error banners, invalid input fills | `#93000A` | `#FFDAD6` | `#7A0008` |
|  | `OnErrorContainer` | text and icons on `ErrorContainer` | `#FFDAD6` | `#410002` | `#FFFFFF` |
| **Surface** | `Surface` | window background, canvas | `#121316` | `#FDF8FD` | `#000000` |
|  | `OnSurface` | primary text and icons | `#E2E2E6` | `#1A1C1E` | `#FFFFFF` |
|  | `SurfaceVariant` | variant surface (tab bars, wells) | `#42474F` | `#E1E2EC` | `#1F1F1F` |
|  | `OnSurfaceVariant` | secondary text, placeholders, inactive icons | `#C4C6D0` | `#43474E` | `#E6E6E6` |
|  | `InverseSurface` | toasts, tooltips (the opposite polarity) | `#E2E2E6` | `#2E3133` | `#FFFFFF` |
|  | `InverseOnSurface` | text on `InverseSurface` | `#1A1C1E` | `#F1F0F4` | `#000000` |
|  | `InversePrimary` | accent on `InverseSurface` (the other theme's primary) | `#0060A3` | `#86C4FF` | `#085B98` |
|  | `SurfaceDim` | dimmed surface under dialogs | `#121316` | `#DED8E1` | `#000000` |
|  | `SurfaceBright` | highlighted surface panels | `#38393D` | `#FFF8FF` | `#2E2E2E` |
|  | `SurfaceContainerLowest` | deepest nested surface | `#0D0E11` | `#FFFFFF` | `#000000` |
|  | `SurfaceContainerLow` | cards and sidebars | `#1C1D21` | `#F7F2FA` | `#0A0A0A` |
|  | `SurfaceContainer` | dialogs, default container | `#211F26` | `#F3EDF7` | `#121212` |
|  | `SurfaceContainerHigh` | menus, dropdowns, list items | `#2B2A2F` | `#ECE6F0` | `#1A1A1A` |
|  | `SurfaceContainerHighest` | hover fill, input fields | `#35343A` | `#E6E0E9` | `#242424` |
| **Outline and utility** | `Outline` | control boundaries and focus rings (>= 3:1 on every surface) | `#93959E` | `#6A6E76` | `#FFFFFF` |
|  | `OutlineVariant` | decorative dividers only (not a control boundary) | `#44474E` | `#C4C6D0` | `#9A9A9A` |
|  | `Scrim` | modal backdrops and the veil over photographs | `#000000` | `#000000` | `#000000` |
|  | `Shadow` | shadow colour | `#000000` | `#000000` | `#000000` |
| **Status (new)** | `Success` | success status: text, icons, fills | `#7CD591` | `#07672E` | `#96F0AA` |
|  | `OnSuccess` | text on `Success` | `#011C08` | `#F5FFF6` | `#000600` |
|  | `SuccessContainer` | success banners, result rows | `#194224` | `#C1EFC9` | `#063215` |
|  | `OnSuccessContainer` | text on `SuccessContainer` | `#D2E4D4` | `#18251A` | `#E9FBEB` |
|  | `Warning` | warning status: text, icons, fills | `#E9B452` | `#6F4F06` | `#FFCF7F` |
|  | `OnWarning` | text on `Warning` | `#211300` | `#FFFBF5` | `#090200` |
|  | `WarningContainer` | warning banners | `#4B3301` | `#FADDAD` | `#372501` |
|  | `OnWarningContainer` | text on `WarningContainer` | `#E8DCCA` | `#292012` | `#FFF4E2` |
|  | `Info` | informational status: text, icons, fills | `#49CFFC` | `#045E78` | `#9FE3FF` |
|  | `OnInfo` | text on `Info` | `#001923` | `#F7FDFF` | `#000409` |
|  | `InfoContainer` | information banners, hints | `#003E51` | `#BAEAFE` | `#012E3C` |
|  | `OnInfoContainer` | text on `InfoContainer` | `#CBE2EC` | `#12252C` | `#E9F8FE` |

**How the palettes are made.** The surfaces, outlines, error and tertiary groups are fixed tables per theme (`theme::palette`). Everything that depends on the accent (`Primary…`, `Secondary…`, `InversePrimary`) and the status roles are **generated** from a hue and a chroma in OKLCH by `theme::accent`: a starting lightness per theme kind is chosen, then moved until every pair reaches its threshold (`contrast::ensure_contrast`). `Primary` additionally has to read as *text* on every surface, because links and active labels use it.

**Contrast rules** (`theme::audit`, 89 pairs per theme):

| Rule | Normal themes | High contrast |
| :--- | ---: | ---: |
| text and icons: every `Role`/`OnRole` pair; `OnSurface`, `OnSurfaceVariant`, `Primary`, `Error`, `Success`, `Warning`, `Info` on every surface | 4.5:1 | 7:1 |
| control boundaries: `Outline` on every surface | 3:1 | 4.5:1 |
| exempt (decorative) | `OutlineVariant`, `Scrim`, `Shadow` | same |

Measured over every theme and every accent preset (all pass; the lowest pair of each is listed):

| Theme | Accent | Rules | Violations | Lowest text pair | Lowest control pair |
| :--- | :--- | ---: | ---: | :--- | :--- |
| Dark | Transit blue | 89 | 0 | 4.95 (Warning on SurfaceVariant) | 3.13 (Outline on SurfaceVariant) |
| Dark | Amber | 89 | 0 | 4.87 (Primary on SurfaceVariant) | 3.13 (Outline on SurfaceVariant) |
| Dark | Violet | 89 | 0 | 4.86 (Primary on SurfaceVariant) | 3.13 (Outline on SurfaceVariant) |
| Dark | Emerald | 89 | 0 | 4.95 (Warning on SurfaceVariant) | 3.13 (Outline on SurfaceVariant) |
| Dark | Teal | 89 | 0 | 4.95 (Warning on SurfaceVariant) | 3.13 (Outline on SurfaceVariant) |
| Light | Transit blue | 89 | 0 | 4.62 (Error on SurfaceDim) | 3.66 (Outline on SurfaceDim) |
| Light | Amber | 89 | 0 | 4.62 (Error on SurfaceDim) | 3.66 (Outline on SurfaceDim) |
| Light | Violet | 89 | 0 | 4.62 (Error on SurfaceDim) | 3.66 (Outline on SurfaceDim) |
| Light | Emerald | 89 | 0 | 4.52 (Primary on SurfaceDim) | 3.66 (Outline on SurfaceDim) |
| Light | Teal | 89 | 0 | 4.53 (Primary on SurfaceDim) | 3.66 (Outline on SurfaceDim) |
| HighContrast | Transit blue | 89 | 0 | 7.08 (Error on SurfaceBright) | 13.58 (Outline on SurfaceBright) |
| HighContrast | Amber | 89 | 0 | 7.08 (Error on SurfaceBright) | 13.58 (Outline on SurfaceBright) |
| HighContrast | Violet | 89 | 0 | 7.08 (Error on SurfaceBright) | 13.58 (Outline on SurfaceBright) |
| HighContrast | Emerald | 89 | 0 | 7.08 (Error on SurfaceBright) | 13.58 (Outline on SurfaceBright) |
| HighContrast | Teal | 89 | 0 | 7.08 (Error on SurfaceBright) | 13.58 (Outline on SurfaceBright) |

**Colour-blind safety.** Red and green are never the only distinction; the simulation of deuteranopia/protanopia named in the design language is part of the accessibility audit of M-0065 and is **not** implemented here.

### 3.2 Typography


*Decision D-UI-03, taken in M-0061 after a rendered comparison.*

#### The typeface

| | |
| :--- | :--- |
| **UI family** | **Noto Sans** (SIL OFL 1.1): Regular 400, Medium 500, Bold 700 bundled |
| **Fallback** | DejaVu Sans Regular behind it in the family stack, for glyphs Noto Sans lacks (arrows `→ ↔`, geometric shapes, check marks) |
| **Monospaced companion** | JetBrains Mono Regular (SIL OFL 1.1): diagnostics and logs |
| **Icons** | Material Icons (Apache 2.0), a separate font |
| **Family stack** | `fonts::UI_FONT_STACK = "Noto Sans, DejaVu Sans"`; text selects the face by family and `FontWeight` |

**How it was decided.** The font files had to be obtainable offline, so the candidates were the licensed fonts present on the build machine: Noto Sans (full static weight set), Inter (only Regular and SemiBold available, from the JetBrains Runtime), DejaVu Sans and Open Sans; IBM Plex Sans was not available. `crates/opts-gui/examples/font_compare.rs` draws the same specimen lines (German text, sizes 12, 13, 16, 20 px, the four weights, numerals, instrument readouts) in each candidate with the real Bevy text stack and saves a PNG; the result was inspected at 14 px:

| Candidate | What the rendering showed |
| :--- | :--- |
| **Noto Sans** | clean at 12-16 px; Regular, Medium and Bold are distinct faces (600 resolves to Bold); digits already tabular (`1111111111` and `8888888888` align); lacks the arrow `→` (hence the fallback) |
| Inter | the most refined small-size rendering and complete symbols, but only two weights could be had offline (400 and 600; 500 renders as 400, 700 as 600); fails the weight criterion until the full family is added |
| DejaVu Sans | very wide, utilitarian; Medium and Regular look identical; good symbol coverage (kept as fallback) |
| Open Sans | similar to Noto Sans in colour; no arrow glyph, and Medium renders as Regular in the local copy |

Noto Sans was chosen for: three distinct weights plus 600 -> Bold, OFL licence compatible with the repository, German diacritics and a path to Greek and Cyrillic (the same family covers them) and to further scripts, tabular figures by default, and good rendering under Bevy's text stack. **Limitation:** four distinct weights (400/500/600/700) were not obtainable offline; three are bundled and a 600 request renders as 700. **Upgrade path:** add the Inter or Noto Sans *variable* font from the upstream release (OFL) to `assets/fonts/`, register it in `fonts::install_fonts` and change `UI_FAMILY`/`UI_FONT_STACK`; nothing else changes, because every size, weight and figure style lives in the type scale table and `FontVariations` can select any weight. The licences are listed in [legal and licensing](../contributing/legal-licensing.md#bundled-fonts-m-0061).

#### The scale

Nine styles (the former 15 Material styles are deprecated aliases that resolve to them, see `TypeScale::canonical`). Sizes are logical pixels at UI scale 100 %; Bevy multiplies them by the UI scale.

| Style | Size / line height | Tracking | Weight | Use |
| :--- | :--- | :--- | :--- | :--- |
| `Display` | 40 / 48 | -0.5 | 700 | hero titles only |
| `Headline` | 28 / 36 | -0.25 | 700 | screen titles |
| `TitleLarge` | 20 / 28 | 0 | 700 | dialog and section titles |
| `TitleMedium` | 16 / 24 | +0.1 | 500 | card and list titles |
| `BodyLarge` | 16 / 24 | +0.1 | 400 | long text |
| `BodyMedium` | 14 / 20 | +0.15 | 400 | standard UI text |
| `Label` | 12 / 16 | +0.4 | 500 | labels, chips, overline, hints |
| `NumericLarge` | 48 / 56 | 0 | 700, tabular | speed readout |
| `NumericMedium` | 32 / 40 | 0 | 700, tabular | gear and secondary readouts |

`sync_typestyle_tokens` applies **all** of it to the text of a `TypeStyleToken`: size, `LineHeight` (as a multiple of the size, so it scales), `LetterSpacing`, weight, family and the `tnum` feature of tabular styles. Text with the icon font keeps its font. Old aliases: `DisplayX` -> `Display`, `HeadlineX` -> `Headline`, `TitleSmall` and `LabelLarge` -> `BodyMedium` at weight 500, `LabelMedium`, `LabelSmall` and `BodySmall` -> `Label`.

**Minimum size.** No text renders below 12 logical px: the scale has no smaller style, and the interim `MinTextSize` clamp (`clamp_text_size`) raises every literal `FontSize::Px` below 12 to 12 (UI scale multiplies afterwards, so the floor is 12 px times the scale). It is a temporary guard for screens that still contain literal sizes of 10 and 11 px; it disappears when they have migrated to the scale (M-0062 and later).

### 3.3 Elevation & Shadows

Elevation is expressed twice, in **both** themes (`theme::elevation`, `tokens::sync_*`):

1. **Tonal tint:** the surface colour mixed with `Primary` by a level-dependent fraction (applied by `sync_background_roles` to every entity that has an `ElevationToken`; text on the tinted surface keeps 4.5:1, tested for levels 1 and 5 in both themes).
2. **A real drop shadow:** Bevy's `BoxShadow`, inserted and refreshed by `sync_elevation_shadows`. The shadow colour is the `Shadow` role at alpha 0.60 in dark themes (a shadow is hard to see on a dark surface) and 0.28 in light ones.

| Level | Elevation | Tonal tint | Shadow (y offset / blur) | Use |
| :--- | :--- | :--- | :--- | :--- |
| `Level0` | 0 dp | 0 % | none | flat cards, inline dividers |
| `Level1` | 1 dp | 5 % | 1 / 3 px | cards at rest |
| `Level2` | 3 dp | 8 % | 2 / 6 px | hovered cards, the navigation sidebar |
| `Level3` | 6 dp | 11 % | 4 / 8 px | menus, popups |
| `Level4` | 8 dp | 12 % | 6 / 10 px | sheets |
| `Level5` | 12 dp | 14 % | 8 / 14 px | modal dialogs |

Floating layers additionally get a 1 px `Outline` border (decision in the design language §7); that part belongs to the widgets.

### 3.4 Shape (Corner Radii)

The seven levels are unchanged; what changes is **which one OPTS uses where** (D-UI-01):

| Use | Level | Value |
| :--- | :--- | :--- |
| controls: buttons, fields, list items | `Small` | **8 px** |
| cards, panels | `Medium` | **12 px** |
| sheets, dialogs | `Large` | **16 px** |
| chips and toggles only | `Full` | pill |
| badges, tooltips | `ExtraSmall` | 4 px |
| (rarely) large modal sheets | `ExtraLarge` | 28 px |
| none | `None` | 0 |

Pill buttons everywhere (the former look) are retired as screens migrate (M-0062 and later).

### 3.5 Spacing, density and scale

**Grid.** 4 px (`SpacingTokens::base_unit`). `SpacingScale` has ten steps: `None` 0, `Dense` 2, `XSmall` 4, `Compact` 6, `Small` 8, `Medium` 12, `Standard` 16, `Large` 24, `XLarge` 32, `Huge` 48 px. `Dense` and `Compact` are the two small off-grid steps used by the density modes.

**Density** (`Density` resource, setting `density`):

| Mode | Control height | Spacing factor | Use |
| :--- | ---: | ---: | :--- |
| `Comfortable` (default) | 40 px | 1 | everyday |
| `Compact` | 32 px | 0.75 | many controls per screen |
| `Dense` | 24 px | 0.5 | the editor only (below the 32 px minimum target size, a documented exception) |

`SpacingScale::px_in(density)` / `val_in(density)` give the value in a mode (rounded, at least 2 px). The widgets adopt it as part B reworks them.

**UI scale** (`UiScalePercent`, setting `ui_scale`, 75-200 %, default 100): `apply_ui_scale` writes it into Bevy's `UiScale`, which multiplies every logical size including font sizes. The first-run suggestion from the display's pixel density is **open** (not implemented).

**Layout classes** (`LayoutClass` resource, updated by `update_layout_class` from the window width **divided by the UI scale**, i.e. in layout pixels): `Compact` < 900, `Medium` 900-1400, `Expanded` > 1400. `LayoutClass::page_margin()` gives 16 / 24 / 32 px. The class is what navigation and master-detail screens read (M-0062).

**Window.** The minimum is **1024×640** logical px (`opts_cli::MIN_WINDOW_SIZE`, used for `resize_constraints`; `--window-size` smaller than that is raised to it).

### 3.6 Motion

Durations (D-UI: hover and focus 100 ms, panels 250 ms, screens 400 ms) are consumed by a transition system (`theme::motion`):

| `MotionRole` | Duration token | Easing | Application |
| :--- | :--- | :--- | :--- |
| `HoverFocus` | `Short` 100 ms | Standard `cubic-bezier(0.2, 0, 0, 1)` | hover, focus, press feedback |
| `Panel` | `Medium` 250 ms | Standard | menus, dialogs, panels |
| `Screen` | `Long` 400 ms | Emphasized `cubic-bezier(0.05, 0.7, 0.1, 1)` | screen changes |

`BackgroundTransition { from, to, role }` on an entity with a `BackgroundColor` is driven by `drive_background_transitions` on **real** time (interface motion keeps running while the simulation is paused) and removes itself when done. **Reduced motion** (`MotionPreference`: the `reduced_motion` setting *or* the OS hint) sets `MaterialTheme::motion.reduced_motion`, after which every duration is zero and a transition jumps to its target in the same frame. The OS hint is read once at start-up (`gsettings` on Linux/GNOME; `OPTS_REDUCED_MOTION=1` forces it; other desktops report "no preference"). The theme switch itself is **not** animated: it is one frame.

### 3.7 Instrument tokens

A separate token set for HUD instruments (`theme::instrument`, `MaterialTheme::instruments`), independent of the application theme but never hard-coded in a widget:

| Token | Day | Night |
| :--- | :--- | :--- |
| `panel` (the one legitimate use of glass) | `rgba(13, 15, 20, 0.78)` | `rgba(5, 5, 8, 0.82)` |
| `text`, `text_dim` | near-white, light grey | warm grey (dimmer) |
| `needle`, `ok`, `warning`, `alarm`, `track` | bright blue, green, amber, red, grey | the same hues at lower brightness |
| sizes | speed 48 (`NumericLarge`), gear 32 (`NumericMedium`), panel radius 12, padding 12 | same |

The variant (`InstrumentVariant`) follows the session's time of day (`update_instrument_variant`: night when the sky model needs vehicle lights, sun below 3°). Entities carry `InstrumentPanel` / `InstrumentInk` markers; `sync_instrument_tokens` paints them and repaints on a variant or theme change. **Guarantee, tested:** every instrument colour keeps its contrast (4.5:1 for text, 3:1 for lamps) over a white, a black and a sky-blue backdrop, in both variants.

### 3.8 Text over images

`theme::scrim` computes the veil over a photograph instead of using a fixed alpha: `luminance_stats` measures the image (5th and 95th percentile of relative luminance), `scrim_for_text` returns the theme's scrim colour (`Scrim` black in dark themes, `Surface` in light ones) with the **smallest alpha (floor 0.25, at most 0.95)** for which the text role still reaches 4.5:1 over the image's worst-case pixel. The main-menu background uses it (`menu_scrim_color`, re-evaluated on every theme change); tests use a white, a black and a mid-grey synthetic image in all three themes.

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

```rust
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct MaterialTheme {
    pub scheme: ColorScheme,          // 46 roles, carries its ThemeKind
    pub typography: TypographyTokens, // the 9 styles
    pub elevation: ElevationTokens,
    pub shape: ShapeTokens,
    pub spacing: SpacingTokens,
    pub motion: MotionTokens,         // reduced_motion
    pub instruments: InstrumentTokens,
}
```

`MaterialTheme::new(kind, accent)`, `::dark()`, `::light()`, `::high_contrast()`, `with_scheme`, a builder, and `kind()` / `instrument(variant)` accessors. When it changes, the sync systems re-skin every tagged entity in the same frame.

### 5.2 Token Marker Components

`BackgroundRole`, `ForegroundRole`, `BorderRole` (colours), `TypeStyleToken` (+ optional `TypeWeight`), `RadiusToken`, `ElevationToken` (tint **and** shadow), and for HUD instruments `InstrumentPanel` and `InstrumentInk` (in `opts-game`).

### 5.3 Reactive Sync Systems

| System | Reads | Writes |
| :--- | :--- | :--- |
| `sync_background_roles` | `BackgroundRole`, `ElevationToken` | `BackgroundColor` (role colour + tonal tint) |
| `sync_border_roles`, `sync_foreground_roles` | `BorderRole`, `ForegroundRole` | `BorderColor`, `TextColor` |
| `sync_radius_tokens` | `RadiusToken` | `Node::border_radius` |
| `sync_typestyle_tokens` | `TypeStyleToken`, `TypeWeight` | `TextFont` (size, weight, family stack, tabular figures), `LineHeight`, `LetterSpacing` |
| `sync_elevation_shadows` | `ElevationToken` | `BoxShadow` |
| `apply_ui_font` | `TextFont` | moves text on Bevy's default font onto the UI family stack |
| `clamp_text_size` | `MinTextSize` | raises pixel sizes below 12 (interim guard) |
| `drive_background_transitions` | `BackgroundTransition`, `Time<Real>` | `BackgroundColor` |
| `apply_ui_scale`, `update_layout_class` | `UiScalePercent`, `Window` | `UiScale`, `LayoutClass` |

### 5.4 Plugin Composition

```text
OptsGuiPlugin
├── ThemePlugin                      (theme::plugin)
│   ├── PreUpdate: track_system_theme → resolve_theme → sync_motion_preference → apply_ui_scale
│   └── Update: update_layout_class, drive_background_transitions, sync_elevation_shadows,
│               apply_ui_font, clamp_text_size
├── token sync systems               (BackgroundRole, BorderRole, ForegroundRole, Radius, TypeStyle)
├── focus / interaction systems      (part B reworks them)
└── widget plugins                   (modal, text area, button, checkbox, ...)
```

Resources `ThemePlugin` registers: `ThemePreferences`, `ThemeSwitchLock`, `SystemColorScheme`, `ThemeClock`, `UiScalePercent`, `MinTextSize`, `Density`, `LayoutClass`, `MotionPreference`, `InstrumentVariant`. A `MaterialTheme` inserted before the plugin is respected: the preferences start out matching it.

---

## 6. Theming Strategy & Accessibility

### 6.1 The three palettes

`ThemeKind::{Dark, Light, HighContrast}`. `ColorScheme::for_kind(kind, accent)` builds any of them; `dark()`, `light()` and `high_contrast()` use the default accent. **High contrast** has a black surface and white text (7:1), white outlines (4.5:1 and well above), and brightened accent and status colours; it is a dark-based palette (a light high-contrast variant is open).

### 6.2 Theme modes and runtime switching

`ThemeMode::{Dark (default), Light, System, AutoClock}` (setting `theme_mode`: `dark`, `light`, `system`, `auto`) plus the separate `high_contrast` switch, which overrides the mode and is **never** turned on automatically.

* **System:** follows `Window::window_theme` and `WindowThemeChanged` (`track_system_theme`). **Verified on the development machine (GNOME on Wayland): winit reports no window theme there**, so `opts-game` additionally asks the desktop (`gsettings get org.gnome.desktop.interface color-scheme`, on a worker thread, at once and then once a minute while System is active and the window reports nothing: `prefer-dark` is dark, `prefer-light` is light, `default` means no preference). When nothing is reported the theme stays **dark**. Not verified: a live change of the desktop setting on a running game (the code path is the same message/resource, unit-tested).
* **Auto (clock):** `theme_for(time, light_from, dark_from)` is a pure interval rule (light from `light_from` inclusive until `dark_from` exclusive; wraps midnight; equal times mean dark). It is tested for midnight wrap, equal times and both daylight-saving days (spring day 23 h, autumn day 25 h, built from real UTC instants; no equality-on-the-switch-minute, so a skipped or repeated hour cannot flap or miss a switch). `ThemeClock` is the device's local time (`chrono`, timezone and DST from the operating system) and is replaceable in tests. Times: `auto_light_from` (default 07:00), `auto_dark_from` (19:00).
* **When it switches** (`resolve_theme`): an explicit change of the preferences applies **at once**; an automatic change (clock, OS) is evaluated **once per minute** and held back while a **modal dialog** is open (`ModalRoot` exists) or `ThemeSwitchLock` is set. The game sets the lock in `AppState::Loading` and `InGame` (loading screen and drive, `lock_theme_switching`); the change happens when the obstacle is gone.
* **Quick toggle:** the floating button and **F2** flip Dark/Light (clearing high contrast).

### 6.3 Accents, theme files and what was removed

**Accent presets** (`AccentPreset`: Transit blue (default), Amber, Violet, Emerald, Teal; setting `active_theme` keeps the old names `Default`, `Amber`, ...) are seeds, not colour tables. The generator guarantees contrast for every preset in every theme (§3.1). The former per-widget colour overrides (`widget_color_overrides` table, `widget:*` keys, the swatches in the settings) and the unused `custom_themes` table are **removed**: old databases keep working because the keys and tables are simply no longer read or written.

**Theme files** (`theme::file`, RON, in the `themes/` folder of the configuration directory, chosen by the `theme_file` setting) replace both:

```ron
(
    version: 1,                                   // the only version this build reads
    name: "Harbour",
    accent: Some("#2A7F62"),                      // optional; replaces the preset
    dark:  { "Surface": "#0E1A22" },              // optional per-theme role overrides
    light: {},
    high_contrast: {},
)
```

Role names are the `ColorRole` variant names. A file goes through the contrast checker (`audit` + `auto_correct`): a pair that fails is **corrected** (the foreground is moved in lightness, the corrections are reported in the log); a file that does not parse, has another version, an unknown role, a malformed colour, or colours that cannot be made to work (for example a near-white `Surface` in the dark theme) is **rejected** with a message and the built-in theme is used.

### 6.4 Settings keys and migration

| Key | Values | Default |
| :--- | :--- | :--- |
| `theme_mode` | `dark`, `light`, `system`, `auto` | `dark` |
| `auto_light_from`, `auto_dark_from` | `HH:MM` | `07:00`, `19:00` |
| `high_contrast` | `true`/`false` | `false` |
| `active_theme` | `Default`, `Amber`, `Violet`, `Emerald`, `Teal` | `Default` |
| `theme_file` | file stem in `themes/`, empty for none | empty |
| `ui_scale` | 75-200 (percent) | `100` |
| `density` | `comfortable`, `compact`, `dense` | `comfortable` |
| `reduced_motion` | `true`/`false` | `false` |

**Migration.** A database or `settings.toml` **without** `theme_mode` takes it from the old `is_dark_theme` (`true`: dark, `false`: light); with neither key the theme is dark. A present `theme_mode` always wins, whatever the row order. Unreadable values fall back to the defaults. The old key is never written again. (A user who last chose light before M-0061 keeps light; a fresh install is dark.)

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

> **Components and interaction (M-0061 part B).** States, activation, focus, actions, text input, UI ids, "Feature not available yet!", localisation and the CI rules are specified in [GUI Interaction Layer and Components](gui-interaction-layer.md); that page overrides the component sections above where they differ.
