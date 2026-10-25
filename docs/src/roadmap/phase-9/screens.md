# Screen Specifications (shell, Home, first run, pause, session end, Help)

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Implemented with M-0062 (shell part); this page is the contract the code and the tests follow |
| **Date** | 2026-10-12 |
| **Builds on** | [flow](flow.md) (journeys, wireframes), [interaction and feel](interaction-and-feel.md) (states, errors, Esc), [expandability §4](expandability.md#4-screen-and-navigation-registry) |
| **Not covered** | *Plan a drive*, the loading screen ([wizard agent](M-0062.md)) and *Settings* ([M-0063](M-0063.md)); they only appear here as destinations |

Copy lives in message keys (`crates/opts-game/src/shell/l10n/shell-en.ftl` and `shell-de.ftl`, merged into the `Localization` at start-up); the keys below are the contract. Strings are written in English; German is complete.

## 1. Shell frame

```
┌────┬───────────────────────────────────────────────────────────────┐
│rail│ banner area (per screen, optional)                            │
│    ├───────────────────────────────────────────────────────────────┤
│    │ content area  (ShellContent)                                  │
└────┴───────────────────────────────────────────────────────────────┘
        toasts: bottom right, above content, below modals
```

| Part | Rule |
| :--- | :--- |
| **Rail** | 72 px (icons only) in *Compact* (< 900 px), 240 px (icon + label) in *Medium* and *Expanded*. Order: brand (`OPTS`), main group (Home, Drive, Library, Workshop), spacer, utility group (Settings, Help), footer (theme toggle, version). Generated from the screen registry. |
| **Entry** | icon, label (`shell-nav-<id>`), `UiId` `shell.nav.<id>`, tooltip with the label (always, needed in Compact), selected marker (container colour + bar, not colour alone), `NotYet` entries disabled with the red text, `Disabled(reason)` entries with the reason as tooltip. |
| **Content area** | fills the rest; every screen builds into it with `spawn_screen_root`. Hidden (`Display::None`) while a drive runs; `ShellContent` is then `None` so full-window overlays (loading, HUD) build as roots. |
| **Footer** | theme toggle (`shell-theme-to-light` / `shell-theme-to-dark`, F2) and `shell-version` (`v{ $version }` from `CARGO_PKG_VERSION`). |

### Navigation

| Concept | Behaviour |
| :--- | :--- |
| Registry | `ScreenRegistry` of `ScreenDescriptor { id, title_key, icon, group, order, availability, state }`; `state` is the `AppState` that hosts the page. One module + one registration per screen. |
| Navigate | message `Navigate { target }`; `target` is `home`, `settings`, `settings/graphics` (deep link `screen:settings/graphics`). Checked against `availability`; a `NotYet`/`Disabled` target is refused (a toast names the reason). The rail, Home, Help, the protocol command `ui.navigate <screen>` and tests all send it. |
| Back stack | owned by the shell. A navigation from the rail **replaces** the stack (the rail is flat); navigation from inside a screen (*New drive*, *Open this setting*) **pushes**. *Back* pops; an empty stack goes to Home. |
| Deep-link payload | `PendingDeepLink(Option<String>)` holds the part after the first `/`; the target page reads and clears it. |

### Esc (one implementation, `interaction-and-feel §6`)

| Context | Esc does |
| :--- | :--- |
| a modal / dialog / overlay (pause menu, report, confirmation) | closes it (cancels); the topmost only |
| a combobox, a toast, a focused text field | the library's `Back` first (closes the popup / dismisses / drops focus); the shell does nothing in the same frame |
| a screen reached from another (back stack not empty) | goes one level up |
| a rail destination without back stack | goes to **Home** |
| **Home** | nothing (quitting is explicit: the pause dialog and the rail footer menu never quit without confirmation) |
| in game | opens the **pause overlay**, pauses the simulation, releases the cursor; again resumes |
| loading | asks to cancel (owned by the loading screen) |

## 2. Home (`screen: home`, `AppState::MainMenu`)

Layout (Expanded): header line *Welcome back* / *Welcome to OPTS*; status chip right; banners above; the **Continue** card with hero; **Recent drives** (≤ 6, cards of 3 per row in Expanded, 2 in Medium, 1 in Compact), **New drive** button right of the heading.

| State | Content |
| :--- | :--- |
| **No history** | heading `home-welcome-first`; primary **Quick drive** card (`home-quick-title`, `home-quick-body`, button `home-quick-action`), secondary **New drive** |
| **History** | heading `home-welcome-back`; **Continue** card: hero (map photo, `cover` crop, computed scrim), `home-continue-title`, line `map · vehicle (livery)`, line `time · weather`, primary **Drive** (`home-continue-drive`, default focus); **Recent drives** cards (each: map, vehicle, time; click starts that drive); **New drive** (`home-new-drive`) |
| **Catalogue refreshing** | chip `home-status-refreshing`, count of maps found so far; Continue and Quick drive work with what exists |
| **Catalogue ready** | chip `home-status-ready` (`{ $maps } maps · { $vehicles } vehicles`) |
| **OMSI not found** | banner `home-banner-no-omsi-title` / `-body` with action **Choose folder** (`home-banner-choose-folder`), drive buttons disabled with the reason `home-reason-no-omsi` |
| **Empty catalogue** | banner `home-banner-empty-title` / `-body`, actions **Choose folder**, **Rescan** (not-yet until a rescan command exists) |
| **Load game** | secondary entry `home-load-game` (`home.load-game`): opens the save list overlay ([Session save](../../architecture/session-save.md) §6); the not-yet place is gone |

Hero image: the photo fills the card at its aspect ratio with `object-fit: cover` semantics (the node is wider than the photo's crop: the image node uses `ImageNode` with a computed `Node` size and `overflow: clip` so no band is visible); a scrim whose strength is computed from the picture's lightness and the theme guarantees 4.5:1 for the text on it. Missing photo: a `SurfaceContainerHigh` card with the map initial.

Removed from Home: "Clean-Room Simulation Engine", "SQLite (WAL) · Material Design 3 Glassmorphism", raw paths, hard-coded version text.

## 3. First run (modal flow over Home, `OmsiSetup`)

Start condition: no installation known (`settings.omsi_path` empty or no longer valid, none given on the command line or in `$OMSI_PATH`, and Steam's library configuration names none). Since 2026-10-25 an installation found through Steam is used without a dialog ([startup](../../architecture/application-startup.md#finding-omsi-through-steam)); the former **Found** step (*Use this folder* / *Choose another folder…*) was removed.

| Step | State | Copy keys | Actions |
| :--- | :--- | :--- | :--- |
| **Choose** | nothing found | `firstrun-choose-title`, `firstrun-choose-body` (what a valid folder contains: `maps` and `Vehicles`) | **Choose folder…** (native dialog, own thread), **Quit** |
| **Wrong folder** | answer fails `root_from_selection` | inline error `firstrun-wrong-folder` (`{ $path }`) beside the buttons, in the error role, with the rule again | the same buttons |
| **Cancelled dialog** | no answer | `firstrun-cancelled` inline | the same buttons |
| **Reading** | installation known, catalogue running | `firstrun-reading-title`; stage line `firstrun-stage-<stage>` + count `firstrun-count` (`{ $done } of { $total }`) from the catalogue service | **Continue in background** (the user may open Settings or Help), no modal |
| **Welcome** | catalogue has at least one map and one vehicle | `firstrun-welcome-title`, `firstrun-welcome-body` | **Pick a map and a vehicle** (→ Drive), **Quick drive**; dismisses once and never again (`first_run_done` remembered in `drive_history`'s sibling `first_run.json`, so no settings key is needed) |

Changing the folder later: `Settings → Content` calls `first_run::choose_omsi_folder` (a function that opens the native dialog and validates; the result is a `OmsiFolderChosen(Result<PathBuf, ChooseError>)` message). A **confirmed** folder is saved (`settings.omsi_path`) and the catalogue is rebuilt.

## 4. Pause (in game, Esc)

Centred panel over the paused scene (`Modal`); simulation paused (`Time<Virtual>`), cursor released, the theme switch lock stays set.

| Item | Key | Behaviour |
| :--- | :--- | :--- |
| title | `pause-title` | |
| **Resume** (default focus) | `pause-resume` | closes, restores pause state and cursor |
| Vehicle | `pause-vehicle` | closes the pause panel and opens the vehicle menu (L-Alt menu); Esc there returns to the game |
| Save game… / Load game… | `pause-save`, `pause-load` | the pause panel steps aside and the save list opens as a modal (name field, *Save*, rows with *Load*, *Overwrite*, *Rename*, *Delete*); Esc returns to this panel. F5 and F9 quick save and load from the game |
| Settings | `pause-settings` | opens the settings page as an overlay over the paused scene (`settings_screen::overlay`); Esc or *Close* returns to this panel |
| Controls | `pause-controls` | opens *Help → Controls* as an overlay (the same page content, in a modal); the Help overlay |
| Report an issue | `pause-report` | the report dialog |
| ─── separator | | |
| **Leave drive…** | `pause-leave` | confirmation dialog |

*Settings in the pause panel*: the shell's content area is hidden in game, so the same page (`spawn_settings_page`) is hosted in a modal (`SettingsOverlay`). The pause panel steps aside and returns when the overlay closes (`pause::return_after_overlay`).

**Behaviour as built.** Esc while the drive is still loading cancels the load (the loading screen of the wizard agent owns that later); Backspace never opens the overlay (a vehicle uses it); the overlay steps aside while the leave dialog is open and returns on *Cancel*; *Report an issue* closes the overlay, takes the screenshot of the game, and the overlay returns when the report dialog closes. The HUD's top-right button reads *Pause [Esc]* and opens the overlay.

**Leave dialog**: title `leave-title` ("Leave the drive?"), body `leave-body` ("Your progress on this route is lost."), actions **Leave to Home** (`leave-home`, destructive styling), **Quit OPTS** (`leave-quit`), **Cancel** (`leave-cancel`, default focus). Always shown (the user chose to end the session; Esc cancels and the pause panel returns). Only a drive that moved leaves a summary on Home.

The HUD's *Exit to Menu* button opens the same pause panel (not a direct exit).

## 5. End of session summary

Shown after *Leave to Home* from a drive that moved, over Home (modal-free page `session_summary`, `AppState::MainMenu` with a summary card at the top) with:

| Row | Key | Source |
| :--- | :--- | :--- |
| Duration | `summary-duration` (`{ $time }`) | session clock |
| Distance | `summary-distance` (`{ $km }`) | integrated player speed |
| Maximum speed | `summary-max-speed` | maximum of player speed |
| Problems logged | `summary-problems` (plural) with **Report** (`summary-report`) | the reports/diagnostics counter that exists; row hidden when it is zero |
| Stops served | not shown (no passengers yet) | |

Actions: **Drive again** (`summary-again`, same `DriveRecord`), **Change vehicle** (`summary-change-vehicle`, → Drive with the map kept), **Home** (`summary-home`). Only measures that exist are shown; there are no placeholders.

## 6. Help (`screen: help`, `AppState::Help`)

Left list (tabs) **Controls · Guides · Report an issue · About** and a right pane.

| Tab | Content |
| :--- | :--- |
| **Controls** (`help-controls`) | generated from `help::controls::GROUPS`, the one registry of the keyboard bindings (Driving, Camera, Menus); a test checks every listed key against the source file of the input system that reads it. The per-vehicle list is not part of it |
| **Guides** (`help-guides`) | red *Feature not available yet!* (`help.guides`, M-0064) |
| **Report an issue** (`help-report`) | what is sent (screenshot, log excerpt, the text), the **Report an issue** button that opens the existing report dialog; a failure produces an error toast and a banner on the page |
| **About** (`help-about`) | `OPTS`, version, `help-about-licence` (MIT or Apache-2.0), the third-party notices (Bevy, Rust, Noto Sans, JetBrains Mono, Material Icons: from [legal-licensing](../../contributing/legal-licensing.md)), the clean-room statement, project link |

F1 opens Help (D-UI-06) from anywhere with a window: the page in the shell, an overlay (controls table, *Close*, *Report an issue*) over the paused scene during a drive; closing the overlay returns to the pause menu. Pause → *Controls* opens the same overlay. The report dialog has a title, a visible screenshot preview with a caption, localised texts, and a failure to save (no reports folder, I/O error) is shown inline in the dialog and as a toast. The control-protocol command `report` is unchanged. Deep links: `screen:help/about` (tabs `controls`, `guides`, `report`, `about`).

## 7. Notifications

Implemented as described in [interaction and feel §5](interaction-and-feel.md#5-errors) (*Toast* row).

| Kind | Rule |
| :--- | :--- |
| **Toast** | `Commands::notify(severity, key, args)`; at most 3 visible, newest at the bottom, 8 px gap, no overlap; identical toasts (same key and variables) are merged into one with a counter; below modals, above content. Information 4 s, warnings and errors 10 s; Esc dismisses a focused toast (library). |
| **Error** | `Commands::notify_error(key, args, Durable)`: the durable place is a required argument (`Banner` is shown by the service; `Inline` and `Dialog` are put there by the owner). Used by the first-run flow, the report dialog (inline text and toast) and the navigation guard. |
| **Banner** | `shell::banner::show_banner(BannerSpec { id, severity, title_key, body_key, args, action, owner })` per screen, at the top of the content area; de-duplicated by `id`; removed when the owning screen is left or by `dismiss_banner`; the action arrives as the message `BannerAction`. |
| **Refused navigation** | a toast names the destination (`shell-nav-refused`); the rail entry itself already shows the red marker or the reason. |

## 8. Registered not-yet places added here

| Place id | Where | Milestone |
| :--- | :--- | :--- |
| `nav.library` | rail, Library | M-0062 (Library, after the wizard's master–detail pattern) |
| `nav.workshop` | rail, Workshop | M-0051 |
| `help.guides` | Help, Guides | M-0064 |
| `home.rescan` | Home banner, Rescan | M-0063 |

## 8a. In-game HUD (M-0071)

Not a screen of the shell but the overlay of a drive; see [HUD registry](../../architecture/hud-registry.md). Regions: status bar top left and *Pause [Esc]* top right, the camera label below the bar, the **cluster at the bottom left** (hints, alert line, speed / gear / parking brake) and the **mini map at the bottom right**. Toasts stack above the map. The HUD is laid out for 1024 x 640 up to 2560 x 1440 at a UI scale of 75 to 200 % (a compact layout below 760 x 420 layout px); `tests/hud_matrix.rs` and the golden `hud-1280x720` guard it.

## 9. Test matrix (S7)

Rail from registry · availability rendering · Esc matrix (§1) · Continue → drive request · first-run decisions (found, wrong folder, cancelled, confirmed) · pause/leave flow · summary content · CI rules (target ≥ 32 px, text ≥ 12 px, keyboard traversal, no literal colours or strings) on every new module.
