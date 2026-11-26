//! Assembles the HUD of a kind at a size, with the extras a screen asks for.

use bevy::prelude::*;
use opts_bevy::{CameraMode, CameraState};
use opts_game::hud::{build, HudProfiles, RaiseAlert, VehicleKind};
use opts_game::AppState;

use crate::layout::resize;
use super::setup::{hud_base, spawn_drive, Extras};

/// Raises the alert, camera label, toast and editor panel that `extras` asks for.
fn apply_extras(app: &mut App, extras: Extras) {
    if extras.alert {
        app.world_mut().write_message(RaiseAlert {
            id: "fixture",
            severity: opts_game::hud::AlertSeverity::Warning,
            key: opts_game::hud::keys::ALERT_BRAKE_MOVING,
            args: Vec::new(),
        });
    }
    if extras.camera_label {
        app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
        // The first sample is the start of the drive; the change after it is announced.
        app.update();
        app.update();
        app.world_mut().resource_mut::<CameraState>().mode = CameraMode::External;
    }
    if extras.toast {
        let mut commands = app.world_mut().commands();
        opts_gui::components::toast::ToastBuilder::new(
            "A fixture toast with a rather long message to show how it wraps",
        )
        .spawn(&mut commands);
    }
    if extras.editor {
        app.world_mut()
            .resource_mut::<opts_bevy::ViewpointState>()
            .editing = true;
    }
}

/// A HUD of `kind` in a window of `w` x `h` logical pixels at UI scale `scale`.
pub fn build_hud(w: u32, h: u32, scale: f32, kind: VehicleKind, extras: Extras) -> App {
    let mut app = hud_base(w, h);
    spawn_drive(&mut app);
    resize(&mut app, w, h);
    app.insert_resource(UiScale(scale));
    app.update();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..4 {
        app.update();
    }
    let profile = app
        .world()
        .resource::<HudProfiles>()
        .get(kind)
        .cloned()
        .expect("profile");
    let palette = *app
        .world()
        .resource::<opts_gui::theme::MaterialTheme>()
        .instrument(opts_gui::theme::InstrumentVariant::Day);
    {
        let mut commands = app.world_mut().commands();
        build::spawn_hud(&mut commands, &profile, &palette);
    }
    for _ in 0..6 {
        app.update();
    }
    apply_extras(&mut app, extras);
    for _ in 0..8 {
        app.update();
    }
    app
}
