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
pub mod zoom;

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

/// The map's colours from an instrument palette.
#[must_use]
pub fn colors_of(palette: &InstrumentPalette) -> MapColors {
    let u8s = |c: Color| Srgba::from(c).to_u8_array();
    let mut ground = u8s(palette.panel);
    ground[3] = 255;
    MapColors {
        ground,
        road: u8s(palette.text_dim),
        active: u8s(palette.text),
        other: u8s(palette.warning),
        north: u8s(palette.text),
        route: u8s(palette.ok),
    }
}

/// The compass heading (radians) of a world transform, and its (east, north) position.
#[must_use]
pub fn pose_of(t: &GlobalTransform) -> (DVec2, f32) {
    let p = t.translation();
    let f = t.forward();
    (DVec2::new(f64::from(p.x), f64::from(-p.z)), f.x.atan2(-f.z))
}

/// Keeps [`MiniMapState::config`] in step with the settings.
pub fn sync_config(settings: Res<GameSettings>, mut state: ResMut<MiniMapState>) {
    if settings.is_changed() || state.frames == 0 {
        let config = MiniMapConfig::read(&settings);
        if config != state.config {
            state.config = config;
        }
    }
}

/// F7 shows or hides the map, Shift+F7 switches north-up and heading-up, Ctrl+F7 switches the
/// speed zoom on and off, `+` and `-` zoom (the actions `minimap_*`, rebindable; the numpad
/// `+` and `-` always zoom). The keys change the settings, so the page and the keys agree and the choice is remembered.
pub fn minimap_keys(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    mut api: crate::settings_registry::SettingsApi,
) {
    use crate::settings_registry::ChangeSource;
    let keys = opts_bevy::action_keys_of(&action_keys);
    let config = MiniMapConfig::read(&api.settings);
    let change = if keys.just_pressed(&keyboard, "minimap_speed_zoom") {
        Some((
            SETTING_ZOOM_MODE,
            config.zoom_mode.toggled().id().to_owned(),
        ))
    } else if keys.just_pressed(&keyboard, "minimap_rotate") {
        Some((
            SETTING_ORIENTATION,
            config.orientation.toggled().id().to_owned(),
        ))
    } else if keys.just_pressed(&keyboard, "minimap_toggle") {
        Some((SETTING_ENABLED, (!config.enabled).to_string()))
    } else if keys.just_pressed(&keyboard, "minimap_zoom_in") {
        Some((SETTING_ZOOM, format!("{}", step_zoom(config.zoom_m, 1))))
    } else if keys.just_pressed(&keyboard, "minimap_zoom_out") {
        Some((SETTING_ZOOM, format!("{}", step_zoom(config.zoom_m, -1))))
    } else {
        None
    };
    if let Some((key, value)) = change {
        if let Err(e) = api.change(key, Some(&value), ChangeSource::Page, true) {
            tracing::warn!("mini map setting {key}: {e}");
        }
    }
}

/// Creates the texture when the surface needs another size and hands it to the surface node.
pub fn ensure_image(
    mut images: ResMut<Assets<Image>>,
    mut handle: ResMut<MiniMapImage>,
    mut canvas: ResMut<MiniMapCanvas>,
    mut state: ResMut<MiniMapState>,
    mut surfaces: Query<(&ComputedNode, &mut ImageNode), With<MiniMapSurface>>,
    palette_owner: (Option<Res<MaterialTheme>>, Res<InstrumentVariant>),
) {
    let Some((computed, mut image_node)) = surfaces.iter_mut().next() else {
        return;
    };
    // The texture matches the physical size of the node, within sensible bounds.
    let wanted = (computed.size().x.round() as u32).clamp(96, 448);
    let ground = palette_owner.0.as_ref().map_or([12, 15, 20, 255], |t| {
        colors_of(t.instrument(*palette_owner.1)).ground
    });
    if canvas.0.as_ref().is_none_or(|c| c.size != wanted) || handle.0.is_none() {
        let c = Canvas::new(wanted, ground);
        let image = Image::new(
            Extent3d {
                width: wanted,
                height: wanted,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            c.data.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        handle.0 = Some(images.add(image));
        canvas.0 = Some(c);
        state.texture_px = wanted;
        // Draw soon: do not wait for the next tick with an empty map.
        state.since = UPDATE_INTERVAL.as_secs_f32();
    }
    if let Some(h) = &handle.0 {
        if image_node.image != *h {
            image_node.image = h.clone();
        }
    }
}

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
