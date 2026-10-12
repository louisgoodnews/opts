# UI Foundations Spikes (M-0061 Phase A)

| Metadata | Value |
| :--- | :--- |
| **Status** | Verdicts recorded 2026-10-12 (batch B0 of `agent/ui-widgets`); amended per spike when the implementing batch has run |
| **Scope** | Part B of [M-0061](../roadmap/phase-9/M-0061.md): text input, accessibility tree, virtualised list, pointer events |
| **Method** | Bevy 0.20.0 sources in the local cargo registry were read (`bevy_ui`, `bevy_ui_widgets`, `bevy_input_focus`, `bevy_text`, `bevy_a11y`, `bevy_picking`) and the current `opts-gui` code was compared with them. "Tried" says what was compiled or run; nothing was assumed about screen readers or IME on a real desktop (not testable in the container). |

The typeface spike (D-UI-03) belongs to part A (`agent/ui-theme`).

## 1. Text input (IME, paste, caret)

**What Bevy 0.20 offers.** `bevy_text::EditableText` (a `parley` plain editor: caret, selection, word/line movement, undo-less edits queued in `pending_edits` as `TextEdit` values, IME preedit via `PreeditCursor`/`TextEdit::clear_ime_compose`) and the headless widget `bevy_ui_widgets::TextInput` (requires `EditableText` and an `AccessibilityNode` with role `TextInput`). `TextInputPlugin` is part of `UiWidgetsPlugins`, which `DefaultPlugins` adds with the default `ui` feature, so no new crate and no feature flag is needed. The widget consumes `FocusedInput<KeyboardInput>` (dispatched by `InputDispatchPlugin` to the entity held in the `InputFocus` resource), maps `Ctrl+C/V/X/A`, `Key::Copy/Paste` to `TextEdit::Copy/Paste` (clipboard through `bevy_clipboard`, added by `TextPlugin`), handles pointer press and drag selection and the `Ime` window events, and keeps the caret in view. Characters come from the logical key of the keyboard event, that is **the platform layout**, which is what removes the Linux layout detector (audit F11).

**What the existing code does.** `TextArea` (multi-line) types through its own `KeyboardInput` reader and a Linux-only layout detector; `M3TextField` is a visual shell with a `value` string and no keyboard handling of its own.

**Verdict.** Rebuild `TextField` on `TextInput`/`EditableText` and make `InputFocus` the single source of truth for focus (see the focus design in [`gui-design-system.md`](gui-design-system.md#9-interaction-layer)); the library's `Focused` marker mirrors `InputFocus` so existing code keeps working. `TextArea` keeps its own typing until a multi-line `EditableText` variant is worth the migration; its layout detector stays and is **removed from the single-line search fields only** (B3). Clear button, change message and validation state are ours; caret, selection, IME and clipboard are Bevy's. Risk: the widget is new in 0.20; screen-reader and IME behaviour on the user's desktop is unverified (recorded as an open problem, not assumed to work).

## 2. Accessibility tree

**What Bevy 0.20 offers.** `bevy_a11y::AccessibilityNode(accesskit::Node)` is a component; `bevy_ui`'s accessibility plugin keeps `bounds` and `transform` of every such node in sync with the layout and derives a *label* from child text for its (deprecated) `Button`, `ImageNode` and `Label`. `bevy_winit` forwards the tree to the platform through AccessKit (`accesskit_unix` feature on Linux, enabled by default features). Headless widgets of `bevy_ui_widgets` set roles (`Button`, `CheckBox`, `RadioButton`, `ListBox`, `TextInput`, `Tab`, `Slider`) and states (`InteractionDisabled`, `Checked`, `Selected`, expanded) on their own `AccessibilityNode`. Every node can be given any `accesskit::Role` and a label; nothing else is required.

**Tried.** Source reading plus the `UiId`/role table of batch B4: `opts-gui` gives every interactive element an `AccessibilityNode` with role and label derived from its `UiRole` and its text, and the control protocol's `ui.tree` reads the same data, so the tree is covered by tests that need no window. **Not verified:** that Orca (or any other screen reader) announces the tree; that needs a desktop session.

**Verdict.** Use `AccessibilityNode` directly, written from one place in `opts-gui` (`ui_id.rs`: role + label + state) so widgets cannot drift; reuse Bevy's markers (`InteractionDisabled`, `Checked`, `Selected`) as the *state vocabulary* for the a11y flags. Real screen-reader validation moves to the accessibility milestone of Phase 9.

## 3. Virtualised list

**What Bevy 0.20 offers.** `ListBox`/`ListItem` (`bevy_ui_widgets::list`): selection, `ActiveDescendant` keyboard navigation and a11y roles, but **no virtualisation**: every row is an entity. `bevy_feathers` has no virtual list either (its `virtual_keyboard` is an on-screen keyboard). `ScrollArea` provides scrolling and `ScrollIntoView`.

**Tried.** The windowing arithmetic (fixed row height, overscan, visible range from scroll offset and viewport) as a pure, tested function in `opts-gui` (`components/virtual_list.rs`), independent of any entity.

**Verdict.** Adopt a *fixed-row-height windowing* model: the list owns an item count and a row height, spawns only rows in `visible_range(...)` (+ overscan) and keeps one spacer for the total height; rows are `ListItem`-like with arrow/Home/End/PageUp/PageDown handled by the action layer on the *list* (one focusable, `ActiveDescendant`-style), type-ahead against the model not the entities. Only the pure windowing core is built in this milestone; the entity-spawning widget arrives with the first screen that needs it (vehicle/map lists in M-0063) because the model/provider interface is defined by that screen. Variable-height rows are out of scope.

## 4. Pointer events and the deprecated `Interaction`

**What Bevy 0.20 offers.** `bevy::ui::Interaction`, `bevy::ui::Button` and `FocusPolicy`-driven `ui_focus_system` are deprecated ("Use `picking::hover::Hovered` and `ui::Pressed`"). Replacements: the picking pipeline (`UiPickingPlugin`, in `DefaultPlugins`) with `bevy::picking::hover::Hovered(bool)` (immutable component, true while the pointer is over the entity *or a descendant*) and the bubbling pointer events `Pointer<Press>`, `Pointer<Release>`, `Pointer<Click>`, `Pointer<DragEnd>`; `ui::Pressed` is a marker for "held down", set by the headless widgets.

**Tried.** Event semantics read in `bevy_picking::events`: `Click` is emitted only on the entity that received both `Press` and `Release`, so a press on a child label and a release on the button's padding yields no click on either; `Press`/`Release` bubble through `ChildOf`.

**Verdict.** One small system pair in `opts-gui`, no per-widget pointer code:

1. Every interactive control carries `Interactive` (requires `Node`, `Hovered`, `FocusPolicy::Block`, `UiState`).
2. Global observers on `Pointer<Press>` / `Pointer<Release>` run at each bubbled entity: `Press` inserts `Pressed`; `Release` over a still-pressed control sends **`Activated(entity)`** (activation on release inside the control; descendants count as inside).
3. A cleanup system removes `Pressed` whenever no pointer button is held, which is the *drag-away cancel* (audit F19).
4. Keyboard `Confirm`, gamepad A and the control protocol's `ui.click` send the same `Activated` message, so widget logic exists once.

The crate-wide `#![allow(deprecated)]` is removed with this batch; the library's own `Button` marker no longer requires `Interaction`.
