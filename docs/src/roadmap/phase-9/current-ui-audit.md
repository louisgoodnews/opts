# Audit of the Current UI

| Metadata | Value |
| :--- | :--- |
| **Status** | 📝 Evidence for the Phase 9 design |
| **Date** | 2026-10-12 |
| **Baseline** | `bevy-0.20` at `0a7bd11` (code), plus a build of `agent/visual-quality` for screenshots (it adds two settings rows) |
| **Method** | (1) a read-only inventory of `opts-gui` and `opts-game` with file references; (2) 17 screenshots taken through the control protocol (main menu, New Game ×3 tabs, Settings ×3 tabs, loading screen, HUD, F1 report) at 1600×900 and 1920×1045; (3) contrast ratios computed from the token values; (4) spot checks of the strongest claims directly in the code. Screenshots are not committed (they show OMSI content); they are kept outside the repository. |
| **Labels** | **V** verified in code or on screen · **I** inferred (stated why) |

## 1. What exists

| Area | State |
| :--- | :--- |
| **Library `opts-gui`** | 34 colour roles, 15 type styles, spacing, radius and tonal-elevation tokens, light/dark themes, live theme switching; 12 widgets (button, checkbox, radio, switch, text field, text area, combobox, notebook, toast, modal, toplevel, treeview) plus card, divider, label, icon, date selectors; row/stack/grid builders; Tab focus traversal and modal focus trap |
| **Screens (`opts-game`)** | main menu (6 controls), New Game (3 tabs, ≈ 30 controls and lists), Settings (3 tabs, 22 controls), Load Game and Credits (stubs), loading screen, HUD, F1 report modal, vehicle menu, OMSI-folder prompt, F10 debug overlay |
| **Flow** | `Loading → MainMenu → VehicleSelection → InGame`; Escape returns to the menu; the application quits only from the main menu |
| **Tests** | 8 ECS-level UI test files; none checks pixels, layout at a given size, or rendered contrast |

## 2. What works and should be kept

- **The token architecture** (reference → system → component tokens, marker components, one sync system per concern) is sound, themeable at runtime and the right base to extend.
- **Real progress and a manifest** on the loading screen (map, vehicle, environment), and a stage chip.
- **A clear primary action** on New Game (*Start Simulation*, bottom right, filled) and a persistent summary line.
- **Search in long lists** and entry-point chips; map photos as the menu background.
- **Modal discipline**: focus trap, Escape, input gating through `not_modal_active` with an audit test.
- **Diagnostics for users**: F1 report with screenshot, the control protocol for scripted UI checks.
- **Tab order and a visible focus ring exist on the main menu**: the pattern to generalise.

## 3. Findings

Severity: 🔴 breaks a principle in a way users will hit · 🟠 clear gap · 🟡 polish. *P* lists the principles of [report-analysis](report-analysis.md#4-the-principles).

| ID | Sev | Finding | Evidence | P |
| :--- | :---: | :--- | :--- | :--- |
| **UI-F01** | 🔴 | **Interaction feedback exists on one screen.** Hover, pressed, focus ring and Enter/Space activation are registered only for the main menu. Elsewhere Tab moves an invisible focus and Enter does nothing. `opts-gui` buttons have no hover/pressed/focus/disabled/error visuals; `Interaction::Hovered` does not appear in the library. | V `menu/plugin.rs:305-318`; grep in `opts-gui/src` | 04 06 08 10 |
| **UI-F02** | 🔴 | **Escape destroys.** On the main menu it quits at once; in game it drops the running session; the HUD's *Exit to Menu* does the same. There is no pause menu and no confirmation (the main-menu spec promises one). | V `menu/systems.rs:52-58`; screen test: Esc in game → `MainMenu` | 05 |
| **UI-F03** | 🔴 | **Load problems are hidden.** `ERROR: …` is overwritten by `Ready`; the screen auto-continues after 0.6 s; failed assets are visible only in the F10 counters. | V `loading_pipeline.rs:113-118,291` | 04 07 |
| **UI-F04** | 🟠 | **Settings promise more than they deliver.** *Display & Sound* is static text ("Resolution 1920×1080 · VSync · Backend WGPU", "Audio engine: spatial mixer initialized (Master Volume 100 %)") although audio is not implemented. No resolution, window mode, volume, key bindings or OMSI path can be changed. Two real rows were added on 2026-10-12 (texture filtering, look). | V `display_tab.rs`; screen 07 | 11 07 |
| **UI-F05** | 🟠 | **Developer copy in the product.** "Clean-Room Simulation Engine", "Storage: SQLite (WAL) · UI: Material Design 3 Glassmorphism", raw database paths, "Invariant: User modifications are committed exclusively to SQLite", "ASSET COMPILATION & WORLD SYNTHESIS", hard-coded version string. | V `menu/layout.rs:194,227`; screens 01, 06, 12 | 02 12 |
| **UI-F06** | 🟠 | **Things that look functional but are not.** *Load Last Game* is hard-coded disabled and looks identical to an enabled button; the "Live Device Location" panel shows constants (12 °C, 26 km/h, 91 %) as if measured, its query function is never called and *Refresh* only logs; RPM is always "--"; the vehicle detail grid is "N/A" ×8; *Start Driving Now* is nearly unusable because of the 0.6 s auto-launch. | V `entries.rs:414`; `live_weather.rs:139`; `hud.rs:367`; screens 03, 04 | 11 07 |
| **UI-F07** | 🟠 | **Fixed pixel layouts, no UI scale.** List viewports of 470/420/150/140 px, a 434 px scrollbar track, a 310×400 clock picker. Observed: the Weather tab's clock is cut off at 1600×900. Computed (I): New Game lists clip at the 800×600 minimum window. No `UiScale`, no font-size option. | V screen 04; grep `UiScale` = 0 | 10 |
| **UI-F08** | 🟠 | **Small targets and small text.** Scroll arrows 18×18, year arrows 16×11, colour swatches 24×24, checkbox box 18×18; hand-built buttons ≈ 29–37 px tall; 30 literal font sizes below 12 px. | V code; I heights | 01 10 |
| **UI-F09** | 🟠 | **Contrast failures.** Preset primaries against `OnPrimary` in light theme: Amber 2.15, Emerald 2.57, Teal 2.91; light-theme hero text over the scrimmed map photo 1.0–3.2; card/list borders (`OutlineVariant`) 1.60 (light) and 2.03 (dark); observed: the camera hint line in the HUD (blue on bright sky) is nearly invisible, the loading title is dark-on-dark. | V computed from tokens; screens 01, 09, 12 | 10 |
| **UI-F10** | 🟠 | **Whole screens bypass the theme.** Loading screen, HUD and stub screens use hard-coded dark colours while the application starts in light theme: the product looks like two applications. | V `loading_ui.rs:33`, `hud.rs:66`, `stub_screens/mod.rs:561` | 06 |
| **UI-F11** | 🟠 | **Keyboard operability is partial.** Notebook tabs, tree view, combobox and toasts are not focusable; `TextField` cannot take typed input; search typing relies on a Linux-only layout detector (no IME, caret movement or paste); the theme toggle, HUD exit and loading buttons are not focusable; all Tab indices are 0; no arrow-key navigation. | V `notebook.rs`, `text_field.rs`, `keyboard_layout.rs` | 08 10 |
| **UI-F12** | 🟠 | **Nothing helps the driver.** P only logs (no pause indicator); the one-line key legend omits L-Alt, F1, F10, 0, Shift+Space; the F10 debug block is on by default and overlaps the HUD and the loading screen (observed). | V `in_game/mod.rs:234`; screens 09, 12 | 04 08 13 |
| **UI-F13** | 🟠 | **No localisation.** All strings are English literals; counts such as "Search 956 vehicles…" are hard-coded; OMSI content (German map descriptions) is shown raw without a language cue. The editor spec already promises German and English string tables. | V grep; screen 02 | 10 |
| **UI-F14** | 🟠 | **Accessibility is essentially absent**: no gamepad, no screen-reader hooks, no colour-blind or reduced-motion handling (the token is unused), no remapping. Docs mention gamepad axes and a head-bob accessibility setting that do not exist. | V | 10 |
| **UI-F15** | 🟡 | **Main menu deviates from its spec and looks unfinished**: a 280 px sidebar instead of the centred card; the background photo does not cover the window (dark band at the right in screenshot 01) and is blurry when upscaled; the light-theme title "OPTS" is low-contrast; destructive *Shutdown* is the first and visually strongest-red item. | V screen 01 vs `main-menu.md` | 01 05 06 |
| **UI-F16** | 🟡 | **New Game rough edges**: the summary line appears twice (panel and footer); floating scroll arrows overlap list items (observed, screens 02–03); labels contain "(Scrollable)"; vehicles are listed by raw file names ("HC_C2G_main") with an empty subtitle; no thumbnails; no recents or favourites; choices are not remembered between launches; *Start* is always enabled. | V screens 02–04; `types.rs` defaults | 02 03 07 |
| **UI-F17** | 🟡 | **The loading card named a different map than the one loading** in one run ("Ahlheim 5" while Bad Hügelsdorf loaded). Cause not established (the run followed an F1 modal and several clicks); to verify before it is called a bug. | V screen 12; **I** cause | 04 |
| **UI-F18** | 🟡 | **The F1 report dialog** has a large empty area above "Describe the issue:" and no title; if the reports folder is unavailable, *Report* silently does nothing. | V screen 13; `modal.rs:248` | 04 07 |
| **UI-F19** | 🟡 | **Clicks fire on mouse-down** (`Interaction::Pressed`), with no release and no cancel by dragging away; a missed click cannot be aborted. | V `button.rs:364` | 05 07 |
| **UI-F20** | 🟡 | **Escape in a text field also leaves the screen** (two handlers in one system set). | I `mod.rs:122-137` | 05 |
| **UI-F21** | 🟡 | **No motion and one font weight.** Motion tokens exist but nothing reads them; DejaVu Sans Regular is the only text face, so the specified Medium (500) weights cannot render; line height and tracking are specified but never applied. | V `motion.rs`, `fonts.rs`, `components.rs:400` | 09 12 |
| **UI-F22** | 🟡 | **Toasts share one anchor and one z-level** with the loading screen, so concurrent toasts overlap and ordering is undefined. | V `toast.rs`; I ordering | 04 |
| **UI-F23** | 🟡 | **Maintainability**: 16 screen builders of 150–345 lines; widget `spawn` / `spawn_child` bodies duplicated; 43 raw button spawns and 34 glass-surface calls bypass the builders (only 10 builder calls exist); a new screen touches 6+ files and must re-register hover/focus/Enter itself. | V counts | all |
| **UI-F24** | 🟡 | **Documentation drift**: 26 vs 34 colour roles; shadows specified but absent; main-menu layout and labels; 1280×720 vs 1920×1080 default window; `settings.toml` shutdown write that does not exist; RON theme files that do not exist. | V `gui-design-system.md`, `main-menu.md`, `application-startup.md` | 06 |

## 4. Compliance with the report's matrix

| Rule (report §9) | Today | Findings |
| :--- | :---: | :--- |
| Target acquisition | ❌ small targets, no fixed positions for primary/back | F08 F15 |
| Choice load | 🟡 search exists; no recents/defaults; 5–6 top-level choices (good) | F16 |
| Memory load | 🟡 persistent summary line; choices not remembered | F16 |
| Feedback | ❌ only on the main menu; errors hidden | F01 F03 F22 |
| Error prevention | ❌ destructive Esc; invisible disabled state; misleading features | F02 F06 |
| Reversibility | ❌ no undo, no settings revert, no pause | F02 F12 F19 |
| Consistency | ❌ two visual systems; per-screen handlers | F10 F23 |
| Grouping | 🟡 cards and sections exist; duplicate summary, overlapping arrows | F16 |
| Accessibility | ❌ contrast, scale, keyboard, gamepad, reduced motion | F07 F09 F11 F14 |
| Ethical defaults | 🟡 no tracking found; placeholder "live" data; nothing is sent at startup (the weather query is dead code) | F06 |

## 5. Consequences for the design

1. The foundation work (**feedback, focus, scale, one theme**) is the largest single lever and must come first: it is a library problem (`opts-gui`), not a per-screen one.
2. The flow needs a **pause overlay, honest settings, honest loading** before it needs new visuals.
3. Visual redesign should *remove* things (developer copy, duplicated summaries, non-working features) before adding any.
4. Screen builders must become small and composed from patterns, or every later change stays expensive (F23).
