# 📸 F1 Reporting Feature

You are implementing a new application-wide feature: **F1 reporting**.
When the user presses F1 anywhere in the application, a screenshot is
captured, a modal opens with the screenshot and a text field, and the
user can submit the screenshot plus their written report to disk.

This is a MULTI-PHASE task. Design first, implement second, verify
third. Checkpoints between each.

The feature touches several subsystems — rendering, UI, input gating,
simulation pause, file I/O, and possibly `opts-gui` extensions. Do
not start coding until the design is approved.

---

## ⚠️ 0. Conflict to Resolve First

**F1 is currently assigned to the camera system as "switch to
previous camera mode"** (see `docs/src/architecture/camera-system.md`).

This creates a hard conflict with F1 reporting.

**Before proceeding, produce a decision:**

| Option | Behavior |
| :--- | :--- |
| **A. F1 becomes reporting.** | Camera's "previous mode" moves to a different key (recommend `Shift + F1` or `Alt + F1`). |
| **B. F1 stays camera.** | Reporting moves to a different key (recommend `F12` — the standard Windows screenshot key). |
| **C. F1 is context-sensitive.** | In-simulation: reporting. In-menu: previous camera. Complex and easy to get wrong. |

**Do not implement either behavior until the user chooses.** Report
the three options with tradeoffs and STOP.

Only after the user confirms a resolution does the rest of this task
proceed.

---

## 1. Configuration

### OPTS repository root
The current working directory. Confirm by checking for a `Cargo.toml`
with `[workspace]` and a `docs/src/SUMMARY.md`.

### Target crates
- `crates/opts-gui/` — if modal and/or multiline text field need
  extending
- `crates/opts-bevy/` — screenshot capture and modal plugin
- `crates/opts-game/` — F1 keybinding, pause integration, wiring
- `crates/opts-sim/` — pause state (if not already exposed)

### Output targets
- `docs/src/architecture/reporting-feature.md` — the design doc
- `docs/src/formats/text/report-format.md` — the `.report` file spec
- `crates/opts-bevy/src/reporting/` — the new module tree
- `docs/crates/opts-bevy.md` — updated

### Related context (read first)
- `docs/src/architecture/camera-system.md`
- `docs/src/architecture/gui-design-system.md`
- `docs/src/architecture/application-startup.md`
- `docs/crates/opts-gui.md`
- `docs/crates/opts-game.md`
- `docs/src/contributing/standards.md`
- `docs/src/adr/0004-logging-architecture.md`
- `docs/src/adr/0002-clean-room-policy.md`

---

## 2. Guardrails

### ❌ Forbidden

- Do not disassemble, decompile, or inspect `OMSI.exe` or any DLL.
- Do not commit any OMSI asset to the repository.
- Do not write screenshots into the OMSI installation directory.
- Do not use `println!` for diagnostics. Use `tracing`.
- Do not block the Bevy render loop waiting for file I/O. All saves
  are asynchronous.
- Do not implement reporting as a full-screen state transition that
  destroys the simulation. The simulation must be preserved across
  the modal.
- Do not skip the design phase. Modal architecture and input gating
  are easy to get wrong and expensive to fix.
- Do not change the camera system's keybinds without the user's
  resolution from Section 0.

### ✅ Permitted

- Read every file in the repository.
- Add a `reporting` module to `opts-bevy`.
- Add a `Modal` primitive and a `TextArea` (multiline text field)
  to `opts-gui` if they don't exist.
- Add a `SimulationPaused` resource or state to `opts-sim` if it
  doesn't exist.
- Add `opts-game` dependencies.
- Add the F1 keybinding to the input system.
- Add integration tests that simulate the flow headlessly.

---

## 3. Preconditions

Verify and report before starting:

### 3.1 Crate and API checks

| Check | How to verify | Result |
| :--- | :--- | :--- |
| `opts-gui` has a `Modal` component | Read `crates/opts-gui/src/components/` | ? |
| `opts-gui` has a multiline text field | Read `crates/opts-gui/src/components/` | ? |
| `opts-sim` exposes a pause mechanism | Read `crates/opts-sim/src/` | ? |
| Bevy's screenshot API is available | Check `Cargo.toml` for `bevy` features including `bevy_render` and inspect `bevy::render::view::screenshot` | ? |
| `opts-game` uses `Time<Virtual>` or similar for sim timing | Read `crates/opts-game/src/` | ? |
| Reports directory path is resolvable | `directories` crate is a workspace dependency | ? |
| Camera system F1 binding is implemented or planned | Read `docs/src/architecture/camera-system.md` | ? |

For each check, report what exists and what is missing. If `Modal`,
`TextArea`, or a pause mechanism is missing, that is a scoped
implementation task inside this feature (Part C), not a blocker.

### 3.2 Bevy screenshot API verification

Confirm the exact API for the Bevy version in use. Common patterns:

- `bevy::render::view::screenshot::Screenshot::window(entity)` with
  a `.observe(save_to_disk(path))` observer
- `ScreenshotManager::take_screenshot(window, callback)`
- `Screenshot::primary_window()`

Report the signature you will use. Do not guess — read the Bevy
version's actual API.

### 3.3 Reports directory

Determine the reports path via the `directories` crate:

- Linux: `~/.local/share/opts/reports/`
- Windows: `%APPDATA%\opts\reports\`
- macOS: `~/Library/Application Support/opts/reports/`

Confirm the directory can be created at startup. Log its resolved
path at `INFO` level when the app starts.

**If any precondition reveals a missing subsystem that isn't
obviously a Part C task, STOP and report.**

---

## 4. Feature Specification

### 4.1 Trigger

On **F1 key press** (or the resolved key from Section 0), **anywhere
in the application**:

1. Capture a screenshot of the primary window.
2. Open a modal.
3. If the simulation is running, pause it.

The modal must appear even if the app is in the main menu, a
loading screen, or in-simulation.

### 4.2 Modal Layout

The modal is a full-screen overlay with a semi-transparent backdrop
and a centered content card.

**Structure:**

```
FullScreenBackdrop (node, absolute, 100%×100%, rgba(0,0,0,0.6))
└── ModalCard (centered, ~70% width, ~90% height)
    ├── ScreenshotPreview (top ~25% of viewport height)
    │   └── Image (screenshot texture, letterboxed, fit within)
    ├── DescriptionLabel ("Describe the issue:")
    ├── TextArea (multiline, fills available vertical space)
    └── ButtonRow (bottom, horizontal)
        ├── ReportButton   [(/) Report]
        └── CancelButton   [(x) Cancel]
```

**Sizing:**
- Modal card: max-width 1200px, min-width 600px, ~90% of viewport
  height.
- Screenshot preview: top 25% of viewport height. Preserve aspect
  ratio; letterbox as needed.
- TextArea: fills the remaining vertical space between the
  screenshot and the button row.
- Button row: fixed height at the bottom.

**Colors:**
- Backdrop: theme token, alpha ~0.6, no blur (unless backdrop blur
  is available — treat as stretch).
- Modal card: theme `surface` color, glassmorphic (translucent) if
  `opts-gui` supports it.
- Text area: theme `surface_variant`.
- Buttons follow `opts-gui` M3 conventions.

**Focus:**
- On modal open, focus the TextArea so the user can type
  immediately.
- Escape key = Cancel (equivalent to clicking Cancel).

### 4.3 Screenshot Capture

The screenshot must:

1. Capture the primary window, not any individual camera.
2. Not include the modal itself (the modal opens after capture).
3. Be captured at the current window resolution.
4. Be saved as PNG (`{datetime}.png`).

**Async flow:**

1. F1 pressed → set `ReportModalState::Capturing`.
2. Spawn a Bevy screenshot entity targeting the primary window.
3. When the screenshot is ready (observer fires with `Image`):
   - Store the `Image` handle in a resource.
   - Transition to `ReportModalState::Open`.
   - Spawn the modal UI.

The modal UI opens only once the screenshot is available — not
before. This prevents the user from seeing an empty preview.

### 4.4 Modal Input Gating

While the modal is open, all input to the underlying application is
blocked, except:

- Input to the modal's TextArea (text typing).
- Escape key (cancel).
- The two buttons (Report, Cancel).

**Implementation approach (recommended):**

Introduce a `ModalActive(bool)` resource. Every input system in the
app that should be gated reads a run condition:

```rust
fn not_modal_active(modal: Res<ModalActive>) -> bool {
    !modal.0
}
```

Systems register with `.run_if(not_modal_active)`. The modal's own
systems do not use this condition.

This is invasive but correct. Every input-driven system must
register the condition. Audit the workspace for input systems and
list them in the design doc.

**Alternative (less invasive, less correct):** gate only the camera
and simulation input systems, and rely on Bevy UI focus to prevent
UI clicks from reaching background widgets. This is faster but
leaves gaps — throttle input, keyboard shortcuts, etc.

**Recommendation:** the resource-based approach. Document the
complete list of gated systems in the design doc.

### 4.5 Pause and Resume

While the modal is open:

- If the simulation is running, it must pause.
- On modal close, it must resume — only if it was running before.

**Implementation approach (recommended):**

Use Bevy's `Time<Virtual>::pause()` and `.unpause()`:

```rust
fn on_modal_open(mut time: ResMut<Time<Virtual>>) {
    time.pause();
}

fn on_modal_close(mut time: ResMut<Time<Virtual>>) {
    time.unpause();
}
```

This is the least invasive pause mechanism — all systems that use
virtual time naturally freeze when paused, and resume without any
state machine changes. Physics, animation, and simulation all
pause uniformly.

**Do not pause `Time<Real>`** — that would freeze input handling
and the modal itself.

**Preservation of prior state:** If the simulation was already
paused before F1 was pressed (e.g., user opened a settings menu),
the modal must not unpause it on close. Track `was_running_before:
bool` in the modal state and only unpause if `was_running_before`
was true.

### 4.6 Report Submission

On **Report button click**:

1. Save the screenshot as `<reports_dir>/{datetime}.png`.
2. Save the report text as `<reports_dir>/{datetime}.report`.
3. Close the modal.
4. Resume simulation (if it was running).
5. Log an `INFO` event: "Report saved to {path}".
6. Optionally show a brief toast or confirmation.

On **Cancel button click**:

1. Discard the screenshot and text.
2. Close the modal.
3. Resume simulation (if it was running).
4. Log a `DEBUG` event: "Report cancelled".

On **Escape key**:

- Same as Cancel.

### 4.7 File Naming

Filename format: `{datetime}.png` and `{datetime}.report`, where
`{datetime}` is:

```
YYYY-MM-DD_HH-MM-SS
```

Example: `2026-10-10_14-30-52.report`.

**Rationale:**
- ISO 8601-style with colons replaced by dashes for Windows
  compatibility.
- Lexically sortable.
- Unambiguous.
- Local time (not UTC) for user familiarity — note in the report
  file itself that the timestamp is local.

If two reports are saved in the same second (edge case), append
`-1`, `-2`, etc. to the filename.

### 4.8 Report File Format

The `.report` file is a **text file with a metadata header and the
user's description**. Format:

```
OPTS Report
===========

Timestamp:   2026-10-10T14:30:52+02:00
Version:     0.1.0-alpha
Git:         abc1234
Platform:    linux
Map:         Berlin-Spandau
Camera:      F4 External
Vehicle:     MAN_NG272 (if any)
Focused UI:  TextArea

--- Description ---

The user's text appears here verbatim,
preserving newlines and any UTF-8 characters.
```

Fields beyond the description are **best-effort**. If a field
cannot be sourced (e.g., no map loaded), it is written as `(none)`.

**Do not attempt to parse or interpret the description text.**
Write it verbatim.

**Encoding:** UTF-8, no BOM. Line endings: `\n` (Unix-style),
even on Windows. Document this in the format spec.

The format spec lives at
`docs/src/formats/text/report-format.md`.

---

## 5. Part A — Design

Produce `docs/src/architecture/reporting-feature.md` before writing
any code.

### Stage A1 — Resolve the F1 Conflict

Report the three options from Section 0 with tradeoffs. **STOP and
wait for user decision before proceeding.**

### Stage A2 — Modal State Machine

Define the states:

```rust
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportModalState {
    Inactive,
    Capturing,   // F1 pressed, waiting for screenshot
    Open,        // Modal visible, waiting for user action
}
```

Define the transitions:

| From | Trigger | To |
| :--- | :--- | :--- |
| Inactive | F1 pressed | Capturing |
| Capturing | Screenshot ready | Open |
| Capturing | Screenshot failed | Inactive (+ log error) |
| Open | Report clicked | Inactive (+ save) |
| Open | Cancel / Escape | Inactive |

Also define a `ReportModalData` resource holding:
- `screenshot: Option<Handle<Image>>`
- `text: String` (the TextArea content, kept in sync)
- `was_running_before: bool`
- `capture_started_at: Instant`

### Stage A3 — Input Gating Audit

Enumerate every input-driven system in the workspace. Produce a
table:

| Crate | System | Should be gated? |
| :--- | :--- | :---: |
| `opts-bevy::camera` | `handle_camera_key_input` | yes |
| `opts-bevy::camera` | `handle_camera_wheel_input` | yes |
| `opts-bevy::camera` | `handle_camera_pan_input` | yes |
| `opts-game::menu` | `handle_menu_button_click` | yes |
| `opts-game` | (etc.) | ... |

Every system marked "yes" gets the `not_modal_active` run condition.
The modal's own systems do not.

### Stage A4 — Screenshot Capture Integration

Describe:

1. Which Bevy screenshot API is used.
2. How the captured `Image` is passed to the modal.
3. How the modal preview displays it (aspect ratio, letterboxing).
4. What happens if capture fails (e.g., window resize during
   capture).

### Stage A5 — Pause Semantics

Confirm:

1. `Time<Virtual>` is the right pause mechanism.
2. All simulation systems use `Time<Virtual>` (verify by reading
   `opts-sim`).
3. The modal's own animations (if any) use `Time<Real>` so they
   continue while paused.
4. The `was_running_before` flag is tracked correctly.

### Stage A6 — `opts-gui` Extensions

If `Modal` and `TextArea` don't exist:

**Modal:**

- Component: `ModalRoot` (marker on the backdrop node).
- Component: `ModalContent` (marker on the card node).
- Plugin: `ModalPlugin` that registers modal-related systems.
- Behavior: full-screen overlay, backdrop click = cancel (optional,
  confirm with user), Escape = cancel.

**TextArea (multiline):**

- Component: `TextArea` (marker).
- Component: `TextAreaValue(String)` (the content).
- Behavior: accepts keyboard input, inserts characters at cursor,
  supports backspace, enter for newlines, arrow keys for cursor
  movement.
- Scrolls when content exceeds visible height.
- Full spec: this is a nontrivial widget. If the existing
  `TextField` can be extended to multiline, prefer that over a new
  widget.

Report what needs to be added versus what exists.

### Stage A7 — File Save Strategy

Confirm:

1. The reports directory is created at app startup.
2. Save is asynchronous (does not block the render loop).
3. Save failures are logged at `WARN` and shown as a UI toast if
   available.
4. If save fails, the modal does not close — the user can retry.

### Stage A8 — Checkpoint

Report the design.

**Wait for user approval before Part B.**

---

## 6. Part B — `opts-gui` Extensions (if needed)

Only if `Modal` and/or `TextArea` don't exist.

### Stage B1 — Modal Primitive

Implement `Modal` in `crates/opts-gui/src/components/modal.rs`:

- Full-screen backdrop node.
- Content card with theme-driven styling.
- Focus trap: Tab cycles within the modal only.
- Escape closes (emits an event).
- Click on backdrop = configurable (default: no-op, since closing
  by backdrop click is destructive for a report).

### Stage B2 — Multiline TextArea

Implement `TextArea` in
`crates/opts-gui/src/components/text_area.rs`:

- Multiline text input.
- Cursor position tracking.
- Backspace, delete, enter, arrow keys, home/end.
- Ctrl+A, Ctrl+C, Ctrl+V (basic clipboard if `arboard` is
  available).
- Scrolling when content exceeds visible area.
- Line wrapping.
- Theme-driven colors.

This is a substantial widget. Budget effort accordingly.

### Stage B3 — Verify

- Unit tests for the TextArea's text mutation logic.
- Doctests for both new components.
- Example binary demonstrating the modal and text area.

### Stage B4 — Checkpoint

Report the extensions.

**Wait for user approval before Part C.**

---

## 7. Part C — Feature Implementation

### Stage C1 — Module Tree

Create `crates/opts-bevy/src/reporting/`:

```
reporting/
├── mod.rs                 # Public API + ReportingPlugin
├── state.rs               # ReportModalState, ReportModalData
├── trigger.rs             # F1 detection
├── capture.rs             # Screenshot capture
├── modal.rs               # Modal spawning and layout
├── save.rs                # File saving
├── pause.rs               # Pause/resume logic
└── format.rs              # .report file formatting
```

### Stage C2 — Trigger System

`trigger.rs`:

- Watch for the resolved F1 key press.
- On press, transition state to `Capturing`.
- Record `was_running_before = !Time<Virtual>.is_paused()`.
- Spawn the screenshot capture.

### Stage C3 — Screenshot Capture

`capture.rs`:

- Use the Bevy screenshot API confirmed in Stage A4.
- Store the resulting `Image` handle in `ReportModalData`.
- Transition to `Open` when ready.
- Handle capture failure with a logged error and return to
  `Inactive`.

### Stage C4 — Modal UI

`modal.rs`:

- Spawn the modal UI per the layout in Section 4.2.
- Bind `ReportModalData.text` to the TextArea.
- Handle button clicks and Escape.
- Clean up modal entities on close.

### Stage C5 — Pause Integration

`pause.rs`:

- On entering `Capturing` or `Open`, pause `Time<Virtual>` if it
  wasn't already paused.
- On entering `Inactive`, unpause if `was_running_before` was true.

### Stage C6 — Save Logic

`save.rs`:

- Write the PNG.
- Write the `.report` text via `format.rs`.
- Handle collisions by appending `-1`, `-2`, etc.
- Log success at `INFO`, failure at `WARN`.

### Stage C7 — Format Module

`format.rs`:

- Build the `.report` string from the metadata template.
- Timestamp in RFC 3339 with local offset.
- Report text written verbatim.

### Stage C8 — Input Gating

- Register the `not_modal_active` run condition.
- Apply it to every system marked "yes" in Stage A3.
- Verify no system was missed.

### Stage C9 — Wiring

In `opts-game`:

- Add `ReportingPlugin` to the app.
- Ensure it's added before input-handling plugins so the run
  conditions register correctly.

### Stage C10 — Verify

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

All must pass.

### Stage C11 — Checkpoint

Report the implementation.

**Wait for user approval before Part D.**

---

## 8. Part D — Verification

### Stage D1 — Manual Flow

Run the app. Verify:

- [ ] F1 opens the modal from the main menu.
- [ ] F1 opens the modal from in-simulation.
- [ ] The screenshot shows the app state before the modal opened.
- [ ] The text area is focused and accepts typing.
- [ ] Enter inserts a newline.
- [ ] Backspace, arrows, home/end work.
- [ ] Escape cancels the modal.
- [ ] Clicking Cancel closes and resumes.
- [ ] Clicking Report saves two files and resumes.
- [ ] Simulation is paused while the modal is open.
- [ ] Simulation does not advance during the modal.
- [ ] Simulation resumes on close.
- [ ] If the sim was already paused, it stays paused.
- [ ] No input leaks to the app (e.g., W doesn't move the free-roam
      camera while typing).
- [ ] No input leaks to the app (e.g., clicking the modal doesn't
      trigger underlying buttons).
- [ ] The `.report` file contains the description text.
- [ ] The `.report` file contains correct metadata.
- [ ] The `.png` file matches the screenshot.
- [ ] The filenames use the timestamp format.

### Stage D2 — File Inspection

After generating several reports, verify:

- [ ] All files are in the reports directory.
- [ ] Filenames sort lexically by time.
- [ ] PNGs open correctly.
- [ ] `.report` files are UTF-8 with `\n` line endings.
- [ ] Special characters (umlauts, emoji) survive the round trip.

### Stage D3 — Automated Tests

In `crates/opts-bevy/tests/reporting.rs`:

- Test the `.report` formatting function with a fixed timestamp.
- Test filename collision handling.
- Test the pause/resume logic with a simulated "was running"
  state.
- Test that modal state transitions correctly across all paths.

### Stage D4 — Documentation

- `docs/src/architecture/reporting-feature.md` — promoted from draft
  to accepted.
- `docs/src/formats/text/report-format.md` — the `.report` spec.
- `docs/crates/opts-bevy.md` — updated with the reporting module.
- `docs/crates/opts-gui.md` — updated with Modal and TextArea.
- `docs/src/SUMMARY.md` — add the new docs.
- Update the camera system doc if F1 was reassigned.

### Stage D5 — Final Report

Produce `docs/src/meta/f1-reporting-YYYY-MM-DD.md`:

- Which option was chosen for the F1 conflict
- What was built
- Which subsystems were touched
- Screenshots of the modal
- Sample `.report` file
- Any deviations from the design
- Any follow-up items

---

## 9. Reporting Cadence

1. Preconditions — approval before Section 0 resolution
2. Section 0 resolution — approval before Part A
3. Part A (design) — approval before Part B
4. Part B (`opts-gui` extensions, if needed) — approval before Part C
5. Part C (feature) — approval before Part D
6. Part D (verification) — end of task

Format:

```
## F1 Reporting — Checkpoint N

**Phase:** <name>
**Deliverables:** <summary>
**Blockers:** <if any>

Ready to proceed? (yes / adjust)
```

---

## 10. Exit Criteria

- ✅ F1 conflict resolved (Section 0).
- ✅ Modal opens from any application state.
- ✅ Screenshot captured and displayed.
- ✅ TextArea accepts multiline input.
- ✅ Report and Cancel buttons work.
- ✅ Escape cancels.
- ✅ Input gating blocks all non-modal systems.
- ✅ Simulation pauses and resumes correctly.
- ✅ Files saved with correct naming and content.
- ✅ `.report` format documented.
- ✅ Modal and TextArea added to `opts-gui` (if needed).
- ✅ `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` pass.
- ✅ Integration tests in `crates/opts-bevy/tests/reporting.rs`.
- ✅ Every file under 500 lines.
- ✅ Every public item documented.
- ✅ No `println!`.
- ✅ No `unsafe`.

When finished, respond with the summary from Stage D5.

Then respond with the exact string:
"F1 REPORTING COMPLETE — ready for user review"