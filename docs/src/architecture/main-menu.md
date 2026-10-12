# 🎬 Home and Application Shell

| Metadata | Value |
| :--- | :--- |
| **Document** | `main-menu.md` (the file keeps its name; it replaces the former *Main Menu* specification) |
| **Layer** | L6 Applications (`opts-game`) / GUI consumer |
| **Target Crate** | `crates/opts-game` (`shell/`, `home/`, `menu/`) |
| **Status** | 🟢 Implemented (M-0062, shell part); the screen contract is [screens.md](../roadmap/phase-9/screens.md) |
| **Date** | 2026-10-12 |

The sidebar *Main Menu* (five buttons, glass cards, developer text) is gone. OPTS starts in the **application shell**: a navigation rail on the left and a content area; the first page in it is **Home**. Why: [flow](../roadmap/phase-9/flow.md) and the [audit](../roadmap/phase-9/current-ui-audit.md).

## 1. Shell (`crates/opts-game/src/shell/`)

| Module | Role |
| :--- | :--- |
| `registry` | `ScreenDescriptor { id, title_key, icon, group, order, availability, state }`, `ScreenRegistry`, `App::register_screen`; the six built-in destinations (Home, Drive, Library, Workshop, Settings, Help) |
| `frame` | the frame (rail, banner area, content area), spawned once at start-up; rail items generated from the registry; icons only in the *Compact* layout class (< 900 px), icon + label otherwise; selected marker; red *Feature not available yet!* line for `NotYet` entries; theme toggle (F2) and *Quit OPTS* in the footer |
| `nav` | message `Navigate { target, mode }` (`home`, `settings/graphics`, `screen:help`), `NavigateBack`, `BackStack`, `PendingDeepLink`, `NavigationRefused`; control-protocol `ui.navigate` |
| `esc` | the one implementation of Esc / Back (`EscGate`: a dialog, an open combobox, a focused text field or toast owns the press) |
| `dialog` | confirmation dialogs (`DialogSpec`, `DialogChoice`; safe choice first, Esc cancels) |
| `banner` | banners (`show_banner`, de-duplicated by id, removed when their screen is left) |
| `keys`, `l10n/` | message keys and the English and German tables, merged into `Localization` with `Localization::extend` |

`AppState` hosts the pages: `MainMenu` = Home, `VehicleSelection` = Drive, `Settings`, `Help`, `InGame`. A page builds itself in `OnEnter` with `spawn_screen_root`, which puts it into the content area. While a drive runs the frame is `Display::None` and `ShellContent` is `None`.

## 2. Home (`crates/opts-game/src/home/`)

* **Continue** card from `DriveHistory::last()`: the map's `picture.jpg` as hero, cropped like CSS `cover` to the card (`hero::cover_rect`, re-cropped on resize; never a dark band beside the picture), a scrim whose strength is *computed* from the picture and the theme (`menu::background`), map, vehicle (livery), time and weather, one primary **Drive** button with default focus.
* **Recent drives** (the remaining entries of the history, at most five) as cards; a click starts that drive.
* **New drive** opens *Drive* (pushes Home on the back stack). Without any history **Quick drive** is the primary action and *New drive* is secondary.
* Starting a drive is the message `StartDrive(DriveRecord)`: the choices become the `NewGameSetup`, the drive moves to the front of the history (`drive_history.json`) and `AppState::InGame` follows.
* **Status chip**: *Reading content…* while the catalogue scan runs, *N maps · M vehicles* afterwards, *No OMSI folder* without an installation. **Banners**: *OMSI 2 was not found* (action *Choose folder*), *No maps or vehicles found*. The drive buttons are disabled with the reason as tooltip while no drive is possible.
* *Load game* stays visible with the red marker (`home.load-game`).
* The page is built again when the catalogue lists change (the scan finished after Home appeared), so the hero and the map names appear without a restart.

## 3. What was removed

The glass sidebar, the brand block, the info card ("Clean-Room Simulation Engine", "SQLite (WAL)…", raw paths), the footer text with a hard-coded version, the floating theme toggle, the Escape-quits-the-application handler and the stub screens (*Load Game*, *Credits*). The version in the rail comes from `CARGO_PKG_VERSION`.

## 4. Tests

`crates/opts-game/tests/shell_tests.rs` (rail from registry, availability, navigation, back stack, deep links, Esc with a dialog open), `src/home/tests.rs` (states, Continue, drive request, banner), `tests/ui_rules.rs` (target size, text size, disabled reasons, keyboard traversal on the shell and Home).
