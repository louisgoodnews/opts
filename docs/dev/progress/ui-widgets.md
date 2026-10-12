# Progress: M-0061 part B (UI widgets, focus, actions, ids, localisation)

Branch `agent/ui-widgets`, worktree `/home/louisgoodnews/Dokumente/opts-wt-ui-widgets` (from `agent/integration`).
Parallel: part A on `agent/ui-theme` (theme/tokens/fonts) - do not touch `crates/opts-gui/src/theme/**`, `tokens/**`, fonts.
Bulky data: `~/.local/share/opts/benchmark/dev-ui-impl/widgets/`.

## Resume

```sh
cd /home/louisgoodnews/Dokumente/opts-wt-ui-widgets      # exists = you are a replacement agent; read this file
git status --short; git log --oneline agent/integration..HEAD
cargo check --workspace --all-targets --all-features      # own target/, cold build is slow
```

Gates: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`; `python3 scripts/check_links.py`; `python3 scripts/gen_roadmap_copy.py --check`.

Runs of the game/tools: `XDG_CONFIG_HOME=$(mktemp -d) XDG_DATA_HOME=$(mktemp -d) XDG_CACHE_HOME=~/.local/share/opts/benchmark/dev-textures/cache` (copy of the cache).

## Batches

| Batch | Status |
| :--- | :--- |
| B0 Orient + spikes | done (`docs/src/architecture/ui-spikes.md`) |
| B1 State mechanism, release activation, tooltips | done |
| B2 Focus and action layer | done |
| B3 TextField | done |
| B4 UI ids and `ui.*` protocol | done |
| B5 NotAvailableYet + registry | todo |
| B6 Localisation scaffold | todo |
| B7 CI rules, gallery, docs | todo |

## Design decisions (all batches)

- `InputFocus` (bevy_input_focus) is the single focus truth; `Focused` marker mirrors it. Own visual-order navigation (Bevy's TabNavigation is hierarchy/TabIndex based; not used).
- One `Activated(Entity)` message for pointer release, Confirm, gamepad A, `ui.click`; widgets react to it, not to pointer state.
- State layers are a child overlay node (no conflict with the token sync of part A that writes `BackgroundColor`); focus ring via `Outline`.
- No Fluent crates offline (`~/.cargo/registry` has none): own Fluent-subset parser behind the `Localization` API.

## Needs from part A / open problems

(none yet)

## Results

(see per batch below)

### B0
Spike verdicts in `docs/src/architecture/ui-spikes.md`. Bevy 0.20 has `EditableText`/`TextInput` (IME, clipboard, selection), `AccessibilityNode`, `ListBox` (no virtualisation), `Hovered`/`Pressed` + picking events. Virtual list: pure windowing core `components/virtual_list.rs` (tested).

### B1
- New: `interaction/activation.rs` (`Activated` message, press/release observers on `PointerPress`/`PointerRelease`, stale-press cancel, Enter/Space confirm, `ActivationSet`), `interaction/layer.rs` (state layer child + disabled scrim child + `Outline` focus ring + error border swap; `on_color`), `interaction/tooltip.rs` (`Tooltip`, 600 ms hover / immediate keyboard focus, `DisabledReason` text), `testing.rs` (`headless_app`, `press`, `release`, `click`).
- `interaction/state.rs`: `Hovered`/`Pressed` are now Bevy's; added `DisabledReason`, `ErrorState`, `UiState`, M3 constants.
- All widgets react to `Activated` instead of `Interaction`; crate-wide `#![allow(deprecated)]` removed from opts-gui and opts-game; `opts-game/src/menu/interactions.rs` deleted (hover/focus/Enter now in the library).
- Test note: `world.trigger` outside a schedule needs `world.flush()` for observer commands (in `testing.rs`).
- Disabled treatment: scrim of `Surface` at 62 % (content 38 %); container fades by the same factor (12 % container of M3 is a documented simplification).
- Not yet verified visually (screenshots in B7).
- Gates: fmt, clippy, test --workspace, doc, links, roadmap copy all green at the B1 commit.

### B2
- `actions.rs`: `UiAction`, `Direction`, `ScrollStep`, `key_action`/`button_action` tables, `keyboard_actions`/`gamepad_actions` systems, `UiAction::parse` (protocol spelling). Documented in `interaction-and-feel.md` section 1.
- `interaction/navigation.rs`: `FocusGraph` (candidates: visible, enabled-or-with-reason, modal scope, layout-tree order, rects), `next_linear`, `next_region`, `pick_directional`, `move_focus` (pure, unit-tested).
- `interaction/focus.rs` rewritten: `InputFocus` <-> `Focused` reconciliation, `FocusEvent` handler (also consumes focus-moving actions), `FocusRegion`/`Landmark`, `DefaultFocus`, `PointerOnly` (HUD exit button), modal focus restore, arrow-owner rules (tree/combobox/text). `Focusable` now `#[require(TabIndex)]`; every `Button` is focusable; `tab_index` field unused.
- `interaction/handlers.rs`: `Back` (combobox close, toast dismiss, text entry drops focus), `Search`.
- Widgets: notebook next/prev tab + focus selects, tree Left/Right, combobox Up/Down open + focus item + restore, toast region + keyboard dismiss, modal trap replaced by `FocusGraph` scope.
- `escape_to_menu_system` of the new game screen ignores Esc while a text entry has focus.
- Tests: `tests/keyboard_operability.rs`, focus unit tests, navigation unit tests.
- Deviation: tab order = layout-tree order, not geometric reading order (documented in interaction-and-feel 2a).
### B3
- `TextField` on `TextInput`/`EditableText`, new-game search bars migrated, `handle_search_typing` and friends deleted. `keyboard_layout.rs` kept for label hints.
- Headless typing is simulated with queued `TextEdit`s (no fonts, no key dispatch); real key dispatch and IME are NOT verified here.

### B4
- `ui_id.rs`: `UiId`, `slug`, automatic ids (`parent/role:slug`, `-2` suffix, ordered by layout path), a11y nodes (`accesskit` 0.24 direct dependency, same version as `bevy_a11y`) written for every element with an id.
- `ui_tree.rs`: `UiRole`, `role_of`, `label_of`, `UiNodeState`, `UiNode`, `UiSnapshot`, `snapshot(world)`, `find_entity`; `A11yHidden`, `ListRow` markers.
- `opts-bevy/src/control/handlers/ui.rs`: `ui.tree`, `ui.click`, `ui.focus`, `ui.action`, `ui.state`; `opts-cli` command table (54 methods); `control-protocol.md` section 4.9.
- Tests: `opts-gui/tests/ui_tree.rs`, `opts-bevy/tests/control_ui.rs` (real socket, headless app).
- `l10n/mod.rs` is a stub (`L10nKey`, `key_of`) until B6.
