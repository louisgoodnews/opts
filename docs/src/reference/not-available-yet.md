<!-- Generated from `crates/opts-gui/src/not_yet.rs` by the test `reference_page_is_up_to_date`; regenerate with `UPDATE_NOT_YET=1 cargo test -p opts-gui not_yet`. -->

# Features Marked "Feature not available yet!"

Decision D-UI-09 ([phase 9](../roadmap/phase-9/README.md#decisions)): a feature that does not work yet stays visible and carries the red text *Feature not available yet!*. Every place is registered in `opts_gui::not_yet::PLACES`; a test fails when a place waits for a milestone that is already complete, so this list can only shrink by shipping the feature.

| Place | Id | Milestone | What is missing |
| :--- | :--- | :--- | :--- |
| Navigation rail, Library | `nav.library` | M-0062 | The Library needs the master-detail pattern of the Plan a drive wizard and its detail panels. |
| Navigation rail, Workshop | `nav.workshop` | M-0051 | Editor projects, installed mods and packaging need the editor and mod system. |
| Help, Guides | `help.guides` | M-0064 | The short guides are written with the HUD and help overlay of the HUD milestone. |
| Plan a drive, When, Live weather | `wizard.live-weather` | M-0062 | The weather query is never called; live conditions need an explicit, explained opt-in. |
| Settings, Gameplay, Time scale | `settings.time-scale` | M-0064 | The session clock cannot be sped up or slowed down from the interface yet. |
| Settings, Controls, Change a binding | `settings.rebind` | M-0035 | The keys of the vehicle systems (engine, lights, doors, ...) can be changed above; driving and camera keys, gamepad buttons and wheel controls are fixed in code. |
