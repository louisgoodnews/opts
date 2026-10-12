<!-- Generated from `crates/opts-gui/src/not_yet.rs` by the test `reference_page_is_up_to_date`; regenerate with `UPDATE_NOT_YET=1 cargo test -p opts-gui not_yet`. -->

# Features Marked "Feature not available yet!"

Decision D-UI-09 ([phase 9](../roadmap/phase-9/README.md#decisions)): a feature that does not work yet stays visible and carries the red text *Feature not available yet!*. Every place is registered in `opts_gui::not_yet::PLACES`; a test fails when a place waits for a milestone that is already complete, so this list can only shrink by shipping the feature.

| Place | Id | Milestone | What is missing |
| :--- | :--- | :--- | :--- |
| Home, Load game | `home.load-game` | M-0062 | There is no save system; Home offers Continue for the last drive. |
| Navigation rail, Library | `nav.library` | M-0062 | The Library needs the master-detail pattern of the Plan a drive wizard and its detail panels. |
| Navigation rail, Workshop | `nav.workshop` | M-0051 | Editor projects, installed mods and packaging need the editor and mod system. |
| New game, Live Device Location, Refresh | `newgame.live-weather-refresh` | M-0062 | The location query is never called; the panel shows placeholder values. |
