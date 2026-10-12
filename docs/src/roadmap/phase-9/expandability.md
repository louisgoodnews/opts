# Expandability — How the UI Grows Without Rewrites

| Metadata | Value |
| :--- | :--- |
| **Status** | ✅ Approved 2026-10-12 (decisions recorded in the [phase page](README.md#decisions)) |
| **Date** | 2026-10-12 |
| **Builds on** | [audit](current-ui-audit.md) (F13, F14, F23), [flow](flow.md), [editor workspace UI](../../editor/workspace-ui.md), [control protocol](../../architecture/control-protocol.md) |

## 1. Goals and growth map

The roadmap will add a lot of interface. The design question is not "what does each screen look like" but **"what does it cost to add the next one"**. Today a new screen touches 6+ files and must re-register hover, focus and Enter handling itself; a new setting needs edits in 5 places (F23).

| Coming | What the UI will need |
| :--- | :--- |
| Passengers, stops, timetables (Phase 4) | stop and trip panels in the HUD, a timetable view in Library, ticket interaction |
| Audio (M-0027) | a mixer page, per-vehicle sound options |
| Traffic / pedestrian options (M-0049, M-0050) | density settings with live effect |
| Vehicle types (trams, trolleybuses) | per-type HUD profiles and control pages |
| Editor (M-0051) | docking workspace, inspector, outliner, menu bar, dense mode, command palette |
| Mods and content (Phase 6) | Workshop screens, theme/HUD/language packs |
| Graphics features (DLSS, M-0048; Phase 8 looks) | settings that only exist when the hardware supports them |
| More languages | string tables, pluralisation, longer text |
| Gamepad / wheel | an action layer and remapping |

**Rule: features extend the UI by registering, not by editing.** A feature crate declares *what* it contributes (a settings entry, a HUD widget, a command, a screen); the shell decides *where and how* it appears.

## 2. Layers and crates

```
 Shell        navigation rail, app frame, overlays, toasts, back stack      opts-game::shell
 Screens      Home, Plan a drive, Library, Settings, Help …                 opts-game::screens
 Patterns     master-detail, wizard, settings page, list+search, dialog,
              banner, inspector, dock                                       opts-gui::patterns
 Components   button, field, list, tabs, tree, slider, tooltip, menu …      opts-gui::components
 Core         focus, actions, states, layout classes, text, icons, motion,
              localisation, UI ids                                          opts-gui::core
 Tokens       colours, type, space, shape, elevation, motion, themes        opts-gui::theme
```

- **Dependencies point downward only.** `opts-gui` knows nothing about OMSI, the catalogue or the game; screens know nothing about `bevy_ui` node styling beyond composition.
- **A screen is a composition of patterns.** If a screen builder grows past the 80-line function limit ([standards](../../contributing/standards.md)), a missing pattern is the cause, and the pattern is extracted (today 16 builders are 150–345 lines).
- **One way to build a widget**: a single builder returning a bundle (the duplicated `spawn` / `spawn_child` go), and **all Bevy-UI specifics stay inside `opts-gui`**, so engine upgrades touch one crate (the Bevy 0.19 and 0.20 ports each needed UI-wide edits). Bevy's own widget work (`ui_widgets`, scene authoring) is re-evaluated when stable, behind the same builders.
- **Every pattern has an entry in a component gallery example** (extending the existing `gallery` example), which doubles as a visual regression target.

## 3. HUD registry

A HUD is *data* binding sim values to widgets in slots.

| Concept | Definition |
| :--- | :--- |
| `HudWidget` | id, title key, **binding** (a simulation value such as speed, or a script variable by name), formatter (unit, precision, tabular), **slot**, size classes, priority, visibility condition (vehicle type, camera mode, setting) |
| **Slots** | `Instruments` (bottom centre), `StatusBar` (top, fades), `Alerts` (above the cluster, at most one visible, critical alerts sticky), `Hints` (bottom left, fade), `Camera label` |
| `HudProfile` | the set of widgets and their slot assignments for a vehicle *type* (bus, articulated bus, tram, trolleybus), shipped as RON files; the user can switch widgets on/off; free-form layouts come later |
| **Contract** | widgets never take input focus, never shift other widgets, never exceed a per-frame budget; alerts follow UI-P13 |

New content (a tram cab, passenger information, a timetable strip) is a new `HudWidget` descriptor and a profile entry, not changes to the HUD systems. Third parties can ship **profiles as data**; they cannot ship code.

## 4. Screen and navigation registry

```
ScreenDescriptor {
  id            "settings"            // stable, used for deep links and tests
  title_key     "nav-settings"
  icon          Icon::Settings
  group         Main | Utility
  order         u16
  availability  fn(&World) -> NotYet{milestone} | Disabled(reason) | Enabled
  content       the plugin/system that builds the page
}
```

- The **rail is generated from the registry**; `NotYet{milestone}` renders the entry disabled with the red text **"Feature not available yet!"** (D-UI-09), a one-line rule instead of a hard-coded `.with_disabled(true)`. Every `NotYet` is **collected into a generated list** (`docs/src/reference/not-available-yet.md`: where, which milestone) and printed at debug startup, so promised features cannot be forgotten; a test fails if a `NotYet` refers to a milestone that is marked complete.
- The shell owns the **back stack** and Esc semantics, so no screen re-implements "Escape returns to the menu".
- **Deep links** (`screen:settings/graphics`) serve the control protocol (`ui.navigate`), the Help overlay ("Open this setting") and tests.
- Adding a screen = **one module and one registration**. A checklist test confirms a registered screen has a title key, an icon, keyboard reachability and a golden screenshot.

## 5. Settings registry (declarative)

Today a setting is hand-built in five places (struct field, load, save, UI, handler). Instead a feature **declares** it:

```
SettingDescriptor {
  key          "graphics.texture_filtering"
  kind         Bool | Enum{options} | Int{min,max,step} | Float{…} | Path | Colour | Binding
  default      …
  category     Graphics            order  …
  label_key    description_key     keywords
  apply        Live | OnRestart | WithRevert(15 s)
  available    Always | Capability(Dlss) | Feature(Audio)
  since        "0.1"               migrate_from  ["old.key"]
}
```

One declaration produces: the **settings row** in the right category (control chosen by `kind`), **persistence** in `settings.db` (typed, with defaults when absent, as the Phase 8 settings already do), **validation**, *Reset*, the **search index**, the control-protocol commands `settings.list/get/set`, and a **generated reference page** in the docs (`docs/src/reference/settings.md`, built by a script and checked in CI), so documentation cannot drift (F24). Feature crates register their own entries (`opts-bevy` registers texture filtering and look); the Settings screen is generic and knows none of them.

## 6. Other registries and services

| Service | Design |
| :--- | :--- |
| **Commands** | id, label key, default bindings, context, handler. One list feeds the shortcut help, the **command palette**, the rebinding page and the control protocol (`ui.command`). |
| **Input actions** | the [action layer](interaction-and-feel.md#1-devices-and-the-action-layer): devices → actions → handlers; bindings stored in settings with conflict detection; gamepad and wheel through Bevy's gamepad support; per-vehicle control pages come from the vehicle type. |
| **Themes** | RON files in the user's config directory, versioned; loaded through the contrast checker, which **rejects or auto-corrects** inadequate pairs (today's preset tables fail it); theme packs are data. |
| **Localisation** | **Fluent** bundles (`.ftl`) with keys, fallback chain *user → English*, plurals, number/date/unit formatting; German and English complete first; a **pseudo-locale** that lengthens text by ~40 % for tests; community translations as files in the user directory. OMSI content (map descriptions) is *content*, not UI: shown as is, labelled with its language. A source test forbids string literals in UI code outside tests and the key tables. |
| **Capabilities** | a resource listing what the machine and build can do (GPU features, audio device, gamepad), consulted by `available` in settings and screens. |
| **Notifications** | one service for toasts, banners and dialogs implementing [interaction §5](interaction-and-feel.md#5-errors): stacking, priority, de-duplication, never overlapping. |

## 7. Testing

The UI must be testable **without pixel coordinates** and without a human (today `input.mouse.click` needs x/y; only the vehicle menu exposes its state).

1. **UI ids and tree.** Every interactive element has a stable path id (`drive/where/map-list`). The control protocol gains `ui.tree`, `ui.click <id>`, `ui.focus <id>`, `ui.action <action>`, `ui.navigate <screen>`, `ui.state <id>`.
2. **Automatic rules over the tree** (run in CI on a synthetic catalogue):
   - no interactive target below 32×32 (UI-P01) and no text below 12 px;
   - **contrast** of every text and control against its *resolved* background, including text over images (sampled), for every theme and accent preset;
   - **keyboard traversal**: starting from the first focus, Tab and arrows reach every interactive element, and each can be activated (UI-P08);
   - disabled controls carry a reason; no hard-coded colour or string literal outside the token and string tables (source lints).
3. **Golden screenshots** of Home, Plan a drive (3 steps), Settings (2 categories), Pause, HUD and Loading at 1280×720, 1920×1080 and 2560×1440, dark and light, UI scale 100 % and 150 %, using self-authored fixture content (no OMSI assets), compared with a tolerance.
4. **Text expansion**: the pseudo-locale and a long German string set must not clip or overlap (ellipsis with tooltip is the only allowed truncation).
5. **Performance**: frame-time test with a 1,000-item list and the HUD active; UI systems ≤ 1 ms (see [interaction §10](interaction-and-feel.md#10-performance-and-latency-budget)).
6. **Usability test**: a short moderated protocol (tasks, success/time/errors, glance times for the HUD) defined in [M-0065](M-0065.md); the report's own advice is that rules are hypotheses until validated.

## 8. Editor and mod reuse

The editor ([workspace UI](../../editor/workspace-ui.md)) is the harshest consumer: dense, many panels, many fields. It needs components `opts-gui` lacks today (audit §5.6). Build order, shared with the editor milestone:

1. **virtualised list and data table** (also needed by the Library and the 956-vehicle list),
2. **tooltip, slider, menu and context menu**, keyboard-operable **tree**, working **text field**,
3. **dock, splitter and panel presets** (stored in settings like other preferences),
4. **colour, file and path pickers**, and the *dense* density mode.

**Mods and third parties** extend the UI through **data only**: themes, HUD profiles, language packs, vehicle control layouts, thumbnails. Anything executable stays in the existing script VM, which cannot create interface elements. All formats are versioned and validated on load; a bad pack is reported in the Workshop with the template of [interaction §5](interaction-and-feel.md#5-errors), never crashes the game.

## 9. Versioning, migration, performance

- **Settings** carry a schema version; `migrate_from` maps renamed keys, so old `settings.db` files keep working (as with the Phase 8 keys).
- **Theme, HUD-profile and language files** carry a `version`; unknown versions are refused with a clear message.
- **Entity budget**: lists virtualised (≤ 200 row entities), thumbnails loaded lazily and cached, screens built on first visit and kept or dropped by a documented policy, no per-frame rebuilds.
- **Bevy upgrades**: UI engine API confined to `opts-gui` (§2); a compile-time gallery guards visual regressions.

## 10. Risks and decisions

| Risk | Mitigation |
| :--- | :--- |
| **Scope**: this is a rebuild of the UI layer | foundations first ([M-0061](M-0061.md)); each milestone leaves a working game; old screens are replaced one at a time |
| **Bevy UI limits**: text input and IME, accessibility tree, virtualised lists | spike each in M-0061 before depending on it; keep the option to use a purpose-built widget for one case |
| **Font and icon licences** | OFL / Apache-2.0 only; licences listed in Help → About |
| **Docking is costly in Bevy UI** | scope the editor dock to what M-0051 needs; evaluate a mature crate only if the cost proves high |
| **Two sources of truth for settings during migration** | the registry owns new settings from day one; old keys are adapted, not duplicated |

| ID | Decision | Recommendation |
| :--- | :--- | :--- |
| **D-UI-10** | Data-driven registries for screens, settings, HUD widgets, commands | yes |
| **D-UI-11** | Stay with one toolkit (Bevy UI through `opts-gui`) for game and editor; do not add egui | yes; revisit only if docking/inspector prove disproportionately expensive |
