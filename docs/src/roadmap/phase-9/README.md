# 🧭 Phase 9: User Interface & Experience

*Goal: Design and build the user interface of OPTS around evidence-based rules: how it should look, feel and flow, and how it grows with the editor, mods, passengers, audio and more languages without rewrites.*

**Status:** 🟡 Design **approved 2026-10-12** (decisions below); implementation in progress (M-0061 part A, the theme foundations, done 2026-10-12)

The checklist for this phase lives in the [roadmap](../README.md); this page lists its documents and milestones.

## What this phase is based on

1. **A human-factors report** supplied by the project lead (*Clinical Report: Evidence-Based Design Rules for Modern Graphical User Interfaces*, 2026), analysed in [report-analysis](report-analysis.md): what applies to a desktop driving simulator, where it needs translation, what it does not cover (driving), and thirteen principles **UI-P01 … UI-P13** with measurable acceptance.
2. **An audit of the current UI** ([current-ui-audit](current-ui-audit.md)): code inventory, 17 screenshots, computed contrast, 24 numbered findings.

## The design in one page

| | Today | Proposed |
| :--- | :--- | :--- |
| **Looks** | M3 look-alike with glass over photos; two visual systems; one font weight; contrast failures | calm "operations room" character; one theme (dark by default) with light, high-contrast, system; own expression on the M3 token architecture; 4-weight typeface; instrument token set for the HUD; real shadows, thumbnails, icons ([design language](design-language.md)) |
| **Feel** | hover and focus on one screen; Escape quits or drops the session; clicks fire on press; no scale, no gamepad | the same seven states on every control, visible focus, full keyboard and gamepad navigation, activation on release, Esc = back/pause, confirmation only for the irreversible, errors that are never hidden ([interaction and feel](interaction-and-feel.md)) |
| **Flow** | menu → three tabs → loading → HUD; stubs; no pause | one shell with a rail; Home with *Continue*; four-step *Plan a drive*; honest loading; pause overlay; end-of-session summary; searchable settings ([flow](flow.md)) |
| **Expandability** | a new screen = 6+ files; a new setting = 5 places | layered library and **registries** (screens, settings, HUD widgets, commands, themes, languages), UI ids and a UI tree for tests, data-only extensions for mods ([expandability](expandability.md), [ADR-0009](../../adr/0009-ui-architecture.md)) |

## Reading order

| # | Document | Question it answers |
| :--- | :--- | :--- |
| 1 | [report-analysis](report-analysis.md) | what do the rules say, and which of them bind OPTS? |
| 2 | [current-ui-audit](current-ui-audit.md) | where are we, with evidence? |
| 3 | [design-language](design-language.md) | how should it **look**? |
| 4 | [interaction-and-feel](interaction-and-feel.md) | how should it **feel**? |
| 5 | [flow](flow.md) | how should it **flow**? |
| 5a | [theme-auto-switch](theme-auto-switch.md) | should the theme follow day and night? |
| 6 | [expandability](expandability.md) | how does it **grow**? |

## Milestones

- [M-0061 — UI Foundations](M-0061.md) — 🟡 In progress: **part A done** (theme foundations: dark default, modes, typography, scale, density, accents with contrast guard, one theme everywhere); part B open (states, focus, action layer, UI ids)
- [M-0062 — App Shell and Flow](M-0062.md) — ⏳ Planned (shell, Home, Plan a drive, loading, pause)
- [M-0063 — Settings System](M-0063.md) — 🟡 Started (registry, generic page, real settings and `settings.*` done; rebinding, audio, view distance, time scale shown as not available yet)
- [M-0064 — HUD and In-Game UI](M-0064.md) — ⏳ Planned (instrument HUD, registry, help, developer panel)
- [M-0065 — Accessibility, Localisation and Validation](M-0065.md) — ⏳ Planned (WCAG audit, gamepad, de/en, usability test)

Order: M-0061 first; M-0062 and M-0063 follow in parallel; M-0064 needs the pause overlay of M-0062; M-0065 closes the phase. Every milestone leaves a working game.

## Decisions

Answered by the project lead on 2026-10-12. Where the answer differs from the original recommendation it is stated in the *Decision* column.

| ID | Question | Decision | Where |
| :--- | :--- | :--- | :--- |
| D-UI-01 | Keep M3 as token vocabulary, not as the look | **yes** | [design language §2](design-language.md#2-keep-the-token-architecture-change-its-expression) |
| D-UI-02 | Default theme | **Dark** stays the default (+ Light, High contrast, System). **Plus** an investigated and implemented mode *Auto (clock)* that follows day and night of the device's timezone ([investigation](theme-auto-switch.md)) | [§3](design-language.md#3-theme) |
| D-UI-03 | Typeface family | **delegated**: decided in M-0061 after a rendered comparison: **Noto Sans** (OFL; Regular/Medium/Bold) with DejaVu Sans fallback and JetBrains Mono, see [design system §3.2](../../architecture/gui-design-system.md#32-typography) | [§5](design-language.md#5-typography) |
| D-UI-04 | Minimum window size | **1024×640** (recommendation accepted) | [§6](design-language.md#6-space-density-scale) |
| D-UI-05 | Drop per-widget colour swatches | **yes** | [§11](design-language.md#11-decisions-for-you) |
| D-UI-06 | F1 = Help (with *Report an issue*), not the report dialog | **yes** | [interaction §9](interaction-and-feel.md#9-proposed-global-shortcuts) |
| D-UI-07 | Left navigation rail: Home / Drive / Library (+ Workshop later), Settings, Help | **yes** | [flow §1](flow.md#1-information-architecture) |
| D-UI-08 | Esc = back/pause, never quit/leave without confirmation | **yes** | [flow §6](flow.md#6-decisions-for-you) |
| D-UI-09 | Unavailable features | **changed**: *not hidden*. Every unimplemented feature stays visible and carries the red warning text **"Feature not available yet!"**, registered centrally and listed in a generated report so none is forgotten | [flow §6](flow.md#6-decisions-for-you) |
| D-UI-10 | Data-driven registries | **yes** | [expandability §10](expandability.md#10-risks-and-decisions) |
| D-UI-11 | One toolkit (Bevy UI via `opts-gui`), no egui | **yes** | [expandability §10](expandability.md#10-risks-and-decisions) |

## Relationship to other phases

- **Phase 8** (visual quality): the *Look* and *Texture filtering* settings migrate into the Settings system; the UI is not affected by the 3D look.
- **Phase 6 / M-0051** (editor, mods): consume the components and patterns built here; the *Workshop* destination is visible from the start and marked **"Feature not available yet!"** until they exist.
- **M-0035** (settings UI and input rebinding): the settings UI is absorbed by M-0063 (done); input rebinding stays with [M-0035](../phase-7/M-0035.md).
- **M-0046** (world-loader extraction) and the loading pipeline: M-0062 depends on loading reporting real stages and problems.

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Design and build the user interface of OPTS around evidence-based rules: how it should look, feel and flow, and how it grows without rewrites. Status: design approved 2026-10-12 (decisions D-UI-01 to D-UI-11 answered; unavailable features stay visible with the red text "Feature not available yet!"); implementation in progress. Documents: [phase page](README.md), [ADR-0009](../../adr/0009-ui-architecture.md).*

- [ ] **🧱 UI Foundations** — [M-0061](M-0061.md)
  - [x] Theme foundations (part A, 2026-10-12): dark default with Light, System, Auto (clock) and High-contrast themes; Noto Sans type scale; UI scale 75–200 %, density, layout classes, 1024×640 minimum window; status roles, shadows, motion with reduced motion; accent presets with a contrast guard and RON theme files; one theme for loading screen, HUD and stubs; instrument tokens.
  - [ ] Hover/focus/pressed/disabled/error states and keyboard operability in every widget (part B).
  - [ ] Action layer, UI ids with `ui.tree`/`ui.click` in the control protocol, CI rules (target size, contrast, keyboard traversal), localisation scaffold.
- [ ] **🧭 App Shell & Flow** — [M-0062](M-0062.md)
  - [ ] Navigation shell, Home with *Continue*, four-step *Plan a drive*, honest loading screen, pause overlay, end-of-session summary; stubs and developer copy removed.
- [ ] **⚙️ Settings System** — [M-0063](M-0063.md)
  - [x] Declarative setting registry, generic settings page with search, *Reset* and revert prompt; Graphics, Interface, Content, Gameplay and Advanced pages with real controls; `settings.*` in the control protocol; generated settings reference (2026-10-12).
  - [ ] Controls rebinding (rows shown as "Feature not available yet!"; moves to M-0035), Audio with M-0027.
- [ ] **🕹️ HUD & In-Game UI** — [M-0064](M-0064.md)
  - [ ] Instrument HUD from registered widgets and vehicle-type profiles; help overlay; developer panel off by default.
- [ ] **♿ Accessibility, Localisation & Validation** — [M-0065](M-0065.md)
  - [ ] WCAG 2.2 AA audit, gamepad/wheel navigation, German and English complete, moderated usability test.
