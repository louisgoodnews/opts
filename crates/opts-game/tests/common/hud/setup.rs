//! The HUD's headless app, its fixture drive and the extras a screen can show.

use std::time::Duration;

use bevy::prelude::*;
use opts_bevy::{CameraState, PlayerVehicle, VehicleController};
use opts_game::hud::minimap::FixtureRoads;
use opts_game::hud::HudPlugin;
use opts_game::in_game::omsi_world::WorldStats;
use opts_game::in_game::types::InGameSessionInfo;
use opts_game::session_vehicles::SessionVehicle;
use opts_game::settings::GameSettings;
use opts_game::shell::ShellPlugin;
use opts_game::AppState;
use opts_gui::testing::layout_base;
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::AccentPreset;

use crate::layout::preferences;

/// What is on screen besides the permanent HUD.
#[derive(Debug, Clone, Copy, Default)]
pub struct Extras {
    /// A warning in the alert line.
    pub alert: bool,
    /// A toast (at the bottom right).
    pub toast: bool,
    /// The camera label (a camera change).
    pub camera_label: bool,
    /// The viewpoint editor's panel.
    pub editor: bool,
}

/// The headless app with the HUD plugins and fixture resources, before any vehicle or layout.
pub(super) fn hud_base(w: u32, h: u32) -> App {
    opts_game::isolation::isolate_user_dirs();
    let mut app = layout_base();
    let mut settings = GameSettings::default();
    let base =
        std::env::var_os("XDG_DATA_HOME").map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let db = base.join(format!("opts-hud-{}-{w}-{h}", std::process::id()));
    let _ = std::fs::create_dir_all(&db);
    settings.db_path_override = Some(db.join("settings.db"));
    app.add_plugins(bevy::state::app::StatesPlugin)
        .insert_resource(settings)
        .insert_resource(preferences(ThemeKind::Dark, AccentPreset::default()))
        .init_state::<AppState>()
        .init_state::<opts_bevy::reporting::ReportModalState>()
        .init_resource::<opts_bevy::reporting::ReportModalData>()
        .init_resource::<CameraState>()
        .insert_resource(InGameSessionInfo {
            map_name: "Fixture Valley".into(),
            weather_preset: "Clear".into(),
            ..Default::default()
        })
        .init_resource::<WorldStats>()
        .insert_resource(FixtureRoads::grid())
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ))
        .add_plugins(ShellPlugin)
        .add_plugins(HudPlugin);
    app
}

/// A drive with two session vehicles: the active one and another one 60 m away.
pub(super) fn spawn_drive(app: &mut App) {
    app.world_mut().spawn((
        VehicleController::standard_bus(12_000.0, 5.9),
        PlayerVehicle,
        SessionVehicle(1),
        GlobalTransform::from(
            Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(Quat::from_rotation_y(-0.5)),
        ),
    ));
    app.world_mut().spawn((
        SessionVehicle(2),
        GlobalTransform::from(Transform::from_xyz(40.0, 0.0, -60.0)),
    ));
}
