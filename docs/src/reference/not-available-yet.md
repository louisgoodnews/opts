<!-- Generated from `crates/opts-gui/src/not_yet.rs` by the test `reference_page_is_up_to_date`; regenerate with `UPDATE_NOT_YET=1 cargo test -p opts-gui not_yet`. -->

# Features Marked "Feature not available yet!"

Decision D-UI-09 ([phase 9](../roadmap/phase-9/README.md#decisions)): a feature that does not work yet stays visible and carries the red text *Feature not available yet!*. Every place is registered in `opts_gui::not_yet::PLACES`; a test fails when a place waits for a milestone that is already complete, so this list can only shrink by shipping the feature.

| Place | Id | Milestone | What is missing |
| :--- | :--- | :--- | :--- |
| Main menu, Load Last Game | `menu.load-last-game` | M-0062 | There is no save system; the shell offers Continue for the last drive. |
| Load Saved Game screen | `screen.load-game` | M-0062 | Loading a saved session needs the save system. |
| Plan a drive, When, Live weather | `wizard.live-weather` | M-0062 | The weather query is never called; live conditions need an explicit, explained opt-in. |
| Settings, Audio, volumes | `settings.audio` | M-0027 | There is no audio engine; the volume settings appear with it. |
| Settings, Graphics, View distance | `settings.view-distance` | M-0029 | Tiles and objects are loaded at a fixed distance; a user setting needs the LOD and culling system. |
| Settings, Gameplay, Time scale | `settings.time-scale` | M-0064 | The session clock cannot be sped up or slowed down from the interface yet. |
| Settings, Controls, Change a binding | `settings.rebind` | M-0035 | Vehicle and camera keys are fixed in code; the list shows them, rebinding needs the vehicle control layer. |
