# Flow — Structure, Screens and Journeys

| Metadata | Value |
| :--- | :--- |
| **Status** | ✅ Approved 2026-10-12 (decisions recorded in the [phase page](README.md#decisions)) |
| **Date** | 2026-10-12 |
| **Builds on** | [principles](report-analysis.md#4-the-principles), [audit](current-ui-audit.md), [design language](design-language.md), [interaction](interaction-and-feel.md) |

## 1. Information architecture

OPTS becomes **one application shell** with a persistent **navigation rail** on the left and a content area; everything the user does lives inside it. Today the application is a menu that opens separate full-screen screens.

| Group | Destination | Contains | Available |
| :--- | :--- | :--- | :--- |
| **Main** (≤ 5, UI-P02) | **Home** | continue the last drive, recent drives, quick actions, status of the installation | now |
| | **Drive** | *Plan a drive*: where, what, when, review | now |
| | **Library** | browse maps, vehicles, liveries, scenery with thumbnails and details; favourites | after M-0062; marked until then |
| | **Workshop** | editor projects, installed mods, packaging | **visible, marked "Feature not available yet!"** until Phase 6 |
| **Utility** | **Settings** | categorised, searchable | now |
| | **Help** | controls, guides, report an issue, about and licences | now |

Rules: a destination or control that does **not work yet stays visible but honest** (UI-P11, decision D-UI-09): it is disabled and carries the red text **"Feature not available yet!"** (component `NotAvailableYet`, one style everywhere), so nobody mistakes it for a bug and nobody forgets it. Every such place is registered with its milestone and appears in a generated list ([expandability §4](expandability.md#4-screen-and-navigation-registry)). Future destinations (timetables, career, multiplayer) join *Main* only while there are ≤ 5 items, otherwise they group under an existing one.

## 2. Navigation map

```
 First run ──► Home ◄──────────────────────────────────────────────┐
 (OMSI folder,  │  ├─ Continue ───────────────► Loading ──► Drive ──┤ end of session:
  catalogue)    │  └─ New drive ─► Plan a drive ─┘            │     │ summary ─► Home
                │                  Where ► What ► When ► Review     │
                ├─ Drive (same as New drive)                  Pause overlay (Esc)
                ├─ Library ─► detail ─► "Drive with this…"    ├ Resume
                ├─ Settings (categories)                      ├ Settings · Controls · Help
                └─ Help (controls, guides, report, about)     └ Leave drive (confirm) ─► Home
```

## 3. Journeys

### 3.1 First run

*Principles UI-P03, UI-P11.*

1. **Find OMSI.** The installation is searched as today (CLI, `$OMSI_PATH`, saved path, default Steam folders). A found folder is **offered for confirmation**, not silently adopted; if none is found the user chooses with the native dialog. A wrong folder explains what a valid one contains.
2. **Read the content.** The catalogue is built in the background with a visible stage ("Scanning maps… vehicles… repaints…") and a count; the user may already open Settings or Help.
3. **Welcome in one screen**: *Pick a map and a vehicle* (→ Plan a drive) or *Quick drive* (a sensible default map and vehicle that exist in the installation). No tour, no nag; tips appear once, contextually, and can be switched off.

### 3.2 Home

One dominant action. If a last drive exists: a **Continue** card (map photo, vehicle, time, weather) with a single primary *Drive*; below it *Recent drives* (≤ 6) and *New drive*. If none: *Quick drive* is the primary. A status chip shows *catalogue up to date / refreshing*; problems with the installation appear as a banner with an action, not as text in a card.

### 3.3 Plan a drive

*Replaces the three tabs of "Start New Game".*

A **stepper** with four steps, each a *master–detail* page (list left, details right), the **summary always visible** (UI-P03):

| Step | Content | Defaults and aids |
| :--- | :--- | :--- |
| **① Where** | map list (search, recents, favourites, filters by region/era), photo, description (in its original language, labelled), start point | last map; the map's default start point |
| **② What** | vehicle list with thumbnails, name and class (not file names), key facts that exist (length, doors, powertrain) — **facts without data are not shown**, not "N/A"; livery list with colour chips; *Configure…* for equipment options | last vehicle; a vehicle that exists on the chosen map's lists |
| **③ When** | quick chips (*Now · Morning · Noon · Evening · Night*), exact date and time, weather presets with icons and a one-line effect (visibility, grip); the era of the map is suggested | last choice; *Live weather* only after the feature works and with an explicit, explained opt-in (no hidden network call) |
| **④ Review** | one summary card with the three choices as editable links | **Drive** (primary), *Save as favourite drive* |

*Next* and *Back* sit in the footer, *Drive* is available from ① as soon as the defaults are valid (*Skip to review*), so the shortest path is **Home → Drive → Drive** (UI-P02). The state of the wizard is remembered across launches.

### 3.4 Loading

Plain stages with counts ("Loading the map: 12 of 31 tiles", "Preparing the vehicle", "Setting up light and weather"), the three-tile manifest as today, **real** progress, **Cancel** always available, a **controls card** to read while waiting. On success the screen **continues automatically** (no *Start driving* button that nobody can press). On problems it **stops** and shows *Ready with N problems* (*Details*, *Continue*) or *Failed* (*Retry*, *Choose another*, *Back*): errors are never overwritten (F03). Problems have a **severity** (one table in `in_game::load_report::severity_of`): **blocking** = the map, a map tile or the player vehicle did not load; **minor** = individual assets (textures, scenery meshes, fonts, text sources, protected meshes without a twin). Only blocking problems stop the screen; minor ones are kept in the `LoadReport`, the drive starts by itself, ONE toast "Loaded with N minor problems" with *Details* follows the first HUD frame, and the F10 panel shows the count. The first two outcomes apply to blocking problems. The screen uses the application theme (F10).

### 3.5 In game

The HUD shows only what a driver needs (see wireframe): a thin context bar (map, time, weather) that fades after a few seconds, the **instrument cluster** (speed, gear, brake state, later RPM and doors), a camera label on change, and **hints that fade** (*Esc Pause · F1 Help*). There are no permanent debug blocks; developers switch the developer panel on (F10).

### 3.6 Pause (Esc)

A centred panel over the paused scene: **Resume** (primary, default focus) · Vehicle · Settings · Controls · Report an issue · **Leave drive…** (confirm, destructive styling, last in the list and visually separated). *Esc* again resumes. Quitting the application is a separate item in the confirm dialog of *Leave drive* ("Leave to Home / Quit OPTS").

### 3.7 End of a session

*Principle UI-P12.*

Leaving a drive shows a short **summary** before Home: duration, distance, maximum speed, stops served (once passengers exist) and **any problems logged** with a *Report* button, plus *Drive again* (same settings), *Change vehicle*, *Home*. Only measures that are actually available are shown.

### 3.8 Settings

A left list of **categories** and a right pane of settings; a **search field** finds a setting by name; every row has a label, control, one-line description and *Reset*. Changes apply immediately; risky ones show the revert prompt ([interaction §6](interaction-and-feel.md#6-control-confirmation-undo)).

| Category | Examples |
| :--- | :--- |
| **Graphics** | window mode, resolution, vsync, texture filtering, look (Vivid / Natural / Bevy default), view distance |
| **Audio** | master, engine, ambience, UI volumes *(appears with audio, M-0027)* |
| **Controls** | key, gamepad and wheel bindings, sensitivities, per-vehicle controls |
| **Interface** | theme, accent, UI scale, density, language, units, HUD options, hints |
| **Gameplay** | difficulty and assists, clock and time scale, traffic and pedestrian density |
| **Content** | OMSI folder (changeable), catalogue refresh interval and *Rescan*, cache size |
| **Advanced** | developer panel, logging, diagnostics, database and file locations (the information that today sits in "General & Storage") |

### 3.9 Library

The place to *look at* content without committing to a drive: tabs for Maps, Vehicles, Liveries, Scenery; the same search/recents/favourites/filters and detail panels as the wizard (one pattern, built once); a *Drive with this…* action starts the wizard with the item preselected. In the editor era the same pages offer *Open in editor*.

### 3.10 Help and feedback

*Controls* (generated from the active bindings and the current vehicle's controls, so it is always right), short guides, *Report an issue* (the F1 report with a titled dialog and a visible screenshot preview), *About* with version and licences (replacing *Credits*). Context help opens at the relevant control list.

### 3.11 Errors and empty states

All use the templates of [interaction §5](interaction-and-feel.md#5-errors): *OMSI not found* (banner on Home, action *Choose folder*), *empty catalogue* ("No maps found — check the folder or rescan"), *vehicle failed to load* (dialog with alternatives), *settings could not be saved* (inline, with the path), *no search results* (offers *Clear filter*).

## 4. Wireframes

### 4.1 Home

```
┌──────────┬────────────────────────────────────────────────────────────────┐
│ OPTS     │  Welcome back                                          ?  ⚙   │
│          │ ┌────────────────────────────────────────────────────────────┐ │
│ ▌ Home   │ │ [ map photo, cropped to fill, scrim under the text ]       │ │
│   Drive  │ │ Continue                                                   │ │
│   Library│ │ Bad Hügelsdorf 2020 · MAN SD77 (Beige)                     │ │
│          │ │ 12:00 · clear summer day                    [  Drive ▶  ]  │ │
│ ──────── │ └────────────────────────────────────────────────────────────┘ │
│   Settings│ Recent drives                                  [ New drive ] │
│   Help   │ ┌──────────┐ ┌──────────┐ ┌──────────┐                         │
│          │ │ thumb    │ │ thumb    │ │ thumb    │                         │
│ v0.1.0   │ │ Berlin…  │ │ Grundorf │ │ Chicago… │                         │
└──────────┴────────────────────────────────────────────────────────────────┘
```

### 4.2 Plan a drive, step ① Where

```
┌──────────┬────────────────────────────────────────────────────────────────┐
│ rail     │ Plan a drive    ① Where ─── ② What ─── ③ When ─── ④ Review    │
│          │ ┌──────────────────────┬─────────────────────────────────────┐ │
│          │ │ 🔍 Search 43 maps…   │ Berlin-Spandau              [photo] │ │
│          │ │ Recent  Favourites ▾ │ 1986 – 1994 · 369 tiles · 16 starts │ │
│          │ │ ★ Berlin-Spandau  ◄  │ Start point  [ Spandau Westerw… ▾ ] │ │
│          │ │   Bad Hügelsdorf     │ Description (German, from the map)  │ │
│          │ │   Grundorf           │ …                                   │ │
│          │ └──────────────────────┴─────────────────────────────────────┘ │
│          │ Berlin-Spandau · MAN SD77 · 12:00 clear      [Back] [Next ▶]   │
└──────────┴────────────────────────────────────────────────────────────────┘
```

### 4.3 In-game HUD

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Bad Hügelsdorf · 08:30 · ☀ 24 °C                         (fades after 8 s) │
│                                                                            │
│                                                                            │
│                              3D scene                                      │
│                                                                            │
│  External camera                                                           │
│                                   ┌──────────────────────────────┐         │
│  Esc Pause · F1 Help              │   32 km/h        D     ⛔P   │         │
│                                   └──────────────────────────────┘         │
└────────────────────────────────────────────────────────────────────────────┘
```

### 4.4 Pause overlay

```
          ┌────────────────────────────────┐
          │ Paused                         │
          │ ▶ Resume              (focus)  │
          │   Vehicle                      │
          │   Settings                     │
          │   Controls                     │
          │   Report an issue              │
          │ ────────────────────────────── │
          │   Leave drive…                 │
          └────────────────────────────────┘
```

### 4.5 Settings

```
┌──────────┬─────────────────────────────────────────────────────────────────┐
│ rail     │ Settings                                  🔍 Search settings    │
│          │ ┌────────────┬─────────────────────────────────────────────────┐ │
│          │ │ Graphics ◄ │ Display                                         │ │
│          │ │ Audio      │ Window mode        [ Windowed ▾ ]       Reset   │ │
│          │ │ Controls   │ Texture filtering  ( Off | 4× | 8× | 16× )      │ │
│          │ │ Interface  │   Sharper textures at an angle, small cost.     │ │
│          │ │ Gameplay   │ Look               ( Vivid | Natural | Bevy )   │ │
│          │ │ Content    │   Colour and contrast of the 3D world.          │ │
│          │ │ Advanced   │                                                 │ │
│          │ └────────────┴─────────────────────────────────────────────────┘ │
└──────────┴─────────────────────────────────────────────────────────────────┘
```

## 5. From today's screens to the proposed ones

| Today | Becomes | Note |
| :--- | :--- | :--- |
| Main menu (sidebar, 6 controls) | **Home** + navigation rail | Escape no longer quits; *Shutdown* moves into the rail footer / pause confirm |
| Start New Game (3 free tabs) | **Drive → Plan a drive** (stepper, summary, remembered choices) | the footer summary and the duplicate in-panel summary merge into one |
| Settings (3 tabs, static text) | **Settings** (7 categories, real controls, search) | *General & Storage* text moves to *Advanced*; *UI & Themes* → *Interface* |
| Load Game (stub) | stays as an entry marked **"Feature not available yet!"** until a save system exists | *Continue* covers "resume the last drive" |
| Credits (stub) | **Help → About** | |
| Loading screen | **Loading** (themed, real stages, errors kept) | |
| HUD, F10 block | **HUD**, developer panel off by default | |
| F1 report dialog | **Help → Report an issue**, titled, with preview | F1 opens Help (D-UI-06) |
| Vehicle menu (L-Alt) | **Pause → Vehicle**, and the hold-Alt shortcut stays | |
| OMSI-folder prompt | **First run → Find OMSI** | also reachable in *Settings → Content* |

## 6. Decisions for you

| ID | Decision | Recommendation |
| :--- | :--- | :--- |
| **D-UI-07** | Navigation structure: left rail with Home / Drive / Library (+ Workshop later) and Settings / Help | yes |
| **D-UI-08** | Keep Escape as *back / pause* and never *quit/leave* without a confirmation | yes |
| **D-UI-09** | Show unavailable features with red "Feature not available yet!" instead of hiding them | **decided 2026-10-12** (changed from the recommendation to hide) |
