# F1 Reporting Feature — Design

> Status: **implemented (Parts A–C), awaiting verification sign-off (Part D).** Code: `crates/opts-bevy/src/reporting/`, widgets in `opts-gui` (`Modal`, `TextArea`), wiring in `opts-game/src/app.rs`. File format: [report-format.md](../formats/text/report-format.md).

**Decisions taken:** F1 = reporting (camera modes on 1–5, previous = 0); backdrop click does nothing; clipboard via `arboard`; `ReportContext` filled by `opts-game`; DejaVu Sans as default font and Material Icons for pictograms (`opts_gui::fonts`, `opts_gui::icons`); success toast after saving.

**Deviation:** input gating is applied per system with `.run_if(not_modal_active)` exactly as designed, and guarded by `crates/opts-game/tests/input_gating_audit.rs` (new input-reading files must be reviewed).

F1 anywhere in the application captures the primary window, opens a modal with the screenshot and a multiline text field, and on **Report** writes `{datetime}.png` and `{datetime}.report` to the reports directory. The simulation is paused while the modal is open.

## A1 — F1 conflict (resolved)

**Decision: option A.** F1 is reporting. The camera modes moved from F2–F6 to the number keys **1–5** (Ego, Cockpit, External, Passenger, Free roam) and "previous mode" moved from F1 to **0**. The gear selector lost its digit aliases (1/2/3); gears are E/N/R. Implemented in `opts-bevy/src/camera/input.rs`, `state.rs::key_number`, `hud.rs`, `vehicle_sim.rs`; see [camera-system.md](camera-system.md). F2 in the main menu still toggles the theme (`menu/theme_toggle.rs`) and is unrelated to the camera.

## Preconditions (verified)

| Check | Result |
| :--- | :--- |
| `Modal` in `opts-gui` | missing → Part B |
| multiline field | missing; `M3TextField` is single-line → Part B (extend, else new `TextArea`) |
| pause mechanism | none; use `Time<Virtual>` |
| screenshot API | Bevy **0.14.2** (before the 0.19 migration): `ScreenshotManager::take_screenshot(window: Entity, callback: impl FnOnce(Image) + Send + Sync + 'static) -> Result<(), ScreenshotAlreadyRequestedError>`; already used by `opts-game/src/in_game/screenshot.rs` |
| reports dir | `directories` 5.0 (workspace dep), `time` 0.3 for the local timestamp (needs the `local-offset` feature) |

## A2 — State machine

```rust
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ReportModalState { #[default] Inactive, Capturing, Open }
```

| From | Trigger | To |
| :--- | :--- | :--- |
| Inactive | F1 pressed | Capturing |
| Capturing | screenshot ready | Open |
| Capturing | capture failed / second request error | Inactive (+ `ERROR`) |
| Open | Report clicked and save succeeded | Inactive (+ `INFO`) |
| Open | Report clicked and save failed | Open (+ `WARN`, toast; user can retry) |
| Open | Cancel / Escape | Inactive (+ `DEBUG`) |

F1 while not `Inactive` is ignored.

`ReportModalData` resource: `screenshot: Option<Handle<Image>>`, `png: Option<Vec<u8>>` (encoded once, off-thread), `text: String`, `was_running_before: bool`, `capture_started_at: Instant`. A `ModalActive(bool)` resource mirrors `state != Inactive` and is what run conditions read (states are not visible to systems registered in `Startup`/menus without `in_state`; a plain resource is simpler and also covers `Capturing`).

## A3 — Input gating audit

Rule: every system that reads `ButtonInput<KeyCode>`, mouse buttons, mouse motion/wheel, or reacts to `Interaction` gets `.run_if(not_modal_active)`; the modal's own systems and the F1 trigger do not.

| Crate | System | Gated |
| :--- | :--- | :---: |
| opts-bevy::camera | `handle_camera_key_input`, `handle_camera_reset_input` | yes |
| opts-bevy::camera | `update_free_roam_camera`, `manage_free_roam_cursor` (must also release the cursor while the modal is open) | yes |
| opts-bevy::camera | `update_ego_look`, `handle_vehicle_view_wheel`, `handle_vehicle_view_drag` | yes |
| opts-bevy::vehicle_sim | `vehicle_input_system`, `vehicle_camera_system` | yes |
| opts-game::in_game | `escape_to_menu_in_game`, F10 overlay toggle (`omsi_world`) | yes |
| opts-game::menu | `menu_escape_exit_system`, `menu_keyboard_activation_system`, `menu_interaction_state_system`, `theme_toggle_system` | yes |
| opts-game::new_game_screen | `handle_*_selection`, `handle_back_button`, `handle_launch_button`, `handle_search_*`, `handle_mouse_scroll`, `handle_scroll_buttons`, `handle_calendar_interaction`, `handle_year_spinbox`, `handle_clock_picker_interaction`, `handle_time_adjustment`, `escape_to_menu_system` | yes |
| opts-game::settings_screen | `handle_*_clicks`, `escape_to_menu_system` | yes |
| opts-game::stub_screens | `escape_to_menu_system` | yes |
| opts-gui::interaction | `keyboard_focus_navigation_system` (Tab) | yes — the modal runs its own focus trap |
| opts-gui | `M3TextField` interaction/typing systems | yes for existing fields; the `TextArea` has its own ungated systems |
| opts-game | `screenshot::capture_screenshot_when_ready` (dev flag, no input) | no |

Clicks: the backdrop node carries `FocusPolicy::Block`, so `Interaction` on nodes beneath it stays `None`; the run condition is the second line of defence for keyboard-driven activation. The list is derived from a grep for `ButtonInput`, `MouseWheel`/`MouseMotion` readers and `Interaction` queries and will be re-run in C8.

## A4 — Screenshot capture

`ScreenshotManager::take_screenshot(primary_window, cb)` in `Update` after F1; the callback runs on the render-world result and sends the `Image` through a channel (`crossbeam`-free: `std::sync::mpsc` held in a resource polled each frame). The next frame the poll system adds the image to `Assets<Image>`, stores the handle, starts PNG encoding on `AsyncComputeTaskPool`, and moves to `Open`. The capture happens in the frame of the keypress, before the modal exists, so the modal is never in the shot. The preview is a `UiImage` in a node with `aspect_ratio` set from the image and `max_height: 25vh`/`max_width: 100%` (letterboxed via `ImageMode::Auto` inside a centred flex box). Failure modes: window minimized or resized mid-capture (callback never fires → 2 s timeout via `capture_started_at`, log `ERROR`, back to `Inactive`); a second request while one is pending returns `ScreenshotAlreadyRequestedError` (treated as failure).

## A5 — Pause semantics

- Bevy's default `Res<Time>` in `Update` is `Time<Virtual>`; `FixedUpdate` accumulates virtual time, so `vehicle_physics_step_system` (`opts-bevy/src/vehicle_sim.rs`) and the free-roam update (`Res<Time>`) freeze when `Time<Virtual>` is paused. `opts-sim` has no clock of its own — it takes `dt` from the caller.
- Systems that must keep running while paused use `Time<Real>`: the TextArea cursor blink (the existing search-field blink uses `Res<Time>` and would freeze, acceptable because it is gated anyway) and the modal toast.
- `was_running_before = !time.is_paused()` is read at F1 time, before pausing. On return to `Inactive`, `unpause()` only if that flag is true.
- Loading screens: world loading is driven by `Update` systems on real frames, not virtual time, so a pause does not stall loading; `Time<Virtual>` pause only affects time-integrated systems. Opening the modal during loading is allowed.

## A6 — `opts-gui` extensions

Exists: `M3TextField` (single line), `toast`, `card`, `button`, focus system. Missing and to be added:

- `components/modal.rs`: `ModalRoot` (backdrop, `FocusPolicy::Block`, `ZIndex::Global(1000)`), `ModalContent` (card), `ModalPlugin`, `ModalEvent::{CancelRequested}`; Escape emits `CancelRequested`; backdrop click is a no-op (destructive for a report); Tab/Shift+Tab focus trap limited to focusables under the modal.
- `components/text_area.rs`: extend the `M3TextField` text model rather than start from zero. `TextArea`, `TextAreaValue(String)`, cursor as char index, Enter/Backspace/Delete/arrows/Home/End/PageUp/PageDown, line wrap through Bevy `Text` layout, scroll by cursor line, `Ctrl+A` and, if `arboard` is acceptable as a new dependency (needs user approval), `Ctrl+C/V`. Pure text-mutation logic lives in a `TextBuffer` struct with unit tests.

## A7 — File save

The reports dir is `ProjectDirs::from("", "", "opts").data_dir()/reports` (Linux `~/.local/share/opts/reports`, Windows `%APPDATA%\opts\reports`, macOS `~/Library/Application Support/opts/reports`). It is created at startup and its path is logged at `INFO`. Saving runs on `AsyncComputeTaskPool`; the PNG bytes are already encoded, the `.report` text comes from `format.rs`. Files are written with `create_new` so a same-second collision yields `-1`, `-2`, … suffixes. On error: `WARN`, toast, modal stays open for retry. Nothing is ever written into the OMSI install directory.

## Module tree (Part C)

`crates/opts-bevy/src/reporting/{mod,state,trigger,capture,modal,save,pause,format}.rs`, each under 500 lines. Dependencies to add to `opts-bevy`: `directories`, `time` (`local-offset`), `image` (already present, PNG feature).

## Open points for approval

1. Backdrop click: no-op (proposed) or cancel?
2. Clipboard (`Ctrl+C/V`) needs `arboard`: add it, or skip clipboard for now?
3. Report metadata `Map`/`Camera`/`Vehicle` come from resources that live in `opts-game`; to keep `opts-bevy` independent, `ReportContext` is a resource `opts-game` fills each frame (proposed).
