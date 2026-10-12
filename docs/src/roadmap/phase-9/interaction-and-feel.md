# Interaction and Feel — How OPTS Should Behave

| Metadata | Value |
| :--- | :--- |
| **Status** | ✅ Approved 2026-10-12 (decisions recorded in the [phase page](README.md#decisions)) |
| **Date** | 2026-10-12 |
| **Builds on** | [principles](report-analysis.md#4-the-principles), [audit](current-ui-audit.md) (F01, F02, F11, F19, F20), [design language](design-language.md) |

"Feel" is what the interface does under the user's hands: how it answers, how forgiving it is, how fast, how predictable. This page defines the rules every screen and widget follows.

## 1. Devices and the action layer

| Device | Role | Notes |
| :--- | :--- | :--- |
| **Mouse + keyboard** | primary, for menus, setup and the editor | everything reachable by both |
| **Gamepad** | secondary, for menus and driving | not present today (F14) |
| **Steering wheel and pedals** | driving; menus through the wheel's buttons and D-pad, mapped like a gamepad | wheel axes are never used for menu navigation |
| **Touch** | not a target; sizes (UI-P01) keep it usable | |

Input never talks to widgets directly. A thin **action layer** translates devices into *actions*, so a new device or a rebinding changes one table, not every screen:

| Action | Keyboard | Gamepad | Meaning |
| :--- | :--- | :--- | :--- |
| `Navigate(dir)` | arrow keys | D-pad / left stick | move focus by direction |
| `Next/Previous region` | Tab / Shift+Tab | — | jump between *landmarks* (rail, content, footer) |
| `Next/Previous tab` | Ctrl+Tab / Ctrl+Shift+Tab, `[` `]` | LB / RB | switch tab or wizard step |
| `Confirm` | Enter, Space | A | activate the focused control |
| `Back` | Esc, Backspace (not in text fields) | B | one step back; see §6 |
| `Search` | Ctrl+F, `/` | Y | focus the search field of the current list |
| `Menu` | Esc (in game) | Start | pause overlay |
| `Help` | F1 *(D-UI-06)* | Select | context help |
| `Scroll` | PgUp/PgDn, Home/End | right stick | list scrolling |

**Implementation (`opts-gui`, M-0061 part B).** `opts_gui::actions::UiAction` is a Bevy message; `key_action` and `button_action` are the two tables above, the only place a binding lives. Notes where the code is more precise than the table:

- Tab means *next region*; on a screen with fewer than two landmark regions (every screen until the shell exists) it falls back to the next control in layout order, so Tab always works. `FocusNext`/`FocusPrevious` (control protocol: `ui.action next|previous`) ignore regions and exist for tests.
- Backspace, `/`, `[`, `]`, Space and PgUp/PgDn/Home/End are *text* while a text entry has focus and produce no action then.
- `Esc` produces `Back` everywhere; `Menu` comes from the gamepad Start button, and an in-game screen maps `Back` to the pause overlay itself. `F1` stays the report dialog until the Help milestone and is reserved for `Help`.
- The right stick produces `Scroll(Line)`; the left stick is edge-triggered at 60 % deflection. There is no repeat for the D-pad yet (keyboard repeat comes from the OS key events).
- `Confirm`, `Back` (open combobox, focused toast, focused text entry outside a modal), `Search`, `NextTab`/`PrevTab` (notebooks) and the arrows (tree rows, combobox headers, tab headers, spatial moves) are handled by the library; everything else is left to the screen, which reads `UiAction` too.
- The in-game HUD carries `PointerOnly` and never takes focus (section 8).

The existing control protocol gains the same actions (`ui.action navigate right`) so tests drive the UI exactly as users do ([expandability](expandability.md#7-testing)).

## 2. Focus and keyboard navigation

*Principles UI-P08, UI-P10.*

1. **A focus ring is always visible** when focus came from keyboard or gamepad (hidden after a mouse click, as in most systems). 2 px, `Secondary` role, 3:1 against the control and its surroundings. Today it exists only on the main menu (F01).
2. **Focus order is visual order**, left→right, top→bottom within a region; `tab_index` is not used to hand-order controls. Hidden, collapsed and filtered-out controls never take focus (today they can).
2a. *Implementation note:* "visual order" is the order of the layout tree (a depth-first walk, which is the reading order of the library's flex builders); arrow keys use the real screen geometry of the nodes, restricted to the current region. `tab_index` is ignored. Disabled controls take focus only when they carry a reason (`DisabledReason` or the *not available yet* marker) so the reason can be read; a hidden control (`Display::None`/`Visibility::Hidden` on it or an ancestor) never does, and a focused control that gets hidden loses focus.
3. **Regions (landmarks)**: *navigation rail*, *content*, *detail panel*, *footer actions*, *toasts*. Tab jumps between regions; arrows move inside one. Lists and grids: arrows move item to item, `Home/End` jump, type-ahead selects by first letters.
4. **Every control is reachable and activatable by keyboard**: tabs, tree items, combo boxes, switches, sliders, the theme toggle, HUD buttons. This is checked by an automated traversal test (UI-P08).
5. **Default focus** goes to the safest sensible control: the primary action when it is harmless, the search field on list screens, *Cancel* on destructive dialogs; never *Quit*.
6. **Focus is restored** after a dialog closes, a tab switches or a list reloads.
7. **Focus traps** exist only in modal dialogs (already implemented) and release on close.

## 3. States and feedback

*Principles UI-P04, UI-P06.*

Every interactive control defines the same seven states with **state layers** over its base colour (M3 values):

| State | Visual | Notes |
| :--- | :--- | :--- |
| **Rest** | base | |
| **Hover** | +8 % layer | pointer only |
| **Focus** | +10 % layer **and ring** | keyboard / gamepad |
| **Pressed** | +10 % layer, no scale jump | |
| **Selected** | container colour + check or marker (not colour alone) | lists, tabs, toggles |
| **Disabled** | 38 % content, 12 % container, no hover | **always with a reason**: tooltip on hover/focus and, where space allows, a caption ("Needs a saved game") |
| **Error** | error border + icon + message | fields and rows |

- **Pointer activation happens on release** inside the control; pressing and dragging away cancels (today it fires on press, F19).
- A control that **cannot work yet** is shown disabled with the red text **"Feature not available yet!"** (decision D-UI-09), never as a silently greyed button. A control that is *temporarily* unavailable shows its reason instead ("Needs a saved game").
- *Implementation (M-0061 part B):* `opts_gui::not_yet` holds `NotAvailableYet` (component: disabled, tooltip "Feature not available yet! (M-xxxx)", red `Error`-role `LabelMedium` (12 px) label with the exact wording inside the control, above the disabled scrim), `Availability { Enabled, Disabled(reason), NotYet(place) }` with `apply`, and the static registry `PLACES`. Every use names a registered place; `docs/src/reference/not-available-yet.md` is generated from it (`UPDATE_NOT_YET=1 cargo test -p opts-gui not_yet`), a test fails when it is stale or a place waits for a completed milestone, and debug builds log the list at startup.
- **Tooltips** after 600 ms hover or on keyboard focus; icon-only buttons always have one; they repeat the shortcut.

## 4. Timing and progress

| Duration of the operation | Required feedback |
| :--- | :--- |
| immediate | the control's pressed/selected state within **100 ms** |
| > 1 s | a progress indicator in the place where the result will appear (skeleton, spinner or bar) |
| > 5 s | determinate progress with a stage name in plain language, and **Cancel** |
| > 30 s | estimated remaining time or a count (tiles, textures), and what the user may do meanwhile |
| background (catalogue refresh) | a quiet status item in the shell, never a modal |

Loading progress is **real**, never simulated; the loading screen shows what is being prepared in the user's terms ("Loading the map: 12 of 31 tiles").

## 5. Errors

Errors follow one template and one of three placements.

| Element | Rule |
| :--- | :--- |
| **What happened** | a plain sentence naming the thing ("Couldn't load the vehicle *MAN SD77*") |
| **Why** | the cause if known, without jargon or paths in the first line ("The model file is missing") |
| **What you can do** | at least one action ("Choose another vehicle", "Open the log") |
| **Details** | expandable: paths, codes, log lines; *Copy* and *Report* buttons |

| Placement | Use |
| :--- | :--- |
| **Inline** (next to the field or row) | validation, a failed list item |
| **Banner** at the top of the screen | the screen can't do its job (empty catalogue, missing OMSI folder), stays until resolved |
| **Dialog** | the action can't continue and needs a decision |
| **Toast** | a finished background event with no decision; max 3 visible, stacked, never overlapping; errors are never *only* a toast. *As built:* `opts_gui::components::toast_stack` orders toasts by spawn sequence, shows the first three (the rest wait, hidden, with a stopped timer), puts the newest at the bottom with an 8 px gap, merges identical toasts (same dedupe key) into the oldest with a counter ("×3") and a restarted timer, and keeps them above content and below modals. Severity sets colours and duration (information 4 s, warning and error 10 s). The game side is `shell::notify`: `Commands::notify` for a toast and `Commands::notify_error`, which **takes the durable place as a required argument** (`Durable::Banner(spec)`, `Inline`, `Dialog`), so an error cannot be raised as a toast alone |

**Load results are never overwritten** (F03): the loading screen ends in *Ready*, *Ready with N problems* (with *Details* and *Continue*) or *Failed* (with *Retry*, *Choose another*, *Back*). Problems have a **severity** (one table in `in_game::load_report::severity_of`): **blocking** = the map, a map tile or the player vehicle did not load; **minor** = individual assets (textures, scenery meshes, fonts, text sources, protected meshes without a twin). Only blocking problems stop the screen; minor ones are kept in the `LoadReport`, the drive starts by itself, ONE toast "Loaded with N minor problems" with *Details* follows the first HUD frame, and the F10 panel shows the count.

## 6. Control, confirmation, undo

*Principle UI-P05.*

| Context | **Esc / Back** does |
| :--- | :--- |
| a dialog or overlay | closes it (cancels) |
| a text field | first press: clears the focus; never leaves the screen |
| a screen inside the shell | goes one level up |
| **Home** | nothing (quitting is an explicit action) |
| **in game** | opens the **pause overlay** and pauses the simulation; never leaves the session |
| loading | asks to cancel, returns to *Plan a drive* |

| Operation | Policy |
| :--- | :--- |
| Leaving a running session, quitting | **one confirmation** naming the consequence ("Leave the drive? Your progress on this route is lost."); skipped when nothing would be lost |
| Deleting user content (a livery, a saved layout) | confirmation listing exactly what goes, or undo for 10 s |
| Changing a setting | **applies immediately** and has *Reset to default*; risky settings (resolution, window mode) apply with a **15 s revert** prompt that defaults to *revert* if no one confirms (the OS-style safety net; it protects the user and is not time pressure on a decision) |
| Everything else | **undoable** instead of confirmed (editor: Ctrl+Z; New Drive: Back keeps choices) |

## 7. Text, lists, scrolling

- **Text fields** support typing with the platform's layout and IME, caret movement, selection, copy and paste, and clear (×); the Linux-only layout detector goes away (F11). Search filters as you type with a 150 ms debounce and keeps the selection when still visible.
- **Long lists are virtualised**: only visible rows exist as entities (today 956 vehicle cards are spawned and hidden); keyboard type-ahead and a scroll bar with a minimum 24 px thumb; wheel, drag and keyboard scrolling all work.
- **Selection** is a single, obvious state (fill + marker); double-click or Enter activates; the detail panel follows the selection without a second click.
- **Empty states** explain and offer an action ("No vehicles match 'xyz' — clear the filter").

## 8. In-game feel

*Principle UI-P13.*

- **The HUD never takes input focus** and never moves; panels appear only in fixed regions. Opening any menu pauses the simulation and releases the cursor; closing restores both exactly (today L-Alt is a held modifier and P only logs).
- **Cursor**: locked and hidden in free-look and driver views, visible in any overlay; a visible hint explains the key that frees it.
- **Camera and mode changes** give a 1.5 s on-screen label ("External camera").
- **Alerts** while driving are short, positioned at the instrument cluster, and may not cover the view ahead; sounds will support them when audio exists.
- **Input gating** (already enforced by `not_modal_active` and an audit test) stays the single mechanism.

## 9. Proposed global shortcuts

| Key | Action | Today |
| :--- | :--- | :--- |
| Esc | Back / pause (see §6) | quits or leaves the session |
| **F1** | **Help and controls** overlay, with *Report an issue* | opens the report dialog directly |
| F10 | developer panel (off by default; *Settings → Advanced → Developer tools*) | debug text on by default |
| F11 | fullscreen | not present |
| Ctrl+, | open Settings | — |
| Ctrl+K | command palette (editor first) | — |

**D-UI-06 — F1.** Convention is *F1 = help*; today F1 opens the report dialog and users of the report feature know it. Recommendation: F1 opens *Help and controls* where *Report an issue* is the prominent button (and stays in the pause menu); the control-protocol command `report` is unchanged.

## 10. Performance and latency budget

- Visible response ≤ 100 ms; the UI systems may take ≤ **1.0 ms per frame at 1080p** while driving (no per-frame rebuilds; change-driven updates only).
- Lists: ≤ 200 live row entities regardless of catalogue size.
- Screen transition: first frame ≤ 100 ms, content ≤ 400 ms; heavy work (catalogue queries, thumbnails) runs off the main thread with skeleton placeholders.
