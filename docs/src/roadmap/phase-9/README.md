# 🧭 Phase 9: User Interface & Experience

*Goal: Design and build the user interface of OPTS around evidence-based rules: how it should look, feel and flow, and how it grows with the editor, mods, passengers, audio and more languages without rewrites.*

**Status:** 📝 Proposed design — written for project-lead review on 2026-10-12; nothing implemented; the decisions below are open

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
| 6 | [expandability](expandability.md) | how does it **grow**? |

## Milestones

- [M-0061 — UI Foundations](M-0061.md) — ⏳ Planned (states, focus, one theme, UI scale, action layer, UI ids)
- [M-0062 — App Shell and Flow](M-0062.md) — ⏳ Planned (shell, Home, Plan a drive, loading, pause)
- [M-0063 — Settings System](M-0063.md) — ⏳ Planned (declarative registry, real settings, controls)
- [M-0064 — HUD and In-Game UI](M-0064.md) — ⏳ Planned (instrument HUD, registry, help, developer panel)
- [M-0065 — Accessibility, Localisation and Validation](M-0065.md) — ⏳ Planned (WCAG audit, gamepad, de/en, usability test)

Order: M-0061 first; M-0062 and M-0063 follow in parallel; M-0064 needs the pause overlay of M-0062; M-0065 closes the phase. Every milestone leaves a working game.

## Decisions for the project lead

| ID | Decision | Recommendation | Where |
| :--- | :--- | :--- | :--- |
| D-UI-01 | Keep M3 as token vocabulary, not as the look | yes | [design language §2](design-language.md#2-keep-the-token-architecture-change-its-expression) |
| D-UI-02 | Default theme | Dark (+ Light, High contrast, System) | [§3](design-language.md#3-theme) |
| D-UI-03 | Typeface family | decide after a rendered comparison in M-0061 | [§5](design-language.md#5-typography) |
| D-UI-04 | Minimum window size | 1024×640 | [§6](design-language.md#6-space-density-scale) |
| D-UI-05 | Drop per-widget colour swatches | yes | [§11](design-language.md#11-decisions-for-you) |
| D-UI-06 | F1 = Help (with *Report an issue*), not the report dialog | yes | [interaction §9](interaction-and-feel.md#9-proposed-global-shortcuts) |
| D-UI-07 | Left navigation rail: Home / Drive / Library (+ Workshop later), Settings, Help | yes | [flow §1](flow.md#1-information-architecture) |
| D-UI-08 | Esc = back/pause, never quit/leave without confirmation | yes | [flow §6](flow.md#6-decisions-for-you) |
| D-UI-09 | Hide unavailable features instead of showing stubs | yes | [flow §6](flow.md#6-decisions-for-you) |
| D-UI-10 | Data-driven registries | yes | [expandability §10](expandability.md#10-risks-and-decisions) |
| D-UI-11 | One toolkit (Bevy UI via `opts-gui`), no egui | yes | [expandability §10](expandability.md#10-risks-and-decisions) |

## Relationship to other phases

- **Phase 8** (visual quality): the *Look* and *Texture filtering* settings migrate into the Settings system; the UI is not affected by the 3D look.
- **Phase 6 / M-0051** (editor, mods): consume the components and patterns built here; the *Workshop* destination stays hidden until they exist.
- **M-0035** (settings UI and input rebinding): absorbed by M-0063.
- **M-0046** (world-loader extraction) and the loading pipeline: M-0062 depends on loading reporting real stages and problems.

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: Design and build the user interface of OPTS around evidence-based rules: how it should look, feel and flow, and how it grows without rewrites. Status: design proposed 2026-10-12, awaiting project-lead review; decisions D-UI-01 to D-UI-11 are open. Documents: [phase page](README.md), [ADR-0009](../../adr/0009-ui-architecture.md).*

- [ ] **🧱 UI Foundations** — [M-0061](M-0061.md)
  - [ ] Hover/focus/pressed/disabled/error states and keyboard operability in every widget; one theme for all screens; UI scale 75–200 %; accent presets with a contrast guard.
  - [ ] Action layer, UI ids with `ui.tree`/`ui.click` in the control protocol, CI rules (target size, contrast, keyboard traversal), localisation scaffold.
- [ ] **🧭 App Shell & Flow** — [M-0062](M-0062.md)
  - [ ] Navigation shell, Home with *Continue*, four-step *Plan a drive*, honest loading screen, pause overlay, end-of-session summary; stubs and developer copy removed.
- [ ] **⚙️ Settings System** — [M-0063](M-0063.md)
  - [ ] Declarative setting registry; Graphics, Interface, Content, Controls, Advanced pages with real controls and search; absorbs the settings part of M-0035.
- [ ] **🕹️ HUD & In-Game UI** — [M-0064](M-0064.md)
  - [ ] Instrument HUD from registered widgets and vehicle-type profiles; help overlay; developer panel off by default.
- [ ] **♿ Accessibility, Localisation & Validation** — [M-0065](M-0065.md)
  - [ ] WCAG 2.2 AA audit, gamepad/wheel navigation, German and English complete, moderated usability test.
