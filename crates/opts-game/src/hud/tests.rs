//! HUD tests: registry and profile consistency, the switches, focus, alerts, the camera label,
//! the developer panel and the Details dialog.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use opts_bevy::{CameraMode, CameraState};
use opts_gui::components::toast::{M3Toast, ToastActionButton, ToastSeverity};
use opts_gui::interaction::activation::Activated;
use opts_gui::interaction::focus::{Focusable, PointerOnly};
use opts_gui::prelude::*;

use super::build::{HudRoot, HudText, HudWidgetNode};
use super::fade::{FadeKind, HudFade, CAMERA_LABEL_SECONDS};
use super::profile::{unplaced, HudProfile, HudProfiles, ProfileProblem};
use super::update::CameraLabel;
use super::widget::{Binding, Slot, VehicleKind, WIDGETS};
use super::*;
use crate::app_state::AppState;
use crate::in_game::omsi_world::WorldStats;
use crate::in_game::types::InGameSessionInfo;
use crate::settings::GameSettings;
use crate::shell::ShellPlugin;

fn app() -> App {
    crate::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::dark())
        .init_state::<AppState>()
        .init_state::<opts_bevy::reporting::ReportModalState>()
        .init_resource::<opts_bevy::reporting::ReportModalData>()
        .init_resource::<CameraState>()
        .init_resource::<InGameSessionInfo>()
        .init_resource::<WorldStats>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(HudPlugin);
    // A drive with a vehicle: without one the HUD shows no instruments.
    app.world_mut().spawn((
        opts_bevy::VehicleController::standard_bus(12_000.0, 5.9),
        opts_bevy::PlayerVehicle,
    ));
    app.update();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    app
}

fn build(app: &mut App, kind: VehicleKind) {
    let profile = app
        .world()
        .resource::<HudProfiles>()
        .get(kind)
        .cloned()
        .expect("profile");
    let palette = MaterialTheme::dark()
        .instrument(opts_gui::theme::InstrumentVariant::Day)
        .to_owned();
    let mut commands = app.world_mut().commands();
    build::spawn_hud(&mut commands, &profile, &palette);
    app.update();
    app.update();
}

fn display_of(app: &mut App, id: &str) -> Option<Display> {
    app.world_mut()
        .query::<(&HudWidgetNode, &Node)>()
        .iter(app.world())
        .find(|(w, _)| w.id == id)
        .map(|(_, n)| n.display)
}

mod profiles;

mod layout;

mod alerts;

mod details;

mod control_label;

// ---- the cluster and the mini map (M-0071) ----

use std::time::Duration;

use super::minimap::{
    FixtureRoads, MiniMapConfig, MiniMapImage, MiniMapState, Orientation, SETTING_ENABLED,
    SETTING_ORIENTATION,
};
use serde_json::{json, Value};

mod full_map;
mod map_widget;
