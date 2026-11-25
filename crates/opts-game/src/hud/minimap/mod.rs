//! The mini map (M-0071): a top-down vector drawing of the roads around the player, bottom right.
//!
//! * [`raster`]: the pure drawing (roads, vehicle markers, north mark) into an RGBA8 buffer.
//! * [`source`]: the thin read-only road interface; the lane graph's road paths implement it.
//! * this module: settings, keys, the 10 Hz update and the widget's texture.
//!
//! The map is centred on the player's vehicle, or on the free camera when there is no vehicle
//! (map-only session). It is **data-poor on purpose**: roads and vehicles only, no terrain, so it
//! costs a fraction of a millisecond and never touches the tile code. All colours come from the
//! instrument tokens; the ground is opaque, so the drawing reads over any scene.

pub mod config;
pub mod raster;
pub mod source;
pub use opts_minimap_zoom_canvas::zoom;

#[cfg(test)]
mod tests;

use bevy::asset::RenderAssetUsages;
use bevy::math::DVec2;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_bevy::PlayerVehicle;
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::Localization;
use opts_gui::prelude::MaterialTheme;
use opts_gui::theme::{InstrumentPalette, InstrumentVariant};

pub use config::{
    step_zoom, MiniMapConfig, Orientation, ZoomMode, BUDGET_US, DEFAULT_ZOOM_M, SETTING_ENABLED,
    SETTING_ORIENTATION, SETTING_ZOOM, SETTING_ZOOM_MODE, UPDATE_INTERVAL, ZOOM_STEPS_M,
};
pub use raster::{Canvas, MapColors, MapLine, MapMarker, MapRoute, MapView};
pub use source::{may_touch, push_chords, FixtureRoads, MapRoad, MapSource};
pub use zoom::ZoomFilter;

use super::keys;
use crate::in_game::omsi_world::TrafficState;
use crate::session_vehicles::SessionVehicle;
use crate::settings::GameSettings;

/// What the map did last (read by `hud.state` and the tests).
#[derive(Resource, Debug, Clone, Default)]
pub struct MiniMapState {
    /// The settings as last read.
    pub config: MiniMapConfig,
    /// Redraws since the session started.
    pub frames: u64,
    /// Time of the last redraw in microseconds.
    pub last_us: u64,
    /// The slowest redraw in microseconds.
    pub worst_us: u64,
    /// Road chords drawn by the last redraw.
    pub lines: usize,
    /// Markers drawn by the last redraw.
    pub markers: usize,
    /// Centre of the last redraw in OMSI metres (east, north).
    pub centre: Option<DVec2>,
    /// Compass heading (degrees) that pointed up in the last redraw.
    pub up_heading_deg: f32,
    /// Side of the texture in pixels.
    pub texture_px: u32,
    /// The radius drawn by the last redraw, metres (the zoom step in `fixed` mode).
    pub radius_m: f32,
    /// The radius the speed asks for, metres (equals `radius_m` in `fixed` mode).
    pub target_m: f32,
    /// The filtered speed that drives the zoom, km/h.
    pub speed_kmh: f32,
    /// The zoom smoothing.
    pub filter: ZoomFilter,
    /// Seconds (real time) since the last redraw.
    pub(super) since: f32,
}

/// The texture's drawing buffer (kept between redraws).
#[derive(Resource, Default)]
pub struct MiniMapCanvas(pub(super) Option<Canvas>);

/// The image the surface shows.
#[derive(Resource, Default)]
pub struct MiniMapImage(pub(super) Option<Handle<Image>>);

impl MiniMapImage {
    /// The texture, once the surface exists.
    #[must_use]
    pub fn handle(&self) -> Option<Handle<Image>> {
        self.0.clone()
    }
}

/// The node that shows the texture.
#[derive(Component, Debug, Clone, Copy)]
pub struct MiniMapSurface;

/// The scale and orientation line below the map.
#[derive(Component, Debug, Clone, Copy)]
pub struct MiniMapCaption;

/// Keeps [`MiniMapState::config`] in step with the settings.
pub fn sync_config(settings: Res<GameSettings>, mut state: ResMut<MiniMapState>) {
    if settings.is_changed() || state.frames == 0 {
        let config = MiniMapConfig::read(&settings);
        if config != state.config {
            state.config = config;
        }
    }
}

mod input;
mod texture;
pub use input::minimap_keys;
pub use texture::{colors_of, ensure_image, pose_of};

mod params;
mod redraw;
pub use redraw::{avoid_viewpoint_panel, redraw, update_caption, Scene};

/// Registers the resources and systems of the map.
pub struct MiniMapPlugin;

impl Plugin for MiniMapPlugin {
    fn build(&self, app: &mut App) {
        use crate::app_state::AppState;
        app.init_resource::<MiniMapState>()
            .init_resource::<MiniMapCanvas>()
            .init_resource::<MiniMapImage>()
            .init_resource::<InstrumentVariant>()
            .add_systems(
                OnEnter(AppState::InGame),
                |mut state: ResMut<MiniMapState>| {
                    *state = MiniMapState::default();
                },
            )
            .add_systems(
                Update,
                (
                    sync_config,
                    (ensure_image, redraw, update_caption).chain(),
                    avoid_viewpoint_panel,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                minimap_keys
                    .run_if(in_state(AppState::InGame))
                    .run_if(opts_bevy::reporting::not_modal_active)
                    .run_if(resource_exists::<crate::settings_registry::SettingsRegistry>),
            );
    }
}
