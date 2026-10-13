//! The in-game HUD on fixture content (invented map, vehicles and roads, no OMSI assets), laid
//! out by Bevy's real UI layout in a headless app. Shared by the HUD matrix and the golden test.

#![allow(dead_code)]

use std::time::Duration;

use bevy::prelude::*;
use opts_bevy::{CameraMode, CameraState, PlayerVehicle, VehicleController};
use opts_game::hud::minimap::FixtureRoads;
use opts_game::hud::{build, HudPlugin, HudProfiles, RaiseAlert, VehicleKind};
use opts_game::in_game::omsi_world::WorldStats;
use opts_game::in_game::types::InGameSessionInfo;
use opts_game::session_vehicles::SessionVehicle;
use opts_game::settings::GameSettings;
use opts_game::shell::ShellPlugin;
use opts_game::AppState;
use opts_gui::testing::layout_base;
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::AccentPreset;

use super::screens::{preferences, resize};

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

impl Extras {
    /// Everything at once.
    pub const ALL: Self = Self {
        alert: true,
        toast: true,
        camera_label: true,
        editor: true,
    };
}

/// A HUD of `kind` in a window of `w` x `h` logical pixels at UI scale `scale`.
pub fn build_hud(w: u32, h: u32, scale: f32, kind: VehicleKind, extras: Extras) -> App {
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
    // A drive with two session vehicles: the active one and another one 60 m away.
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
    for _ in 0..8 {
        app.update();
    }
    app
}

/// The layout rectangle `[x, y, w, h]` (layout pixels) of the element with `id`.
pub fn rect(app: &mut App, id: &str) -> Option<[f32; 4]> {
    let e = opts_gui::ui_tree::find_entity(app.world_mut(), id)?;
    opts_gui::ui_tree::rect_of(app.world(), e)
}

/// Whether an entity or an ancestor is not shown.
pub fn hidden(app: &App, mut e: Entity) -> bool {
    loop {
        let w = app.world();
        if w.get::<Node>(e).is_some_and(|n| n.display == Display::None)
            || w.get::<Visibility>(e) == Some(&Visibility::Hidden)
        {
            return true;
        }
        match w.get::<ChildOf>(e) {
            Some(p) => e = p.parent(),
            None => return false,
        }
    }
}

/// Whether two rectangles overlap by more than half a pixel.
pub fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] + 0.5 < b[0] + b[2]
        && b[0] + 0.5 < a[0] + a[2]
        && a[1] + 0.5 < b[1] + b[3]
        && b[1] + 0.5 < a[1] + a[3]
}
