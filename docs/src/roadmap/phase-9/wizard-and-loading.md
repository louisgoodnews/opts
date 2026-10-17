# Plan a Drive and Loading: Step Specifications

| Metadata | Value |
| :--- | :--- |
| **Status** | Specification for M-0062 phase C (written before the code, 2026-10-12) |
| **Builds on** | [flow §3.3, §3.4, §4.2](flow.md), [interaction §3, §4, §5, §7](interaction-and-feel.md), [design language §7](design-language.md), [expandability §2](expandability.md) |
| **Code** | `crates/opts-gui/src/patterns/` (master-detail), `crates/opts-game/src/new_game_screen/` (the wizard; the module keeps its name so other branches' imports stay valid), `crates/opts-game/src/in_game/loading_*.rs` |
| **Strings** | message keys in `crates/opts-gui/src/l10n/wizard_en.ftl` / `wizard_de.ftl` (tables of the wizard, the pattern and the loading screen); key names below |

This page is the contract the code and the tests are written against. Copy is English; every key also exists in German.

## 1. The wizard as a whole

State: `AppState::VehicleSelection` stays the wizard's state (renaming is owned by the shell). The wizard keeps its own `PlannerState { step: PlannerStep }` resource with `PlannerStep::{Where, What, When, Review}`; leaving the state resets it to *Where*. The choices live in the existing `NewGameSetup` resource.

**Layout** (screen root made by `spawn_screen_root`, so it sits inside the shell's content area when the shell exists):

```
┌ header: "Plan a drive"   ① Where ── ② What ── ③ When ── ④ Review ───────┐
├ step body (flex: 1, min-height 0) ─────────────────────────────────────┤
├ summary line (always visible, one place)  Berlin-Spandau · MAN SD77 · 08:30 Clear ┤
└ footer (fixed): [ Skip to review ]                       [ Back ] [ Next ▶ ]   ┘
```

- **No fixed pixel heights** for panels: the body is a flex child (`flex_grow: 1`, `min_height: 0`), lists fill it; the only pixel sizes are row heights, control sizes and the fixed-size pickers.
- **Stepper**: four step buttons (also reachable with `[` / `]` = `PrevTab`/`NextTab`). The current step is marked by fill and an underline, reached steps show a check; a step can be jumped to at any time because all defaults are valid.
- **Persistent summary**: exactly one summary line above the footer on every step (it replaces the two duplicate lines of today, audit F16). It names map (and start point when ambiguous), vehicle (and livery), clock and weather; clicking a part jumps to its step.
- **Footer**: *Back*, *Next*, and *Skip to review* (steps ① to ③ only, enabled while the defaults are valid). On ④ the primary action is *Drive*. *Back* on ① leaves the wizard (to the screen the user came from). `Esc` = `UiAction::Back` = *Back* (one step back, then out); inside a text field the first `Esc` only leaves the field (library behaviour), inside a modal it closes it.
- **Validity**: *Next* and *Drive* are enabled only when the selection is valid: a map that exists in the catalogue, a start point of that map (or none when the map has none), a vehicle that exists in the catalogue. When invalid the control is disabled **with a reason** (`DisabledReason`, e.g. "Choose a map first") — never silently.
- **Remembered choices**: on entering the wizard the last `DriveRecord` (`DriveHistory::last`) is applied to `NewGameSetup` when its map and vehicle still exist; otherwise the defaults are chosen by `reconcile_selection`. When *Drive* is pressed the wizard calls `history.record(DriveRecord::from_setup(&setup, now))` and `save()` (a failed save is logged and shown as a toast, the drive still starts). Recents and favourites in the lists come from the same `DriveHistory` (`recents`, `favourite_maps`, `favourite_vehicles`); star toggles save at once.
- **Default focus**: the list on ①/② (typing starts type-ahead; `/` or `Ctrl+F` jumps to the search field); the first quick chip on ③; *Drive* on ④ (safe: it only starts a drive).
- **Landmarks** (`FocusRegion`): stepper+list = *Content*, detail panel = *Detail*, footer = *Footer*; `Tab` jumps between them.
- **No developer wording** anywhere ("Scrollable", "Fleet Models", "setvars", file names).

## 2. Master–detail pattern (`opts_gui::patterns::master_detail`)

Built once, used by ① and ② and later by the Library (flow §3.9).

| Aspect | Rule |
| :--- | :--- |
| Layout | Expanded/Medium: list (35 %, min 280 px) left, detail right; **Compact** (`LayoutClass`): stacked, list above detail. |
| Model | `MasterDetailModel { entries, recents, favourites, chips, query, active_chips, selected }`; an entry has a stable `id`, `title`, optional `subtitle`, `search_text`, `tags`, optional thumbnail key and glyph. |
| Rows | fixed height (56 px items, 32 px section headers); **at most 200 row entities whatever the catalogue size** (only the visible window plus overscan exists); rows are `PointerOnly` buttons, the list itself is the single focus stop (roving selection, like a listbox). |
| Sections | with an empty query and no chip: *Recent* (≤ 6, from the history), *Favourites*, *All*; as soon as the user searches or filters: one flat result list. |
| Selection | one state: container fill **plus a marker bar** (not colour alone); the detail panel follows the selection without a second click (`MasterDetailSelected`). |
| Keyboard | list focused (`Ctrl+D` stars the selected entry, so favourites are reachable without a pointer): `↑/↓` move the selection, `PgUp/PgDn` page, `Home/End` jump, **type-ahead** (letters typed within 800 ms select the next title that starts with them), `Enter` = *continue* (`MasterDetailChosen`, the wizard answers with *Next*), `Ctrl+F` or `/` focus the search field; `Esc` is not consumed. |
| Pointer | wheel scrolls, click selects, drag of the scroll bar thumb (≥ 24 px); star toggles favourite (a sibling control of the row, so one click never does two things). |
| Search | a `TextField`; filters on every change (no debounce needed: filtering thousands of entries is instant), keeps the selection when still visible, shows the live count in the placeholder ("Search 43 maps…"). |
| Filter chips | toggles (`Activated`), combined with *and*; a chip with no match is still shown. |
| Empty / no result | "No results for “xyz”" with a *Clear filter* button; empty catalogue: the caller's message (key) plus its action. |
| Accessibility | every control has a `UiId` (`<prefix>.search`, `<prefix>.list`, `<prefix>.chip.<tag>`, `<prefix>.star.<id>`), rows report as list items. |
| Thumbnails | a row has a 48×48 thumbnail slot filled lazily by the thumbnail service (§6); until then a neutral glyph. |

Keys: `md-search` (variable `$count`), `md-section-recent`, `md-section-favourites`, `md-section-all`, `md-no-results` (`$query`), `md-clear-filter`, `md-star-add`, `md-star-remove`, `md-count` (plural).

## 3. Step ① Where

| Part | Content |
| :--- | :--- |
| List | the maps of the catalogue by friendly name (`MapItem::friendly_name`), subtitle: the years of the map when it has them; chips: none today (regions and eras do not exist in the catalogue — no fake chips); recents/favourites sections. |
| Detail | hero image (the map's `picture.jpg` through the thumbnail service, cropped to fill, 16:9, scrim under overlaid text; no image → neutral panel with the map glyph and nothing invented), title, **facts that exist** (`wiz-fact-years`, `wiz-fact-tiles` ("369 tiles"), `wiz-fact-starts` ("16 start points"), `wiz-fact-depot`), **start point** selector (a combobox of `MapItem::entry_points`; hidden with the line `wiz-where-no-starts` when the map has none), description in its original language under the label `wiz-where-description` ("Description from the map, in its original language"); long descriptions scroll inside their box; the **date** (`wiz-where-date`: day, month, year steppers of `shell::calendar`, the one place where the date is chosen) with the era hint `wiz-when-era-hint` ("This map is set in 1986–1994") when the chosen year lies outside the map's years. |
| Defaults | the last map; the first start point of the map (or the last one used on this map). Date (C6, 2026-10-17): unless the user changed it or it was remembered from the last drive (`PlannerState::date_chosen`), choosing a map sets `planner::default_map_date`: today + the map's `[realyearoffset]` years (29 Feb → 28 Feb), clamped into `[years]` a..b as `a-01-01 ..= b-12-31` (Berlin-Spandau, offset −25, years 1986–1994: 1994-12-31 for any date after 2019); a map without `[years]` keeps the shifted date. `[years]`/`[realyearoffset]` are read from the map's `global.cfg` once per map. There is no hard-coded default date any more (`NewGameSetup` starts on today). |
| Empty | no maps: banner text `wiz-where-empty` ("No maps found — check the OMSI folder or rescan") with the action *Open settings* (the Settings screen exists; the action goes to `AppState::Settings`). |
| Errors | a remembered map that vanished: silently replaced by the first map and a one-line note `wiz-where-remembered-missing` in the detail ("Your last map is no longer installed."). |

## 4. Step ② What

| Part | Content |
| :--- | :--- |
| List | vehicles by **name** (`manufacturer + model`) and **class** chip (`wiz-class-road/rail/aircraft` from `entities.vehicle_class`; a vehicle without class shows none), **never raw file names**; thumbnails (§6); chips: the classes present; favourites and recents. |
| Detail | name, class, **facts that exist** (`wiz-fact-mass` = empty weight in t when the catalogue has it; the description text when not empty). Facts without data (length, doors, power, today "N/A") are **not shown**. |
| Livery | list of the vehicle's repaints (`repaints::vehicle_repaints`) as selectable rows with a **colour chip** (a swatch derived from the repaint's texture file when readable — average colour of the first texture, computed lazily by the thumbnail service — else a neutral outline chip with the first letter; never a made-up colour); the default livery is preselected and marked "Default". Repaints are discovered on a background task (never blocking a frame); while pending the list shows `wiz-what-livery-loading`. |
| Configure… | *Configure…* opens the equipment modal (`Modal`) listing `get_vehicle_setvars`, each with a switch and a plain-language label; *Reset* and *Done*. Hidden when the vehicle has no options, with the line `wiz-what-no-options` ("This vehicle has no equipment options."). Labels of unknown options are the option's own name, not "Vehicle Equipment Option". |
| Defaults | the last vehicle; its default livery. |
| Errors | a remembered vehicle that vanished: replaced by the first vehicle and the note `wiz-what-remembered-missing`. |

## 5. Step ③ When

| Part | Content |
| :--- | :--- |
| Quick chips | `wiz-when-now` (the current wall-clock time of day, rounded to 5 min), `wiz-when-morning` 08:00, `wiz-when-noon` 12:00, `wiz-when-evening` 18:00, `wiz-when-night` 22:00; exactly one chip is *selected* when the time equals its time. Choosing a chip changes only the time of day. |
| Exact time | **fixed-size steppers**, not free-growing pickers (`shell::calendar`, shared with the map step's date and the pause menu's *Environment* page): hour, minute (steps of 5, `Shift` = 1) with − / + (target ≥ 32 px); the block is ≤ 560 px wide, so it fits the width of a 1024×640 window at 100 % and at 150 % scale (683 logical px; checked by test, as is the map step's date block). The **date moved to the map step** (C6): day, month (name), year, invalid dates (31 Feb) clamp to the month length, years 1900–2100, with the era hint there. |
| Weather | preset cards (a short list, no scrolling): glyph, name, a **one-line effect that is true today** — the simulator only changes sky and light for weather, so the lines say that (`wiz-weather-clear-effect` "Clear sky, full sunlight", `…-overcast-effect` "Cloud cover, softer light", `…-rain-effect` "Dim, diffuse light (rain itself is not simulated yet)", `…-snow-effect`, `…-night-effect`, `…-storm-effect`, `…-fog-effect`). No invented temperatures, friction values, wind or visibility numbers. Preset ids stay the ones stored in `NewGameSetup::weather_preset` and in the history. |
| **Live weather** | stays visible as a card at the end of the weather list, **registered `NotYet` place** `wizard.live-weather` (milestone M-0062, "The weather query is never called; live conditions need an explicit, explained opt-in") and carries the red "Feature not available yet!". No numbers, no network call, no placeholder values (audit F06). The existing background query function and `DeviceLocationWeather` resource are removed from the wizard (the dead code goes). |
| Defaults | the last choice; first start: 08:30 clear summer day (the date: see step ①). |

## 6. Step ④ Review and thumbnails

**Review**: one card with three rows — Where (map photo thumbnail, map, start point), What (vehicle thumbnail, name, livery, option count), When (date, time, weather) — each with a *Change* link that jumps to its step (focusable, ≥ 32 px). Below: the primary **Drive** button (default focus) and, when `DriveHistory` has a favourite map/vehicle matching, nothing else — *Save as favourite drive* of flow §3.3 is **deferred** (stars on maps and vehicles cover favourites today; recorded in the progress file). *Drive* writes the history and moves to `AppState::InGame` (the loading screen).

**Thumbnail service** (`drive_planner/thumbnails.rs`): `ThumbnailService` resource. `request(key, source_path)` returns immediately; a background task (Bevy `IoTaskPool`) decodes the source image, scales it to at most 320×180 (maps) / 96×96 (vehicles) with `image`, stores it as a small PNG in the cache directory `<user cache dir>/opts/thumbnails/<hash>.png` (never in `~/.cache/opts` during tests: the directory is a parameter, defaulting to the XDG cache), and sends the pixels back through a channel; a main-thread system turns finished pixels into `Image` assets and fires `ThumbnailReady`. The cache has a **size cap** (default 64 MB, least recently used removed first when the cap is exceeded, checked once at startup and after writes), keys include the source file's modification time so a changed picture is rebuilt. Maps: `picture.jpg` of the map folder. Vehicles: **an existing preview image when the content has one** (none of the shipped OMSI vehicles has one today — verified by listing the installation), otherwise a neutral placeholder: a vehicle glyph and the class label. No fake data.

## 7. Loading screen

Plain language, themed (all colours and sizes through role/type tokens, no `Color::srgb`, no sub-12 px text), shown as an overlay (`GlobalZIndex` above the HUD but below modals).

| Part | Content |
| :--- | :--- |
| Header | `load-title` "Preparing your drive"; no developer wording ("OPTS TRANSIT OPERATIONS", "Synthesizing…" go). |
| Manifest | three tiles (map and start point; vehicle and livery; date, time and weather) built from **`InGameSessionInfo`**, i.e. from the session that is **actually loading**, and refreshed whenever the session changes. Cause of audit F17: the tiles were built once from `NewGameSetup` when the screen was spawned, while the map that loads is the one resolved by `resolve_session_config` (a `--map-path` argument or a catalogue match wins over the menu choice); the card therefore could name a different map than the one loading. Fix: tiles read the session record and the loader writes the resolved map and vehicle into it; tested with a setup naming one map and a request resolving another. |
| Stages | real counts from the loader, plain sentences: `load-stage-catalogue` ("Reading your OMSI content (first start only)"), `load-stage-map` ("Loading the map: { $done } of { $total } tiles"), `load-stage-vehicle` ("Preparing the vehicle"; with `load-stage-vehicle-meshes` "Preparing the vehicle: { $left } protected models left"), `load-stage-light` ("Setting up light and weather"), `load-stage-start` ("Getting ready to drive"). One progress bar (`ProgressBar` role `Primary`), the percentage is the loader's real fraction. |
| Controls card | the driving keys that exist (steer, accelerate, brake, doors, camera, pause, help) as a two-column list of key chip and action; text from keys `load-controls-*`. |
| Cancel | always available (button and `Esc`): asks first (`Modal`, default focus *Keep loading*) and returns to the wizard (`AppState::VehicleSelection`), not to the main menu. |
| **Outcomes** | The loader records a `LoadReport { problems: Vec<LoadProblem> }` (severity, what, why, details) as it works. **Ready**: no problems, the screen continues automatically into the drive (no *Start driving* button). **Ready with N problems** (blocking problems only; minor ones auto-continue, see section 10): the world is usable (a vehicle or some tiles are missing but a camera exists) — the screen **stays**, shows the problem list (what happened, why, what you can do) under the result line, buttons *Details* (expands paths and messages with *Copy*) and *Continue*. **Failed**: nothing usable (map failed to resolve or load, no tile) — the screen stays with the result line, the cause, and *Retry*, *Choose another* (back to the wizard) and *Back* (to the previous screen). |
| Errors are kept | problems are appended to `LoadReport` and are never replaced by a later status text (audit F03: the status line "ERROR: …" was overwritten by "Ready"); the status text and the report are separate. Seeded failures (missing vehicle, missing map tile, failed asset) are covered by tests. |
| Developer overlay | the F10 panel (`WorldStats` overlay) is not drawn while `LoadingScreenRoot` exists. |
| Plurals | `load-ready-problems` ("Ready with 1 problem" / "Ready with { $count } problems"). |

## 8. Keys (summary)

`wiz-*` (wizard), `md-*` (pattern), `load-*` (loading) in `wizard_en.ftl` and `wizard_de.ftl`; the completeness tests of the localisation module cover both tables.

## 9. Tests

- Pattern: windowing (≤ 200 rows for 100 000 entries), keyboard (arrows, paging, Home/End, type-ahead), selection marker, search/clear, chips, sections, star toggle, Compact stacking — in `layout_app`.
- Wizard: every step builds, *Next*/*Back*/*Skip to review*, `Esc` order, validity reasons, remembered choices applied, history written on *Drive*, facts without data not shown, live weather is a not-yet control without numbers, pickers fit 1024×640 at 100 %/150 %, `ui.tree` snapshots.
- Loading: seeded failures keep their text, auto-continue on success, outcomes and their buttons, manifest follows the real session, no overlay while loading.
- CI rules (`ui_rules.rs`): target ≥ 32 px, text ≥ 12 px, keyboard traversal, disabled with reason, no hard-coded colours or user-visible literals in the wizard and loading modules.

## 10. Implementation notes and deviations (2026-10-12)

- *Copy* and *Report* in the problem details are not built: they need the clipboard and the report dialog owned by the Help milestone (M-0064). The details view shows paths and messages.
- *Back* on a failed load goes Home (`AppState::MainMenu`); *Choose another* and `Esc` go to the wizard. Retry restarts the pipeline on the same screen.
- The menu camera now stays active until a 3D scene camera exists; before, the loading screen (and a failed one, which never gets a scene camera) was invisible. The control-protocol screenshot is black during the stages for the same reason it was black before; with the fix the screen is drawn by the menu camera.
- Severity (decided by the orchestrator): only blocking problems (map, tile, player vehicle) stop the screen; minor asset problems auto-continue and are announced by one toast with *Details* (a dialog with the list; *Copy* / *Report* wait for Help, M-0064). The F10 panel shows the problem count.
- Long list titles are cut with an ellipsis at the estimated column width; the full title is the row's tooltip.
- Compact stacking: the list gets 260 px and the page scrolls; verified in the layout tests, not in a real small window (the compositor kept the window at 1920x1013).
