//! Reviewed input-reading files in `opts-game` (paths relative to `crates/`).

/// Files whose systems read input and were reviewed.
pub(super) const REVIEWED: &[&str] = &[
    "opts-game/src/in_game/hud.rs",
    "opts-game/src/in_game/loading_pipeline.rs",
    "opts-game/src/in_game/mod.rs",
    "opts-game/src/in_game/omsi_world/overlay.rs",
    // Split out of `omsi_world/overlay.rs`: the panel shown with F10 is gated by the overlay.
    "opts-game/src/in_game/omsi_world/overlay/panel.rs",
    // F11 toggles the lane and agent-id drawing flags only. Nothing is drawn unless the F10 panel
    // is shown, and the toggle changes no simulation state or text field, so it needs no gate.
    "opts-game/src/in_game/omsi_world/traffic/lane_overlay.rs",
    // P pauses the simulation; it checks `ReportModalState` and `MenuModalActive` itself, so it
    // is ignored while a modal is open.
    "opts-game/src/in_game/pause_toggle.rs",
    "opts-game/src/help/mod.rs",
    "opts-game/src/help/params.rs",
    "opts-game/src/hud/tests.rs",
    // Test module (split out of `hud/tests.rs`): presses the keys to test the systems above.
    "opts-game/src/hud/tests/alerts.rs",
    // F7, Shift+F7, + and - drive the mini map settings; gated with `not_modal_active`.
    "opts-game/src/hud/minimap/mod.rs",
    // F6, Home, the mouse drag and the wheel drive the full-size map; while it is open it holds
    // `MenuModalActive` (through `FullMapState::holds_input`), so all gated input stands still.
    "opts-game/src/hud/fullmap/mod.rs",
    // Split out of `hud/fullmap/mod.rs`: the open key (F6) and the map input run under the same
    // rule as the map itself.
    "opts-game/src/hud/fullmap/open.rs",
    "opts-game/src/hud/fullmap/input.rs",
    "opts-game/src/pause/mod.rs",
    "opts-game/src/pause/params.rs",
    "opts-game/src/shell/footer.rs",
    "opts-game/src/new_game_screen/systems.rs",
    "opts-game/src/settings_screen/page.rs",
    // Split out of `settings_screen/page.rs`: the page is the modal.
    "opts-game/src/settings_screen/page/systems.rs",
    // `capture_rebind`: while a key binding waits for its key, the next press is the new key;
    // it acts only on the open settings page, which is itself the modal.
    "opts-game/src/settings_screen/input.rs",
    // Split out of `settings_screen/input.rs`: the rebind capture, same rule as its parent.
    "opts-game/src/settings_screen/input/rebind.rs",
    // Esc closes the overlay; it never coexists with the report dialog (opened from the pause panel).
    "opts-game/src/settings_screen/overlay.rs",
    // Tram route choice (Z/X/C) ignores keys while a modal is up; the menu toggle checks the modals itself.
    "opts-game/src/in_game/omsi_world/rail.rs",
    // Split out of `rail.rs`: the player's keys, same rule as the rail controls above.
    "opts-game/src/in_game/omsi_world/rail/player_step.rs",
    "opts-game/src/in_game/omsi_world/rail/player_track.rs",
    // The wheel and the page keys scroll the Help lists; Help is a modal itself and is open while
    // `ModalActive` is set, so the system cannot be gated on `not_modal_active`.
    "opts-game/src/help/scroll.rs",
    "opts-game/src/in_game/vehicle_menu.rs",
    // Split out of `vehicle_menu.rs`: the search keys and the route page read input under the
    // same rule (the open menu holds `MenuModalActive`, which gates driving and camera input).
    "opts-game/src/in_game/vehicle_menu/entries.rs",
    "opts-game/src/in_game/vehicle_menu/route_page.rs",
    // Split out of `vehicle_menu.rs`: the plugin wiring of the menu, same rule.
    "opts-game/src/in_game/vehicle_menu/plugin.rs",
    // The formation page of the vehicle menu: typing and the wheel act only while that page is
    // open, and the open menu holds `MenuModalActive`, which gates driving and camera input.
    "opts-game/src/in_game/formation_menu/mod.rs",
    "opts-game/src/in_game/formation_menu/actions.rs",
    // Split out of `formation_menu/actions.rs`, same rule.
    "opts-game/src/in_game/formation_menu/actions/lists.rs",
    // F8 switches the session vehicle; gated with `not_modal_active` where it is registered.
    "opts-game/src/session_vehicles/mod.rs",
    // Test module: presses the key to test the system above.
    "opts-game/src/session_vehicles/tests.rs",
    // F5 / F9 (quick save and load); gated with `not_modal_active` where they are registered.
    "opts-game/src/save/quick.rs",
];
