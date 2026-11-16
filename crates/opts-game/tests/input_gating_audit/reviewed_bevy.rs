//! Reviewed input-reading files in `opts-bevy` (paths relative to `crates/`).

/// Files whose systems read input and were reviewed.
pub(super) const REVIEWED: &[&str] = &[
    // The registry of key actions only answers `just_pressed(action)`; the systems that ask are
    // gated where they are registered.
    "opts-bevy/src/action_keys.rs",
    // The rail keys are applied by the vehicle input system (`apply_keys`), which carries the
    // run conditions `vehicle_accepts_input` and `not_modal_active`.
    "opts-bevy/src/rail_controls.rs",
    // The mouse driving mode reads the wheel and right button; its systems are registered with the
    // vehicle input system and share its run conditions (`vehicle_accepts_input`, `not_modal_active`).
    "opts-bevy/src/mouse_driving.rs",
    // Its tests press the mouse buttons on purpose.
    "opts-bevy/src/mouse_driving/tests.rs",
    "opts-bevy/src/camera/input.rs",
    "opts-bevy/src/camera/mode/ego.rs",
    "opts-bevy/src/camera/mode/free_roam.rs",
    "opts-bevy/src/camera/mode/vehicle_views.rs",
    // Tab opens the viewpoint editor of the driver's view; gated in the camera plugin.
    "opts-bevy/src/camera/viewpoint.rs",
    // Tab opens the virtual mirrors of the driver's view; gated with `not_modal_active` in
    // `MirrorPlugin`.
    "opts-bevy/src/mirrors/virtual_view.rs",
    // The cab switch key: gated with `not_modal_active` where it is registered (`vehicle_sim`).
    "opts-bevy/src/cab_switch.rs",
    // F4 cycles the door camera; gated with `not_modal_active` where it is registered.
    "opts-bevy/src/door_camera.rs",
    "opts-bevy/src/reporting/trigger.rs",
    "opts-bevy/src/vehicle_sim.rs",
    // Split out of `vehicle_sim.rs`: its systems are registered in `vehicle_sim/plugin.rs` with
    // `not_modal_active`.
    "opts-bevy/src/vehicle_sim/input.rs",
    "opts-bevy/src/vehicle_triggers.rs",
    // Left click on scenery hot-spots (M-0113); registered in `VehicleTriggerPlugin` with
    // `not_modal_active`.
    "opts-bevy/src/vehicle_triggers/scenery_click.rs",
    // Split out of `camera/viewpoint.rs`: the Tab key of the viewpoint editor, gated as its parent.
    "opts-bevy/src/camera/viewpoint/keys.rs",
    // Split out of `camera/mode/vehicle_views.rs`: the camera keys, gated as its parent.
    "opts-bevy/src/camera/mode/vehicle_views/input.rs",
    // Split out of `vehicle_triggers.rs`: the trigger keys, registered with `not_modal_active` in `VehicleTriggerPlugin`.
    "opts-bevy/src/vehicle_triggers/keys.rs",
    // Split out of `vehicle_triggers.rs`: the trigger clicks, registered with `not_modal_active` in `VehicleTriggerPlugin`.
    "opts-bevy/src/vehicle_triggers/mouse.rs",
    // Split out of `action_keys.rs`: the key layout tables only ask `just_pressed(action)`; the systems that ask are gated where registered.
    "opts-bevy/src/action_keys/layout.rs",
    // Split out of `mouse_driving/tests.rs`: its tests press the mouse buttons on purpose, same as their parent.
    "opts-bevy/src/mouse_driving/tests/wheel.rs",
];
