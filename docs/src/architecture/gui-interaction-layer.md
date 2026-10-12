# GUI Interaction Layer and Components (M-0061 part B)

| Metadata | Value |
| :--- | :--- |
| **Status** | Implemented on `agent/ui-widgets` (2026-10-12); supersedes the component sections of [`gui-design-system.md`](gui-design-system.md) where they differ |
| **Crate** | [`opts-gui`](../../crates/opts-gui.md) |
| **Specs** | [interaction and feel](../roadmap/phase-9/interaction-and-feel.md), [expandability](../roadmap/phase-9/expandability.md), [UI spikes](ui-spikes.md) |

Tokens and themes (colours, type, spacing, motion) belong to the design system and part A of M-0061. This page describes everything that makes a control *behave*: one state mechanism, one activation path, one focus model, one action layer, and the test hooks.

## 1. Anatomy of an interactive control

A control is an entity with the [`Button`](../../crates/opts-gui.md) marker (every widget root has it, composite parts such as tab headers, tree rows and combobox items too). The marker *requires*:

| Component | Meaning |
| :--- | :--- |
| `Node`, `FocusPolicy::Block` | layout and pointer blocking |
| `bevy::picking::hover::Hovered` | pointer is over the control or a descendant (Bevy keeps it current) |
| `Focusable` (and Bevy's `TabIndex`) | can take keyboard focus |
| `StateLayered` | gets the state layer, scrim and focus ring below |

Optional, added by the owner: `UiId` (explicit stable id), `Tooltip`, `DisabledReason`, `NotAvailableYet`, `ErrorState`, `Selected`, `DefaultFocus`, `PointerOnly`, `FocusRegion`.

## 2. States (one mechanism)

`interaction::layer` builds, for every `StateLayered` entity, two helper children and one outline; part A's token sync keeps writing `BackgroundColor`/`BorderColor` of the control and never meets them.

| State | Visual | Source |
| :--- | :--- | :--- |
| hover | content colour at 8 % in the *state layer* (first child) | `Hovered` |
| focus | 10 % layer and a 2 px `Secondary` ring (`Outline`, offset 2 px) while the ring is visible | `Focused` + `InputFocusVisible` |
| pressed | 10 % layer, no scale jump | `Pressed` |
| disabled | `Surface` scrim at 62 % above the content (38 % content remains; the container fades with it - a simplification of M3's 12 % container); no hover/pressed layer | `Disabled` |
| error | border role swapped to `Error`, restored when `ErrorState` goes | `ErrorState` |
| selected | owned by the widget (container colour plus marker), reported as `Selected` | widget |

The layer colour is the content colour of the control's `BackgroundRole` (`on_color`) or `StateLayerRole`. `ForcedState` pins hover/focus/pressed for the gallery and screenshots only.

**A disabled control always says why**: `DisabledReason` (tooltip on hover and keyboard focus) or the [not-yet marker](../reference/not-available-yet.md). The CI rule `disabled_without_reason` fails otherwise.

## 3. Activation

`Activated(Entity)` is the only way a control is *used*. Producers: pointer **release inside** the control (observers on `PointerPress`/`PointerRelease`; a press that ends elsewhere is cancelled when no pointer button is held any more), `UiAction::Confirm` on the focused control, and the control protocol's `ui.click`. Widgets (`checkbox_toggle_system`, `notebook_tab_click_system`, ...) read `Activated`, never pointer state; the old `Interaction` component and the crate-wide `#![allow(deprecated)]` are gone.

## 4. Focus

* `InputFocus` (Bevy) is the truth; the `Focused` marker mirrors it and either side may write (`sync_input_focus`). Move focus with `FocusEvent::{Next, Previous, Set, Clear}` or actions.
* Candidates (`FocusGraph::candidates`): `Focusable`, visible (no `Display::None`/`Visibility::Hidden` on it or an ancestor), enabled or disabled-with-reason, not `PointerOnly`, and inside the topmost modal while one is open. Order is the layout tree; arrows use the node rectangles of the current region; without layout they follow tree order.
* Regions: `FocusRegion(Landmark)`; Tab jumps regions, arrows stay inside one; fewer than two regions makes Tab a plain next/previous.
* Default focus: `DefaultFocus` on the safest control (main menu: *Start New Game*, never *Shutdown*). Dialogs move focus in and restore it on close.
* Arrow ownership: tree rows (left/right), combobox headers (up/down) and text entries (caret) keep their arrows.

## 5. Actions

`UiAction` (keyboard and gamepad tables in `actions.rs`, documented in [interaction and feel §1](../roadmap/phase-9/interaction-and-feel.md#1-devices-and-the-action-layer)). The library handles `Confirm`, `Back` (close combobox, dismiss toast, drop text focus), `Search`, `NextTab`/`PrevTab` and the moves; screens read the rest from the same message.

## 6. Text input

`TextField` is a frame around Bevy's `TextInput`/`EditableText` (platform layout, IME, caret, selection, clipboard) with placeholder, clear button, `TextChanged`, error/disabled treatment and a two-way `M3TextField::text`. `TextArea` keeps its own typing. Verdicts and what is not verified: [UI spikes](ui-spikes.md#1-text-input-ime-paste-caret).

## 7. Ids, tree and protocol

`UiId` (`menu.start-new-game` or automatic `parent/role:slug`), `ui_tree::snapshot`, accessibility nodes, and `ui.tree`/`ui.click`/`ui.focus`/`ui.action`/`ui.state`: [control protocol §4.9](control-protocol.md#49-ui-by-id).

## 8. Not available yet and localisation

`not_yet::{NotAvailableYet, Availability, PLACES}`: [list](../reference/not-available-yet.md). `l10n::{Localization, L10nKey, L10nArgs, keys}`: Fluent-subset tables `en.ftl`/`de.ftl`, fallback locale → language → English, pseudo-locale `en-XA` (`OPTS_LOCALE=en-XA`), live switching; the upgrade path to the real Fluent crates is in `l10n/fluent.rs`.

## 9. Tests and rules

`opts_gui::testing` (`headless_app`, `layout_app` with Bevy's real UI layout, synthetic pointer events, `act`) and `testing::rules` (`small_targets`, `small_text`, `disabled_without_reason`, `keyboard_traversal`, `hardcoded_colours`) run on `opts_gui::showcase` (every component in every state, also the content of the `gallery` example): `crates/opts-gui/tests/ci_rules.rs`. The main menu is held to the same rules in `crates/opts-game/tests/ui_rules.rs`; the other screens still have 11 px texts and targets under 32 px (recorded in [the progress file](../../dev/progress/ui-widgets.md)) and join when the shell milestone rebuilds them. A test forbids user-visible string literals in `opts-gui` outside the key tables.

## 10. Adding a widget

1. Spawn the root with `Button` (or `StateLayered` + `FocusProxy` for a composite frame) - states, focus and `Activated` come for free.
2. React to `Activated`, never to pointer components; send your semantic message.
3. Give interactive parts a role in `ui_tree::role_of` and decorative text `A11yHidden`.
4. Text by `L10nKey`, colours by roles, sizes ≥ 12 px and targets ≥ 32 px; add the widget to `showcase.rs` and run `ci_rules`.
