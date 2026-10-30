//! System-parameter bundles of the full-size map's systems.

use bevy::ecs::system::SystemParam;

pub use super::super::params::{MapLook, MapPoses};
use super::draw::{FullMapCanvas, FullMapImage};
use super::overview::OverviewRoads;
use super::*;

/// What can keep the map shut besides the player: the load state, the dialogs and the world load.
#[derive(SystemParam)]
pub struct MapGates<'w> {
    /// The loading progress of the session.
    pub load: Option<Res<'w, SimulationLoadState>>,
    /// The report dialog holds the input.
    pub report: Option<Res<'w, ModalActive>>,
    /// The menu dialog holds the input.
    pub menu: Option<Res<'w, MenuModalActive>>,
    /// The running world load (its tile listing feeds the overview).
    pub job: Option<Res<'w, crate::in_game::omsi_world::WorldLoadJob>>,
}

/// The texture the map is drawn into.
#[derive(SystemParam)]
pub struct MapTexture<'w> {
    /// The drawing buffer.
    pub canvas: ResMut<'w, FullMapCanvas>,
    /// The image the surface shows.
    pub handle: ResMut<'w, FullMapImage>,
    /// The image assets.
    pub images: ResMut<'w, Assets<Image>>,
}

/// Where the roads and routes come from.
#[derive(SystemParam)]
pub struct MapSources<'w> {
    /// The traffic state.
    pub traffic: Option<Res<'w, TrafficState>>,
    /// Fixture roads (tests, tools).
    pub fixture: Option<Res<'w, FixtureRoads>>,
    /// The player's route.
    pub route: Res<'w, super::super::route_path::PlayerRoutePath>,
    /// The scheduled vehicles.
    pub scheduled: Res<'w, super::super::route_path::ScheduledVehicles>,
}

/// The overlay's nodes.
#[derive(SystemParam)]
pub struct OverlayNodes<'w, 's> {
    /// The texture node.
    pub surface: Query<'w, 's, (&'static mut Node, &'static mut ImageNode), With<FullMapSurface>>,
    /// The caption.
    pub caption: Query<
        'w,
        's,
        &'static mut Text,
        (With<FullMapCaption>, Without<FullMapScaleText>),
    >,
    /// The scale bar's text.
    pub scale_text: Query<
        'w,
        's,
        &'static mut Text,
        (With<FullMapScaleText>, Without<FullMapCaption>),
    >,
    /// The scale bar.
    pub scale_bar:
        Query<'w, 's, &'static mut Node, (With<FullMapScaleBar>, Without<FullMapSurface>)>,
}

/// What the overlay's texts and colours read.
#[derive(SystemParam)]
pub struct OverlayEnv<'w> {
    /// The session.
    pub session: Option<Res<'w, InGameSessionInfo>>,
    /// The overview of the roads.
    pub overview: Res<'w, OverviewRoads>,
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The instrument variant.
    pub variant: Res<'w, InstrumentVariant>,
    /// The theme, once there.
    pub theme: Option<Res<'w, MaterialTheme>>,
}
