# 🎬 Main Menu — First `opts-gui` Consumer

You are building the first real UI screen for OPTS: the **main menu**.
It lives in `opts-game` and is the first consumer of the `opts-gui`
widget library.

This task has three purposes:

1. **Ship the main menu.** A working, themed, navigable entry screen.
2. **Validate `opts-gui`.** The menu exercises the library's Tier 1
   components in a real screen. Gaps in the library will surface here.
3. **Produce a review artifact.** The user will review this menu
   before any other screen is built. It must be runnable, screenshot-
   able, and described in a written design note.

**Hard stop after this menu.** Do not build Settings, Vehicle
Selection, Map Selection, Load Game, or any other screen. Those come
after user review.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Target crate
`crates/opts-game/`

### GUI library
`crates/opts-gui/` — the widget library built in the previous task.
Read its public API before writing any menu code.

### Output targets

- `crates/opts-game/src/menu/` — the menu module tree
- `crates/opts-game/examples/main_menu.rs` — a runnable example
- `docs/src/architecture/main-menu.md` — the design note
- `docs/crates/opts-game.md` — updated with menu overview

### Related context (read first)
- `docs/src/architecture/gui-design-system.md` — the design system
- `docs/crates/opts-gui.md` — the library documentation
- `docs/src/vision.md` — where the menu fits in the vision
- `docs/src/adr/0001-why-bevy-and-rust.md`
- `docs/src/contributing/standards.md`
- `docs/src/glossary.md`

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not use any OMSI asset (image, mesh, sound) as the menu
  background or decoration. All visuals must be self-authored or
  generated from theme tokens.
- Do not build screens beyond the main menu.
- Do not hardcode colors, sizes, or typography. All styling comes
  from `opts-gui` theme tokens.
- Do not add new components to `opts-gui` in this task. If the
  library is missing a component, use an available one or note the
  gap in the design note.
- Do not write tests that depend on the OMSI installation.
- Do not modify any format spec.

### ✅ Permitted

- Read all of `opts-gui`'s source.
- Read all format specs for context.
- Add `opts-game` dependencies on `opts-gui`, `opts-bevy`, and
  `bevy` via the workspace.
- Create the menu module tree.
- Create the example binary.
- Produce the design note and update crate docs.
- Add a background asset (self-authored, generated, or procedural)
  if the menu needs one.

---

## 3. Preconditions

Verify and report before starting:

1. `crates/opts-gui/` exists with at least these components:
   Button, Label, Icon, Card, Divider, TextField, Checkbox,
   Switch, Radio.
2. `crates/opts-game/` exists and is a workspace member.
3. `opts-gui`'s theme system supports light and dark schemes.
4. `docs/src/architecture/gui-design-system.md` exists.
5. `docs/crates/opts-gui.md` exists.
6. The example `crates/opts-gui/examples/gallery.rs` runs.

If any precondition fails, STOP and report. Do not attempt to fill
gaps in `opts-gui` as part of this task.

---

## 4. Part A — Library Discovery

Before designing the menu, determine what `opts-gui` actually
provides.

### Stage A1 — API Survey

Read `crates/opts-gui/src/` in full. Produce a table:

| Component | Variants | Builder API | Events | Notes |
| :--- | :--- | :--- | :--- | :--- |

For each component, note:
- How it's constructed (builder chain? spawn bundle?)
- How interactions are captured (event? observer? polled component?)
- Whether it supports a disabled state
- Whether it supports icons alongside text

### Stage A2 — Theme Survey

Read `crates/opts-gui/src/theme/`. Produce a table:

| Token group | Types | Access pattern | Runtime mutability |
| :--- | :--- | :--- | :--- |

For each group, note:
- How to read a token from a system or builder
- Whether the theme can change at runtime
- Whether changes propagate to live widgets

### Stage A3 — Gap Report

List what `opts-gui` **does not** provide that the main menu needs.
For each gap, classify:

- **Blocking** — the menu cannot be built without this
- **Workable** — a workaround exists using available components
- **Cosmetic** — the menu will look slightly worse, but function

If any gap is Blocking, STOP and report. The library must be extended
before the menu can proceed — that is a separate task.

### Stage A4 — Checkpoint

Report the survey and the gap analysis.

**Wait for confirmation before Part B.**

---

## 5. Part B — Design Brief

Produce the design note at `docs/src/architecture/main-menu.md`
**before writing any menu code**. The note is the specification the
implementation follows.

### Stage B1 — Menu Scope

The main menu has a fixed set of entries. State them explicitly.
Recommended set:

| Entry | Purpose | Action (stubbed for now) |
| :--- | :--- | :--- |
| **Play** | Start a new game session | Transition to `VehicleSelection` state (stub) |
| **Load** | Load a saved game | Transition to `LoadGame` state (stub) |
| **Settings** | Open the settings screen | Transition to `Settings` state (stub) |
| **Credits** | Show attribution and licenses | Transition to `Credits` state (stub) |
| **Quit** | Exit the application | Send `AppExit` event |

Adjust this list if the survey suggests a better set. Every entry
must be justified. Every action must be a stub that transitions to a
placeholder state or a logged event — no real functionality.

### Stage B2 — Layout

Decide the layout. Consider:

- **Centered card** with vertical button column
- **Hero area** with a title and a button column below
- **Navigation rail** on the left (overkill for v1; document why
  if chosen)
- **Full-bleed background** with overlay content

Recommendation: **hero area with button column**. Justify or refute.

Document:
- The exact layout structure (entity tree)
- Spacing (which tokens)
- Alignment (which tokens)
- Responsive behavior (window resize)
- Minimum window size

### Stage B3 — Background

Decide what the menu sits on:

- **Solid color** — from a theme token (`surface`, `background`)
- **Gradient** — two theme tokens blended
- **Procedural pattern** — generated (no assets)
- **Self-authored static image** — placed under `assets/gui/`
- **Placeholder image** — replaced later by a real one

Recommendation: **gradient or procedural pattern**. It avoids asset
dependencies and stays theme-aware. If a static image is chosen,
document where it lives and why.

### Stage B4 — Title and Branding

Decide:
- What the menu title is ("OPTS" or "Open Public Transport Simulator")
- Whether a subtitle appears
- Where the version number appears (bottom? top?)
- Whether the build hash appears (useful for bug reports)

Recommendation: title centered above the buttons; version bottom-right
in a `Label` with a small type style.

### Stage B5 — Interactions

Document:
- Keyboard navigation (tab order, enter to activate, escape to
  cancel)
- Mouse hover and click behavior
- Focus visible indicator
- Disabled state (used for Load when no save exists)

Recommendation: **Load** is disabled until a save exists. Since
saves don't exist yet, it renders disabled.

### Stage B6 — Theme

Document:
- Which theme is default (light or dark)
- Whether the menu exposes a theme toggle
- How the toggle interacts with the theme resource

Recommendation: **dark by default**, with a theme toggle accessible
via a small icon button. This proves runtime theme switching works.

### Stage B7 — State Machine

Document the Bevy state machine:

```
AppState:
  - Loading
  - MainMenu      <-- this task builds this
  - VehicleSelection   (stub)
  - LoadGame           (stub)
  - Settings           (stub)
  - Credits            (stub)
  - InGame             (stub)
```

Define:
- Which state the app starts in
- How transitions occur
- What happens on `OnEnter(MainMenu)` and `OnExit(MainMenu)`

### Stage B8 — Checkpoint

Report the design brief. The user reviews it before any code.

**Wait for confirmation before Part C.**

---

## 6. Part C — Implementation

Build the menu following the design brief. Do not deviate from the
brief without documenting the reason in the design note.

### Stage C1 — Module Tree

Create:

```
crates/opts-game/src/
├── lib.rs                    # Library entry (if not present)
├── main.rs                   # Binary entry
├── app_state.rs              # AppState enum and transition helpers
├── menu/
│   ├── mod.rs
│   ├── plugin.rs             # MainMenuPlugin
│   ├── layout.rs             # The menu entity tree
│   ├── entries.rs            # The menu entry definitions
│   ├── background.rs         # Background rendering
│   ├── theme_toggle.rs       # Theme switcher (if included)
│   └── systems.rs            # Systems that drive the menu
└── assets/
    └── gui/                  # Self-authored GUI assets (if any)
```

### Stage C2 — `AppState`

Define the `AppState` enum with the states from the design brief.
Register it with the Bevy `App`.

Transitions:
- App starts in `Loading`
- On assets loaded, transition to `MainMenu`
- Menu entries transition to their target states (stubs)

Each stub state should render a "Not yet implemented" label and
allow returning to the main menu via Escape.

### Stage C3 — Background

Implement the background per the design brief. If gradient:
- Use `opts-gui`'s theme tokens for colors
- Update on theme change

If procedural pattern: similar. If static asset: load via Bevy's
asset system.

### Stage C4 — Menu Layout

Implement the entity tree from the design brief. Every widget comes
from `opts-gui`. No raw `Node` or `Style` unless `opts-gui` doesn't
expose a wrapper for it.

### Stage C5 — Menu Entries

Implement each entry. Each entry:

1. Constructs a `Button` from `opts-gui`.
2. Has an `on_click` handler.
3. Sets its enabled/disabled state based on runtime conditions.
4. Participates in keyboard navigation.

Handlers should be small functions that send the appropriate state
transition or event. No business logic.

### Stage C6 — Interactions

Implement:
- Tab/shift-tab keyboard navigation
- Enter/space activation
- Escape to close or return
- Hover state
- Focus visible indicator

If `opts-gui` provides focus navigation, use it. If not, document
the gap.

### Stage C7 — Theme Toggle

If the design brief includes a theme toggle, implement it. It should:

1. Flip the theme resource.
2. Verify the menu updates without a restart.
3. Persist the choice (using `opts-gui`'s theme persistence if
   available, or a simple config file if not).

### Stage C8 — Example Binary

Create `crates/opts-game/examples/main_menu.rs` — a minimal Bevy app
that loads the main menu on startup. This is the review artifact.

Requirements:
- Runs with `cargo run -p opts-game --example main_menu`
- Renders the menu
- Accepts keyboard and mouse input
- Supports theme switching
- Exits cleanly on Quit
- Prints log output for stubbed actions

### Stage C9 — Runtime Verification

Manually run the example. Verify:

- [ ] Menu renders
- [ ] All entries are visible
- [ ] Buttons respond to hover
- [ ] Buttons respond to click
- [ ] Buttons respond to keyboard
- [ ] Focus indicator is visible
- [ ] Disabled Load button refuses interaction
- [ ] Theme toggle flips the theme
- [ ] Window resize doesn't break layout
- [ ] Quit exits cleanly

Document any failures.

### Stage C10 — Checkpoint

Report the implementation and runtime verification.

**Wait for confirmation before Part D.**

---

## 7. Part D — Review Artifact

Produce the material the user reviews.

### Stage D1 — Update the Design Note

Add an "Implementation Notes" section to
`docs/src/architecture/main-menu.md` covering:

- Any deviations from the design brief and why
- Any `opts-gui` gaps encountered
- Any workarounds used
- Screenshots or ASCII diagrams of the final layout

### Stage D2 — Screenshots

Capture screenshots of:

1. The menu in default (dark) theme
2. The menu in light theme
3. A focused button (focus ring visible)
4. A disabled button (Load)
5. A stubbed destination screen

Place them under `docs/src/assets/main-menu/` or an equivalent
location, referenced from the design note.

If screenshots cannot be captured (headless environment), produce
ASCII diagrams of the layout instead.

### Stage D3 — Update `docs/crates/opts-game.md`

Add a section covering:

- The `AppState` machine
- The menu module tree
- How a future screen is added (the pattern)
- A note that the main menu is the first `opts-gui` consumer

### Stage D4 — Update `docs/src/SUMMARY.md`

Add `docs/src/architecture/main-menu.md` under Architecture.

### Stage D5 — Final Report

Produce a summary covering:

- What was built
- `opts-gui` components used
- `opts-gui` gaps found
- Deviations from the design brief
- Files created and modified
- Recommendations for the next menu

**This is the end of the task.** Do not build the next menu.

---

## 8. Reporting Cadence

Report after each of these checkpoints:

1. Part A (library discovery) — approval before Part B
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

Do not proceed without confirmation.

---

## 9. Exit Criteria

- ✅ `crates/opts-game/src/menu/` exists with the module tree.
- ✅ `AppState` is defined and registered.
- ✅ The main menu renders with all entries.
- ✅ All interactions work (hover, click, keyboard, focus,
  disabled, quit).
- ✅ Theme toggle flips the theme at runtime.
- ✅ `cargo run -p opts-game --example main_menu` runs successfully.
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ `docs/src/architecture/main-menu.md` exists with design brief
  and implementation notes.
- ✅ Screenshots or ASCII diagrams included.
- ✅ `docs/crates/opts-game.md` updated.
- ✅ `docs/src/SUMMARY.md` updated.
- ✅ No other menu was built.
- ✅ No `opts-gui` component was added.
- ✅ No OMSI assets were used.
- ✅ No hardcoded colors or sizes.
- ✅ Every file under 500 lines.
- ✅ Every public item documented.

When finished, respond with the summary from Stage D5.

Then respond with the exact string:
"MAIN MENU COMPLETE — ready for user review"