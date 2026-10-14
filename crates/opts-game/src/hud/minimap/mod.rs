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
    step_zoom, MiniMapConfig, Orientation, BUDGET_US, DEFAULT_ZOOM_M, SETTING_ENABLED,
    SETTING_ORIENTATION, SETTING_ZOOM, UPDATE_INTERVAL, ZOOM_STEPS_M,
};
pub use raster::{Canvas, MapColors, MapLine, MapMarker, MapView};
pub use source::{may_touch, push_chords, FixtureRoads, MapRoad, MapSource};

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
    /// Seconds (real time) since the last redraw.
    since: f32,
}

/// The texture's drawing buffer (kept between redraws).
#[derive(Resource, Default)]
pub struct MiniMapCanvas(Option<Canvas>);

/// The image the surface shows.
#[derive(Resource, Default)]
pub struct MiniMapImage(Option<Handle<Image>>);

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

/// F7 shows or hides the map, Shift+F7 switches north-up and heading-up, `+` and `-` zoom. The
/// keys change the settings, so the page and the keys agree and the choice is remembered.
pub fn minimap_keys(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut api: crate::settings_registry::SettingsApi,
) {
    use crate::settings_registry::ChangeSource;
    let shift = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let config = MiniMapConfig::read(&api.settings);
    let change = if keyboard.just_pressed(KeyCode::F7) {
        if shift {
            Some((
                SETTING_ORIENTATION,
                config.orientation.toggled().id().to_owned(),
            ))
        } else {
            Some((SETTING_ENABLED, (!config.enabled).to_string()))
        }
    } else if keyboard.any_just_pressed([KeyCode::Equal, KeyCode::NumpadAdd]) {
        Some((SETTING_ZOOM, format!("{}", step_zoom(config.zoom_m, 1))))
    } else if keyboard.any_just_pressed([KeyCode::Minus, KeyCode::NumpadSubtract]) {
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

/// What the map is centred on and the markers on it, in OMSI metres.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    /// The centre (the active vehicle, else the camera).
    pub centre: Option<DVec2>,
    /// Compass heading (radians) of the centre.
    pub heading: f32,
    /// All markers: (position, heading, active).
    pub markers: Vec<(DVec2, f32, bool)>,
}

/// Redraws the map at most 10 times per second.
#[allow(clippy::too_many_arguments)]
pub fn redraw(
    time: Res<Time<Real>>,
    mut state: ResMut<MiniMapState>,
    mut canvas: ResMut<MiniMapCanvas>,
    handle: Res<MiniMapImage>,
    mut images: ResMut<Assets<Image>>,
    variant: Res<InstrumentVariant>,
    theme: Option<Res<MaterialTheme>>,
    vehicles: Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    traffic: Option<Res<TrafficState>>,
    fixture: Option<Res<FixtureRoads>>,
) {
    state.since += time.delta_secs();
    if !state.config.enabled || state.since < UPDATE_INTERVAL.as_secs_f32() {
        return;
    }
    let (Some(canvas), Some(handle)) = (canvas.0.as_mut(), handle.0.as_ref()) else {
        return;
    };
    let started = std::time::Instant::now();
    state.since = 0.0;

    let mut scene = Scene::default();
    for (t, session, is_player) in &vehicles {
        if session.is_none() && !is_player {
            continue;
        }
        let (pos, heading) = pose_of(t);
        if is_player {
            scene.centre = Some(pos);
            scene.heading = heading;
        }
        scene.markers.push((pos, heading, is_player));
    }
    if scene.centre.is_none() {
        if let Some(t) = cameras.iter().next() {
            let (pos, heading) = pose_of(t);
            scene.centre = Some(pos);
            scene.heading = heading;
            scene.markers.push((pos, heading, true));
        }
    }
    let Some(centre) = scene.centre else {
        return;
    };

    let config = state.config;
    let colors = colors_of(
        theme
            .as_ref()
            .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
                t.instrument(*variant)
            }),
    );
    let view = MapView {
        radius_m: config.zoom_m,
        up_heading: match config.orientation {
            Orientation::North => 0.0,
            Orientation::Heading => scene.heading,
        },
    };
    // Roads: the lane graph's paths, then the fixture (tests).
    let mut roads = Vec::new();
    let radius = f64::from(view.radius_m) * std::f64::consts::SQRT_2;
    if let Some(t) = &traffic {
        t.roads_near(centre, radius, &mut roads);
    }
    if let Some(f) = &fixture {
        f.roads_near(centre, radius, &mut roads);
    }
    let px_per_m = f64::from(canvas.size) / 2.0 / f64::from(view.radius_m);
    let min_chord = 3.0 / px_per_m;
    let mut lines = Vec::with_capacity(roads.len() * 2);
    for road in &roads {
        push_chords(road, centre, min_chord, &mut lines);
    }
    let markers: Vec<MapMarker> = scene
        .markers
        .iter()
        .map(|(p, h, active)| MapMarker {
            pos: Vec2::new((p.x - centre.x) as f32, (p.y - centre.y) as f32),
            heading: *h,
            active: *active,
        })
        .collect();
    raster::render(canvas, view, &lines, &markers, &colors);
    if let Some(mut image) = images.get_mut(handle) {
        match image.data.as_mut() {
            Some(data) if data.len() == canvas.data.len() => data.copy_from_slice(&canvas.data),
            _ => image.data = Some(canvas.data.clone()),
        }
    }
    let us = started.elapsed().as_micros() as u64;
    state.frames += 1;
    state.last_us = us;
    state.worst_us = state.worst_us.max(us);
    state.lines = lines.len();
    state.markers = markers.len();
    state.centre = Some(centre);
    state.up_heading_deg = view.up_heading.to_degrees().rem_euclid(360.0);
}

/// Writes the caption below the map: the radius and what points up.
pub fn update_caption(
    state: Res<MiniMapState>,
    loc: Res<Localization>,
    mut captions: Query<&mut Text, With<MiniMapCaption>>,
) {
    let mode = match state.config.orientation {
        Orientation::North => keys::MINIMAP_NORTH_UP,
        Orientation::Heading => keys::MINIMAP_HEADING_UP,
    };
    let line = loc.t_args(
        keys::MINIMAP_CAPTION,
        &[
            ("metres".to_owned(), Arg::from(state.config.zoom_m as i32)),
            ("mode".to_owned(), Arg::from(loc.t(mode))),
        ],
    );
    for mut text in &mut captions {
        if text.0 != line {
            text.0 = line.clone();
        }
    }
}

/// The viewpoint editor's panel owns the right edge while it is open, so the map steps aside: it
/// is hidden (not removed, so nothing moves) and comes back when the editor closes.
pub fn avoid_viewpoint_panel(
    editor: Option<Res<opts_bevy::ViewpointState>>,
    mut maps: Query<&mut Visibility, With<super::parts::MapBox>>,
) {
    let want = if editor.is_some_and(|e| e.editing) {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut vis in &mut maps {
        if *vis != want {
            *vis = want;
        }
    }
}

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
