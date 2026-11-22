# ISS-055: Key chips of different width push the Help table's descriptions out of line

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-055. Owner report 2026-11-22, with screenshots (dark theme, Hilfe > Steuerung). Category QoL |
| **Severity** | minor |
| **Milestone** | [M-0174](../../src/roadmap/phase-18/M-0174.md) UI alignment and spacing consistency (Help and Settings). Placed in the Phase 18 research backlog as a QoL item until the owner names a better phase |
| **Status** | milestone M-0174 proposed (owner report 2026-11-22; not approved to start) |

## Problem

In the Help screen's controls table, each row is a key chip followed by its description. The chips have different widths, so the description column starts at a different x position from row to row. For example, `Ctrl+NumpadDivide` and `Ctrl+NumpadMultiply` push their descriptions to the right. The owner also finds that buttons and other UI elements of different size look untidy and asks for a general improvement.

The code gives the chips a minimum width only, so a chip grows with its text:

- `crates/opts-game/src/help/controls/spawn.rs`, `binding_row` (line 49): the chip of the fixed groups has `min_width: Val::Px(120.0)` and no fixed width. The description follows in a row with a 16 px gap.
- `crates/opts-game/src/help/vehicle_controls.rs` (line 72) calls `chip(row, &shown, 120.0)` for each vehicle action. The chip is built in `crates/opts-game/src/help/vehicle_controls/widgets.rs` (lines 28 to 32), again with `min_width` only. The key text comes from the player's bindings, so its width varies with the key name.

A chip wider than 120 px therefore moves its row's description. The rebindable key text (`ActionChip`, `crates/opts-game/src/help/controls.rs`) changes the width at run time as well.

## Scope limit

This is a layout issue. It changes no key, no action and no binding. The Settings screen was not inspected for this item; its key rows are to be checked when the fix is made.

## Fix direction

Give the key column one sizing rule shared by all Help rows: a fixed chip width sized to the longest default chip, or a column of its own in a grid, so that every description starts at the same x position. The same rule should be applied to the other fixed-size controls the owner named, where it reads better (for example, the Settings rows).

## Done when

- The description column of the Help table starts at the same x position in every row, for the default key names, at the 1920x1080 window size.
- The check is recorded with a dated screenshot path in this issue. Screenshots stay out of the repository.
- The Settings screen is checked and either shares the rule or the reason it does not is recorded.

## Status of the checks

- Screenshot path: not yet recorded. The owner's 2026-11-22 screenshots are not in the repository, and this change ran no game.
- Fix: not started (no code in this change).
