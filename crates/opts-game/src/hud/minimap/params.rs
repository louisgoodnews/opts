//! System-parameter bundles of the mini map's redraw.

use bevy::ecs::system::SystemParam;

use super::*;

pub use super::super::params::{MapLook, MapPoses};

/// The texture the map is drawn into.
#[derive(SystemParam)]
pub struct MiniTexture<'w> {
    /// The drawing buffer.
    pub canvas: ResMut<'w, MiniMapCanvas>,
    /// The image the widget shows.
    pub handle: Res<'w, MiniMapImage>,
    /// The image assets.
    pub images: ResMut<'w, Assets<Image>>,
}

/// Where the roads, routes and speed come from.
#[derive(SystemParam)]
pub struct MiniSources<'w> {
    /// The traffic state.
    pub traffic: Option<Res<'w, TrafficState>>,
    /// The player's route.
    pub route: Option<Res<'w, super::super::route_path::PlayerRoutePath>>,
    /// The scheduled vehicles.
    pub scheduled: Option<Res<'w, super::super::route_path::ScheduledVehicles>>,
    /// Fixture roads (tests, tools).
    pub fixture: Option<Res<'w, FixtureRoads>>,
    /// The HUD values (road speed).
    pub hud: Option<Res<'w, super::super::HudValues>>,
}
