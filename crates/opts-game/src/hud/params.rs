//! System-parameter bundles of the HUD systems.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::reporting::{ReportModalData, ReportModalState};
use opts_bevy::PlayerVehicle;
use opts_gui::prelude::MaterialTheme;
use opts_gui::theme::InstrumentVariant;

use opts_bevy::{SessionEnvironment, WeatherCatalogue};
use opts_gui::l10n::Localization;

use super::build::{HudRow, HudSlot, HudWidgetNode};
use super::layout::HudLayout;
use super::update::CameraLabel;
use super::values::HudValues;
use crate::in_game::types::InGameSessionInfo;
use crate::session_vehicles::SessionVehicle;
use crate::settings::GameSettings;

/// The optional resources a "report an issue" request touches.
#[derive(SystemParam)]
pub struct OptionalReport<'w> {
    /// The virtual clock.
    pub time: Res<'w, Time<Virtual>>,
    /// The modal's data.
    pub data: Option<ResMut<'w, ReportModalData>>,
    /// The modal's state.
    pub state: Option<Res<'w, State<ReportModalState>>>,
    /// The state to move to.
    pub next: Option<ResMut<'w, NextState<ReportModalState>>>,
}

/// The vehicles and cameras the map markers come from.
#[derive(SystemParam)]
pub struct MapPoses<'w, 's> {
    /// The player's and the session's vehicles.
    pub vehicles: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            Option<&'static SessionVehicle>,
            Has<PlayerVehicle>,
        ),
    >,
    /// The free camera.
    pub cameras: Query<'w, 's, &'static GlobalTransform, With<opts_bevy::MainCamera>>,
}

/// The colours' source of a map.
#[derive(SystemParam)]
pub struct MapLook<'w> {
    /// The instrument variant.
    pub variant: Res<'w, InstrumentVariant>,
    /// The theme, once there.
    pub theme: Option<Res<'w, MaterialTheme>>,
}

/// Everything the HUD text writer reads.
#[derive(SystemParam)]
pub struct TextEnv<'w> {
    /// The HUD values.
    pub values: Res<'w, HudValues>,
    /// The settings.
    pub settings: Res<'w, GameSettings>,
    /// The session's name and map.
    pub session: Res<'w, InGameSessionInfo>,
    /// The session's environment.
    pub env: Option<Res<'w, SessionEnvironment>>,
    /// The weather catalogue.
    pub weather: Option<Res<'w, WeatherCatalogue>>,
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The camera label.
    pub label: Res<'w, CameraLabel>,
    /// The HUD layout.
    pub layout: Res<'w, HudLayout>,
    /// The place under the vehicle.
    pub place: Res<'w, crate::session_vehicles::PlaceLabel>,
    /// The rebindable action keys.
    pub action_keys: Option<Res<'w, opts_bevy::ActionKeys>>,
}

/// The widget nodes whose display the HUD switches.
pub type DisplayNodes<'w, 's> = Query<
    'w,
    's,
    (
        &'static HudWidgetNode,
        &'static mut Node,
        Option<&'static mut Visibility>,
    ),
    (Without<HudSlot>, Without<HudRow>),
>;
/// The slot containers.
pub type DisplaySlots<'w, 's> =
    Query<'w, 's, (&'static HudSlot, &'static mut Node), (Without<HudWidgetNode>, Without<HudRow>)>;
/// The row containers.
pub type DisplayRows<'w, 's> =
    Query<'w, 's, (&'static HudRow, &'static mut Node), (Without<HudWidgetNode>, Without<HudSlot>)>;

/// The camera facts that decide whether the virtual mirrors show.
#[derive(SystemParam)]
pub struct MirrorView<'w> {
    /// The camera mode.
    pub camera: Res<'w, opts_bevy::camera::CameraState>,
    /// Which cameras exist.
    pub avail: Res<'w, opts_bevy::camera::CameraAvailability>,
    /// The viewpoint editor's state.
    pub editor: Option<Res<'w, opts_bevy::camera::viewpoint::ViewpointState>>,
}
