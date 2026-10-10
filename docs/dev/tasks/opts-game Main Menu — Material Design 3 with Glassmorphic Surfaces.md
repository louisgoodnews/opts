# 🎬 Main Menu — Material Design 3, Glassmorphic, Map-Backed

You are building the first real UI screen for OPTS: the **main menu**.
It lives in `opts-game` and is the first consumer of the `opts-gui`
widget library.

The menu has three defining visual characteristics:

1. **Material Design 3 baseline palette.** The default theme follows
   the M3 baseline color scheme (soft sage, cream, deep green).
2. **Glassmorphic surfaces.** UI elements are semi-transparent so
   the background shines through them.
3. **Map-backed background.** The background image is loaded from
   the last loaded map's `picture.jpg`, not from a bundled asset.

This task has three purposes:

1. **Ship the main menu.** A working, themed, navigable entry screen.
2. **Validate `opts-gui`.** The menu exercises the library's Tier 1
   components against these visual requirements. Gaps will surface.
3. **Produce a review artifact.** The user reviews this menu before
   any other screen is built.

**Hard stop after this menu.** Do not build Settings, Vehicle
Selection, Map Selection, Load Game, or any other screen.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Target crate
`crates/opts-game/`

### GUI library
`crates/opts-gui/` — read its public API before writing any code.

### Output targets

- `crates/opts-game/src/menu/` — the menu module tree
- `crates/opts-game/examples/main_menu.rs` — a runnable example
- `docs/src/architecture/main-menu.md` — the design note
- `docs/crates/opts-game.md` — updated

### Related context (read first)
- `docs/src/architecture/gui-design-system.md`
- `docs/crates/opts-gui.md`
- `docs/src/adr/0001-why-bevy-and-rust.md`
- `docs/src/contributing/standards.md`

---

## 2. Design Reference

The visual design is grounded in three reference screenshots
provided by the project lead. They are stored in the conversation
history and in `docs/src/assets/design-references/` if committed.

| Reference | Informs |
| :--- | :--- |
| **M3 baseline palette** (screenshot 3) | **Definitive** for colors, shapes, corner radii, typography |
| **Material Dash** (screenshot 1) | Layout inspiration: sidebar patterns, card elevation, icon treatment |
| **Crema Demo** (screenshot 2) | Layout inspiration: top bar, search field, notification badges |

**Only screenshot 3 is prescriptive.** Screenshots 1 and 2 are
informative — they show Material-style UI patterns that inform the
menu's visual language, but their specific colors, layouts, and
component counts are not requirements.

### M3 Baseline Palette (Screenshot 3)

The default theme must match the M3 baseline "monochrome" palette:

- **Surface / background:** warm cream (`#FEF7FF` family)
- **Primary:** soft sage green (`#4F6354` family)
- **On-primary:** pale cream
- **Secondary / accents:** muted olive greens
- **Outline:** subtle grey-green

If `opts-gui`'s existing light theme doesn't match this palette,
**document the gap** — do not silently change `opts-gui`'s theme
(this is a follow-up task).

### Shapes and Radii

From screenshot 3:

- Buttons: **fully rounded (pill)** — `Radius::Full`
- Cards: **large radius** — `Radius::Large`
- Text fields: **medium radius** — `Radius::Medium`
- Icon buttons: **circular** — `Radius::Full`

### Typography

M3 baseline uses a clean geometric sans. Bevy's default font is
acceptable for v1. Document font substitution as a follow-up if the
look diverges noticeably.

---

## 3. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not **commit** any OMSI asset to the repository. Loading
  `picture.jpg` at runtime from the user's installation is
  permitted; copying it into the repo is not.
- Do not build screens beyond the main menu.
- Do not hardcode colors, sizes, or typography. All styling comes
  from `opts-gui` theme tokens.
- Do not modify `opts-gui` in this task. Gaps are documented, not
  fixed. (If a gap is truly blocking, STOP and report.)
- Do not write tests that depend on the OMSI installation.

### ✅ Permitted

- Read all of `opts-gui`'s source.
- Read all format specs for context.
- Add `opts-game` dependencies via the workspace.
- Load the map `picture.jpg` at runtime from a configured path.
- Create the menu module tree.
- Create the example binary.
- Produce the design note and update crate docs.

---

## 4. Preconditions

Verify and report before starting:

1. `crates/opts-gui/` exists with Button, Label, Icon, Card,
   Divider, TextField, Checkbox, Switch, Radio.
2. `crates/opts-game/` exists and is a workspace member.
3. `opts-gui` supports runtime theme switching.
4. `docs/src/architecture/gui-design-system.md` exists.
5. `crates/opts-gui/examples/gallery.rs` runs.

If any precondition fails, STOP and report.

---

## 5. Part A — Library Discovery

Determine what `opts-gui` provides — and specifically, whether it
can meet the three visual requirements.

### Stage A1 — Component API Survey

Read `crates/opts-gui/src/` in full. Produce a table:

| Component | Variants | Builder | Events | Disabled state | Icon support |
| :--- | :--- | :--- | :--- | :---: | :---: |

### Stage A2 — Theme Survey

Read `crates/opts-gui/src/theme/`. Produce a table:

| Token group | Types | Access | Runtime mutable |
| :--- | :--- | :--- | :---: |

**Specifically verify:**

- Does `ColorScheme` include the M3 baseline palette, or a
  different one?
- Does any color role support **alpha transparency** below 1.0?
- Is there a `SurfaceGlass` or similar translucent token?

### Stage A3 — Visual Capability Gap Report

This is the critical stage. Check whether `opts-gui` supports:

| Capability | Needed for | Status |
| :--- | :--- | :--- |
| Translucent node backgrounds (alpha < 1.0) | Glassmorphism | ? |
| Backdrop blur (blur behind UI surfaces) | Glassmorphism (stretch) | ? |
| Full pill radius on buttons | M3 shape language | ? |
| Hover state color shift | Interaction requirement | ? |
| Pressed state color shift | Interaction requirement | ? |
| Ripple effect on click | M3 interaction (stretch) | ? |
| Runtime image loading for backgrounds | Map-backed background | ? |
| Alpha blending for overlay on image | Map-backed background | ? |

For each, classify:

- **Available** — `opts-gui` provides it today
- **Partial** — some capability exists but needs augmentation
- **Blocking** — cannot be built without it
- **Stretch** — nice to have, skip if not available

### Stage A4 — Blocking Decision

If any capability is **Blocking**, STOP and report. The menu cannot
proceed until `opts-gui` is extended. The user will decide whether
to:
- Extend `opts-gui` first (separate task)
- Accept a reduced-scope menu (documented as such)
- Skip this menu for now

If nothing is Blocking, proceed to Part B with the gap list
attached.

### Stage A5 — Checkpoint

Report the surveys and gap report.

**Wait for confirmation before Part B.**

---

## 6. Part B — Design Brief

Produce `docs/src/architecture/main-menu.md` **before any code**.

### Stage B1 — Menu Scope

Fixed entry list:

| Entry | Action (stubbed) |
| :--- | :--- |
| **Play** | Transition to `VehicleSelection` state |
| **Load** | Disabled if no save exists; else transition to `LoadGame` |
| **Settings** | Transition to `Settings` state |
| **Credits** | Transition to `Credits` state |
| **Quit** | Send `AppExit` |

Every action is a stub. No real functionality.

### Stage B2 — Layout

**Structure:**

```
RootNode (fills viewport)
├── BackgroundImageNode (z=0)
│   ├── Image (map picture.jpg, or theme fallback)
│   └── Scrim (overlay with alpha, ensures text contrast)
├── ContentNode (z=1, centered)
│   ├── BrandNode
│   │   ├── Title ("OPTS")
│   │   └── Subtitle ("Open Public Transport Simulator")
│   └── MenuNode
│       └── Button × 5
├── ThemeToggleNode (top-right, circular icon button)
└── VersionLabelNode (bottom-right)
```

**Responsive behavior:** Content stays centered. Buttons have a
min/max width. Window resize does not break layout.

**Minimum window size:** 800×600.

### Stage B3 — Background (Map-Derived)

The background is loaded from the last map's `picture.jpg`.

**Data flow:**

1. `opts-game` reads a `recent_map_path` from a settings resource.
2. If present, the menu loads `<recent_map_path>/picture.jpg` via
   Bevy's asset system.
3. If absent, or the image fails to load, fall back to a solid
   theme color (`ColorRole::Surface`).

**For the example binary:** accept a `--map-path <path>` CLI arg
that overrides the settings resource. This is how the user reviews
glassmorphism against a real map background.

**Scrim requirement:** A semi-transparent overlay sits between the
image and the UI content. Its alpha is a theme token. Purpose:
ensure text remains readable regardless of the underlying image.

**Scrim color:** `ColorRole::Scrim` or `ColorRole::Surface` at
~40-60% alpha.

### Stage B4 — Glassmorphism

Every interactive surface (buttons, cards, theme toggle) uses a
**translucent background** so the map image shines through.

**Specification:**

| Surface | Alpha | Backdrop blur | Source |
| :--- | :--- | :--- | :--- |
| Button (filled) | 0.75–0.85 | Yes if available | `ColorRole::Primary` + alpha |
| Button (outlined) | 0.20–0.35 | Yes if available | `ColorRole::Outline` + alpha |
| Card | 0.65–0.80 | Yes if available | `ColorRole::SurfaceVariant` + alpha |
| Theme toggle | 0.70 | Yes if available | `ColorRole::Surface` + alpha |

**If `opts-gui` does not support alpha on color roles:** document
this as the top-priority follow-up. The menu must still render, but
glassmorphism is reduced to whatever alpha the library provides.

**If `opts-gui` does not support backdrop blur:** proceed with
translucency only. Blur is a stretch goal. Document it.

### Stage B5 — Interaction Effects

Every button requires three visual states beyond rest:

| State | Visual change | Duration |
| :--- | :--- | :--- |
| **Hover** | Background lightens ~10%, outline brightens, cursor changes | 100ms ease |
| **Pressed** | Background darkens ~15%, subtle scale-down (0.98×) | 50ms |
| **Focus** | Focus ring appears (2px, offset 2px, `ColorRole::Primary`) | 100ms |

**Ripple effect:** Optional. If `opts-gui` supports it, enable it
for filled buttons. Otherwise document as a follow-up.

**Disabled state:** No hover, no press, opacity reduced to 40%.

### Stage B6 — Title and Branding

- Title: "OPTS" (large, `DisplayLarge` type style)
- Subtitle: "Open Public Transport Simulator" (`TitleMedium`)
- Version: bottom-right, `LabelSmall`, `ColorRole::OnSurfaceVariant`
- Build hash: optional, next to version

### Stage B7 — Theme

- **Default theme:** M3 baseline light (cream/sage)
- **Theme toggle:** circular icon button, top-right corner
- **Toggle icon:** sun for light, moon for dark
- **Persist choice:** If `opts-gui` supports persistence, use it.
  Otherwise, hold in memory for the session and document.

### Stage B8 — State Machine

```
AppState:
  - Loading
  - MainMenu          <-- this task
  - VehicleSelection  (stub: renders "Not yet implemented")
  - LoadGame          (stub)
  - Settings          (stub)
  - Credits           (stub)
  - InGame            (stub)
```

App starts in `Loading`, then transitions to `MainMenu` after
assets load.

Stub states render a centered label and accept Escape to return.

### Stage B9 — Checkpoint

Report the design brief.

**Wait for confirmation before Part C.**

---

## 7. Part C — Implementation

### Stage C1 — Module Tree

```
crates/opts-game/src/
├── lib.rs
├── main.rs
├── app_state.rs
├── settings.rs               # recent_map_path persistence
├── menu/
│   ├── mod.rs
│   ├── plugin.rs
│   ├── layout.rs
│   ├── entries.rs
│   ├── background.rs         # map picture loading + scrim
│   ├── glass.rs              # translucent surface helpers
│   ├── interactions.rs       # hover/press/focus systems
│   ├── theme_toggle.rs
│   └── systems.rs
└── stub_screens/             # placeholder screens
    └── mod.rs
```

### Stage C2 — `AppState` and `Settings`

- Define `AppState` per Stage B8.
- Define `GameSettings` resource with `recent_map_path:
  Option<PathBuf>`.
- Load/save settings from `~/.config/opts/settings.toml` (Linux),
  `%APPDATA%\opts\settings.toml` (Windows), etc.
- Use the `directories` crate for path resolution.

### Stage C3 — Background Loading

- System loads `picture.jpg` from `GameSettings.recent_map_path`.
- Insert as `Image` node at z=0, scaled to cover the viewport.
- Add scrim node at z=1 with theme alpha.
- On load failure: fall back to `ColorRole::Surface`.
- Log all load attempts via `tracing`.

### Stage C4 — Menu Layout

Build the entity tree from Stage B2. Every widget is from
`opts-gui`. No raw `Node`/`Style` unless `opts-gui` provides no
wrapper.

### Stage C5 — Glassmorphic Surfaces

Apply translucent backgrounds per Stage B4:

- Where `opts-gui` supports alpha on `ColorRole`, use it.
- Where it doesn't, use `BackgroundColor` directly with
  `Color::srgba(...)` derived from the theme token.
- Document every place alpha had to be applied manually.

### Stage C6 — Menu Entries

Each entry:

1. Constructs an `opts-gui` `Button`.
2. Attaches an `on_click` handler.
3. Sets enabled/disabled state (Load disabled when no save).
4. Participates in keyboard navigation.

Handlers are small functions. No business logic.

### Stage C7 — Interaction Effects

Implement per Stage B5:

- Hover: watch for `Interaction::Hovered`, apply hover color.
- Pressed: watch for `Interaction::Pressed`, apply pressed color.
- Focus: track focused widget, render focus ring.
- Cursor: change to pointer on hover if Bevy supports it.

Every system must read colors from theme tokens. No hardcoded
values.

### Stage C8 — Theme Toggle

Implement per Stage B7. Verify the menu updates without restart.

### Stage C9 — Stub Screens

Implement each stub screen as a single label with a "return" hint.
They exist only to prove the state transitions work.

### Stage C10 — Example Binary

`crates/opts-game/examples/main_menu.rs`:

- Accepts `--map-path <path>` to load a specific `picture.jpg`.
- Falls back to solid theme color if no path provided.
- Renders the menu.
- Supports keyboard, mouse, and theme toggle.
- Exits on Quit.

### Stage C11 — Runtime Verification

Manually run the example with a real map path. Verify:

- [ ] Menu renders
- [ ] Background image covers the viewport
- [ ] Scrim ensures text is readable
- [ ] Buttons are translucent (background visible through them)
- [ ] Hover state is visible
- [ ] Pressed state is visible
- [ ] Focus ring is visible
- [ ] Disabled Load refuses interaction
- [ ] Theme toggle flips colors
- [ ] Window resize doesn't break layout
- [ ] Quit exits cleanly

Document any failures in the design note.

### Stage C12 — Checkpoint

Report the implementation and runtime verification.

**Wait for confirmation before Part D.**

---

## 8. Part D — Review Artifact

### Stage D1 — Design Note Update

Add "Implementation Notes" to `docs/src/architecture/main-menu.md`:

- Deviations from the design brief
- `opts-gui` gaps encountered
- Workarounds used
- Any visual requirements that couldn't be met

### Stage D2 — Screenshots

Capture screenshots of:

1. Menu with map background, light theme
2. Menu with map background, dark theme
3. Menu with solid fallback background
4. Focus ring visible on a button
5. Disabled Load button
6. A stub screen

Place under `docs/src/assets/main-menu/`. If headless, produce
ASCII diagrams.

### Stage D3 — Update `docs/crates/opts-game.md`

Cover the `AppState` machine, the menu module tree, and how a
future screen is added.

### Stage D4 — Update `docs/src/SUMMARY.md`

Add `docs/src/architecture/main-menu.md` under Architecture.

### Stage D5 — Final Report

Summarize:

- What was built
- `opts-gui` components used
- `opts-gui` gaps found (especially for glassmorphism)
- Deviations from the design brief
- Files created and modified
- Recommendations for the next screen

**This is the end of the task.** Do not build the next menu.

---

## 9. Reporting Cadence

1. Part A (discovery + gap report) — approval before Part B
2. Part B (design brief) — approval before Part C
3. Part C (implementation) — approval before Part D
4. Part D (review artifact) — end of task

Format:

```
## Main Menu — Checkpoint N

**Phase:** <name>
**Deliverables:** <summary>
**Design decisions:** <bulleted list>
**opts-gui gaps:** <bulleted list>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 10. Exit Criteria

- ✅ Menu renders with map-backed background (or documented fallback).
- ✅ Glassmorphic surfaces (translucent or documented gap).
- ✅ Hover, pressed, and focus states work.
- ✅ Theme toggle flips M3 baseline light/dark at runtime.
- ✅ `cargo run -p opts-game --example main_menu -- --map-path ...`
  runs.
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ Design note exists with implementation notes.
- ✅ Screenshots or ASCII diagrams included.
- ✅ `docs/crates/opts-game.md` and `docs/src/SUMMARY.md` updated.
- ✅ No other menu was built.
- ✅ No `opts-gui` component was added.
- ✅ No OMSI asset committed to the repo.
- ✅ No hardcoded colors or sizes.
- ✅ Every file under 500 lines.

When finished, respond with the summary from Stage D5.

Then respond with the exact string:
"MAIN MENU COMPLETE — ready for user review"