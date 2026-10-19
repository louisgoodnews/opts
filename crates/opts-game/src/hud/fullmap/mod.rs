//! The full-size map (M-0108): key F6 opens a full-screen, north-up map of the loaded roads with
//! the player's vehicle and the session vehicles on it. Drag pans, the wheel zooms toward the
//! cursor, Home follows the vehicle again, F6 or Esc closes. The simulation keeps running.
//!
//! * [`viewport`]: the pure maths of centre, zoom, pan and zoom-to-cursor.
//! * this module: the state, the keys and the mouse, the overlay and the redraw. The drawing is
//!   the mini map's ([`super::minimap::raster`], [`super::minimap::source`]); only the viewport,
//!   a larger texture and the redraw policy are new (on a change of the view at once, while the
//!   vehicles move at most ten times a second, otherwise twice a second).
//!
//! While the map is open it holds [`opts_bevy::reporting::MenuModalActive`] (through
//! [`FullMapState::holds_input`], which the vehicle menu includes in its flag), so driving, camera
//! and script keys stand still and Esc closes the map instead of opening the pause overlay.

pub mod viewport;

#[cfg(test)]
mod tests;

use bevy::asset::RenderAssetUsages;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::math::{DVec2, Vec2};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use opts_bevy::reporting::{MenuModalActive, ModalActive};
use opts_bevy::PlayerVehicle;
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::{L10nKey, Localization};
use opts_gui::prelude::*;
use opts_gui::theme::{InstrumentPalette, InstrumentVariant};

use self::viewport::{canvas_px, scale_bar_m, wheel_factor, Viewport};
use super::keys;
use super::minimap::{
    colors_of, pose_of, push_chords, raster, Canvas, FixtureRoads, MapLine, MapMarker, MapSource,
    MapView,
};
use super::parts::{panel, texts};
use crate::in_game::instruments::InstrumentInk;
use crate::in_game::omsi_world::TrafficState;
use crate::in_game::types::{InGameEntity, InGameSessionInfo};
use crate::in_game::SimulationLoadState;
use crate::session_vehicles::SessionVehicle;

/// The size of vehicle markers on the large canvas, in mini-map units.
const MARKER_UNIT: f32 = 1.4;
/// The shortest chord an arc is cut into, in canvas pixels (the mini map uses 3).
const MIN_CHORD_PX: f64 = 6.0;
/// The longest scale bar in logical pixels.
const SCALE_BAR_MAX_PX: f64 = 180.0;
/// Seconds between redraws while a vehicle moves.
const MOVING_INTERVAL_S: f32 = 0.1;
/// Seconds between redraws of a map nothing changes on (the markers stay current).
const IDLE_INTERVAL_S: f32 = 0.5;
/// The screen size assumed when there is no window (tests, headless runs).
const FALLBACK_SCREEN: Vec2 = Vec2::new(1280.0, 720.0);

/// A request from outside the keyboard (the control protocol).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FullMapRequest {
    /// Open the map (a no-op while it is open or another dialog holds the input).
    Open,
    /// Close the map.
    Close,
}

/// What the full-size map shows and did last (read by `hud.fullmap` and the tests).
#[derive(Resource, Debug, Clone)]
pub struct FullMapState {
    /// The map is shown.
    pub open: bool,
    /// What part of the world the screen shows.
    pub viewport: Viewport,
    /// The centre follows the active vehicle (cleared by a drag, set again by Home).
    pub follow: bool,
    /// The span kept between openings.
    pub remembered_span_m: f64,
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
    /// Side of the texture in pixels.
    pub texture_px: u32,
    /// The logical size of the screen as last seen.
    pub screen: Vec2,
    /// A request from the protocol, handled by the next frame.
    pub request: Option<FullMapRequest>,
    /// The protocol set the view: opening keeps it instead of centring on the vehicle.
    pub preset_view: bool,
    /// What the last redraw showed.
    drawn: Option<(Viewport, Vec<(DVec2, f32, bool)>)>,
    /// A change the user made (drag, wheel, Home): redraw this frame, not at the next tick.
    immediate: bool,
    /// Seconds (real time) since the last redraw.
    since: f32,
    /// The input stays held for a frame or two after closing, so the Esc press that closed the
    /// map does not also open the pause overlay.
    grace: u8,
    /// Where the cursor was when the drag began or last moved.
    drag_from: Option<Vec2>,
}

impl Default for FullMapState {
    fn default() -> Self {
        Self {
            open: false,
            viewport: Viewport::default(),
            follow: true,
            remembered_span_m: viewport::DEFAULT_SPAN_M,
            frames: 0,
            last_us: 0,
            worst_us: 0,
            lines: 0,
            markers: 0,
            texture_px: 0,
            screen: FALLBACK_SCREEN,
            request: None,
            preset_view: false,
            drawn: None,
            immediate: false,
            since: 0.0,
            grace: 0,
            drag_from: None,
        }
    }
}

impl FullMapState {
    /// Whether the map owns the keyboard and the mouse: open, or just closed (see `grace`).
    #[must_use]
    pub fn holds_input(&self) -> bool {
        self.open || self.grace > 0
    }

    /// Asks for a redraw in the next frame (the view was changed from outside the mouse).
    pub fn touch(&mut self) {
        self.immediate = true;
    }

    fn close(&mut self) {
        if self.open {
            self.remembered_span_m = self.viewport.span_m;
            self.open = false;
            self.grace = 2;
            self.drag_from = None;
        }
    }
}

/// The texture's drawing buffer (kept between redraws).
#[derive(Resource, Default)]
pub struct FullMapCanvas(Option<Canvas>);

/// The image the surface shows.
#[derive(Resource, Default)]
pub struct FullMapImage(Option<Handle<Image>>);

/// The root of the overlay.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapRoot;

/// The node that shows the texture (a square that covers the screen).
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapSurface;

/// The map name and the zoom.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapCaption;

/// The bar of the scale.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapScaleBar;

/// The length of the scale bar as text.
#[derive(Component, Debug, Clone, Copy)]
pub struct FullMapScaleText;

/// The logical size of the primary window, or the fallback.
fn screen_of(windows: &Query<&Window, With<PrimaryWindow>>) -> Vec2 {
    windows
        .single()
        .map_or(FALLBACK_SCREEN, |w| Vec2::new(w.width(), w.height()))
}

/// The active vehicle's (else the camera's) position and heading.
fn active_pose(
    vehicles: &Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: &Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) -> Option<(DVec2, f32)> {
    vehicles
        .iter()
        .find(|(_, _, player)| *player)
        .map(|(t, _, _)| pose_of(t))
        .or_else(|| cameras.iter().next().map(pose_of))
}

/// F6 opens and closes the map, Esc closes it, a protocol request does either. Opening needs the
/// session to be loaded and no other dialog (report, vehicle menu, pause) to hold the input.
#[allow(clippy::too_many_arguments)]
pub fn toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    load: Option<Res<SimulationLoadState>>,
    report: Option<Res<ModalActive>>,
    menu: Option<Res<MenuModalActive>>,
    vehicles: Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut state: ResMut<FullMapState>,
) {
    let request = state.request.take();
    let toggle_key = keyboard.just_pressed(KeyCode::F6);
    if state.open {
        if toggle_key
            || keyboard.just_pressed(KeyCode::Escape)
            || request == Some(FullMapRequest::Close)
        {
            state.close();
        }
        return;
    }
    if state.grace > 0 {
        state.grace -= 1;
    }
    let wants_open = toggle_key || request == Some(FullMapRequest::Open);
    // The flag of the menu also carries this map's own hold (just after closing); anything else
    // that holds it keeps the map shut.
    let other_modal =
        report.is_some_and(|m| m.0) || (menu.is_some_and(|m| m.0) && state.grace == 0);
    let loaded = load.is_none_or(|l| l.is_complete);
    if !wants_open || other_modal || !loaded {
        return;
    }
    if state.preset_view {
        state.preset_view = false;
    } else {
        let centre = active_pose(&vehicles, &cameras).map_or(state.viewport.centre, |(p, _)| p);
        state.viewport = Viewport::new(centre, state.remembered_span_m);
        state.follow = true;
    }
    state.open = true;
    state.immediate = true;
    state.drawn = None;
}

/// Drag pans, the wheel zooms toward the cursor, Home follows the vehicle again.
pub fn input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<FullMapState>,
) {
    let screen = screen_of(&windows);
    state.screen = screen;
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    let mut changed = false;
    // Drag: the world follows the cursor.
    if mouse.pressed(MouseButton::Left) {
        if let Some(now) = cursor {
            if let Some(from) = state.drag_from {
                let delta = now - from;
                if delta != Vec2::ZERO {
                    state.viewport.pan_pixels(delta, screen);
                    state.follow = false;
                    changed = true;
                }
            }
            state.drag_from = Some(now);
        }
    } else {
        state.drag_from = None;
    }
    // Wheel: zoom toward the cursor (the middle of the screen without a cursor).
    for ev in wheel.read() {
        let lines = match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / 40.0,
        };
        if lines != 0.0 {
            let at = cursor.unwrap_or(screen / 2.0);
            state.viewport.zoom_at(at, screen, wheel_factor(lines));
            changed = true;
        }
    }
    if keyboard.just_pressed(KeyCode::Home) {
        state.follow = true;
        changed = true;
    }
    if changed {
        state.immediate = true;
    }
}

/// While following, the centre is the active vehicle (else the camera).
pub fn follow(
    vehicles: Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut state: ResMut<FullMapState>,
) {
    if !state.follow {
        return;
    }
    if let Some((pos, _)) = active_pose(&vehicles, &cameras) {
        if state.viewport.centre != pos {
            state.viewport.centre = pos;
        }
    }
}

fn texts_bundle(key: &'static str, palette: &InstrumentPalette, ink: InstrumentInk) -> impl Bundle {
    (texts(key, TypeScale::Label, ink, palette), L10nKey(key))
}

/// Spawns the overlay.
fn spawn_overlay(commands: &mut Commands, palette: &InstrumentPalette, ground: Color) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(ground),
            GlobalZIndex(1000),
            InGameEntity,
            FullMapRoot,
            UiId::new("hud.fullmap"),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ImageNode::default(),
                FullMapSurface,
                UiId::new("hud.fullmap.surface"),
            ));
            // The map name and the zoom, top left.
            root.spawn(panel(
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    top: Val::Px(12.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Medium,
                palette,
            ))
            .with_children(|p| {
                p.spawn((
                    texts("", TypeScale::BodyLarge, InstrumentInk::Text, palette),
                    FullMapCaption,
                    UiId::new("hud.fullmap.caption"),
                ));
            });
            // The scale bar, bottom left.
            root.spawn(panel(
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(16.0),
                    bottom: Val::Px(12.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Medium,
                palette,
            ))
            .with_children(|p| {
                p.spawn((
                    Node {
                        width: Val::Px(100.0),
                        height: Val::Px(3.0),
                        ..default()
                    },
                    BackgroundColor(palette.text),
                    FullMapScaleBar,
                ));
                p.spawn((
                    texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
                    FullMapScaleText,
                    UiId::new("hud.fullmap.scale"),
                ));
            });
            // The keys, bottom right.
            root.spawn(panel(
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(16.0),
                    bottom: Val::Px(12.0),
                    padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                RadiusLevel::Medium,
                palette,
            ))
            .with_children(|p| {
                p.spawn((
                    texts_bundle(keys::FULLMAP_HINT, palette, InstrumentInk::TextDim),
                    UiId::new("hud.fullmap.hint"),
                ));
            });
        });
}

/// Builds the overlay when the map opens and removes it when it closes; keeps the texture node,
/// the caption and the scale bar in step with the window and the zoom.
#[allow(clippy::too_many_arguments)]
pub fn overlay(
    mut commands: Commands,
    state: Res<FullMapState>,
    image: Res<FullMapImage>,
    roots: Query<Entity, With<FullMapRoot>>,
    mut surface: Query<(&mut Node, &mut ImageNode), With<FullMapSurface>>,
    mut caption: Query<&mut Text, (With<FullMapCaption>, Without<FullMapScaleText>)>,
    mut scale_text: Query<&mut Text, (With<FullMapScaleText>, Without<FullMapCaption>)>,
    mut scale_bar: Query<&mut Node, (With<FullMapScaleBar>, Without<FullMapSurface>)>,
    session: Option<Res<InGameSessionInfo>>,
    loc: Res<Localization>,
    variant: Res<InstrumentVariant>,
    theme: Option<Res<MaterialTheme>>,
) {
    let existing = roots.iter().next();
    if !state.open {
        if let Some(root) = existing {
            commands.entity(root).try_despawn();
        }
        return;
    }
    let palette = *theme
        .as_ref()
        .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
            t.instrument(*variant)
        });
    if existing.is_none() {
        let ground = Color::srgba_u8(
            colors_of(&palette).ground[0],
            colors_of(&palette).ground[1],
            colors_of(&palette).ground[2],
            255,
        );
        spawn_overlay(&mut commands, &palette, ground);
        return;
    }
    let screen = state.screen;
    // The texture covers the square around the screen; the root clips it.
    let side = screen.max_element();
    for (mut node, mut img) in &mut surface {
        let (w, h) = (Val::Px(side), Val::Px(side));
        let (left, top) = (
            Val::Px((screen.x - side) / 2.0),
            Val::Px((screen.y - side) / 2.0),
        );
        if node.width != w || node.height != h || node.left != left || node.top != top {
            node.width = w;
            node.height = h;
            node.left = left;
            node.top = top;
        }
        if let Some(h) = &image.0 {
            if img.image != *h {
                img.image = h.clone();
            }
        }
    }
    let map = session.as_ref().map_or("", |s| s.map_name.as_str());
    let ppm = state.viewport.px_per_m(screen);
    let bar_m = scale_bar_m(SCALE_BAR_MAX_PX / ppm);
    let bar_text = if bar_m >= 1000.0 {
        loc.t_args(
            keys::FULLMAP_SCALE_KM,
            &[("km".to_owned(), Arg::from((bar_m / 1000.0).round() as i32))],
        )
    } else {
        loc.t_args(
            keys::FULLMAP_SCALE_M,
            &[("metres".to_owned(), Arg::from(bar_m.round() as i32))],
        )
    };
    let title = if map.is_empty() {
        loc.t(keys::FULLMAP_TITLE)
    } else {
        map.to_owned()
    };
    for mut t in &mut caption {
        if t.0 != title {
            t.0.clone_from(&title);
        }
    }
    for mut t in &mut scale_text {
        if t.0 != bar_text {
            t.0.clone_from(&bar_text);
        }
    }
    for mut node in &mut scale_bar {
        let w = Val::Px((bar_m * ppm) as f32);
        if node.width != w {
            node.width = w;
        }
    }
}

/// Redraws the map: at once after a change of the view, while a vehicle moves at most ten times
/// a second, otherwise twice a second.
#[allow(clippy::too_many_arguments)]
pub fn redraw(
    time: Res<Time<Real>>,
    mut state: ResMut<FullMapState>,
    mut canvas: ResMut<FullMapCanvas>,
    mut handle: ResMut<FullMapImage>,
    mut images: ResMut<Assets<Image>>,
    variant: Res<InstrumentVariant>,
    theme: Option<Res<MaterialTheme>>,
    vehicles: Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    traffic: Option<Res<TrafficState>>,
    fixture: Option<Res<FixtureRoads>>,
) {
    state.since += time.delta_secs();
    let mut markers: Vec<(DVec2, f32, bool)> = Vec::new();
    for (t, session, is_player) in &vehicles {
        if session.is_some() || is_player {
            let (pos, heading) = pose_of(t);
            markers.push((pos, heading, is_player));
        }
    }
    if !markers.iter().any(|m| m.2) {
        if let Some(t) = cameras.iter().next() {
            let (pos, heading) = pose_of(t);
            markers.push((pos, heading, true));
        }
    }
    let screen = state.screen;
    let side = canvas_px(screen);
    let resized = canvas.0.as_ref().is_none_or(|c| c.size != side) || handle.0.is_none();
    let view_changed = state
        .drawn
        .as_ref()
        .is_none_or(|(v, _)| *v != state.viewport);
    let markers_moved = state.drawn.as_ref().is_none_or(|(_, old)| {
        old.len() != markers.len()
            || old
                .iter()
                .zip(&markers)
                .any(|(a, b)| a.0.distance(b.0) > 0.05 || (a.1 - b.1).abs() > 0.01 || a.2 != b.2)
    });
    let due = resized
        || state.immediate
        || (view_changed && state.since >= MOVING_INTERVAL_S)
        || (markers_moved && state.since >= MOVING_INTERVAL_S)
        || state.since >= IDLE_INTERVAL_S;
    if !due {
        return;
    }
    let started = std::time::Instant::now();
    let colors = colors_of(
        theme
            .as_ref()
            .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
                t.instrument(*variant)
            }),
    );
    if resized {
        let c = Canvas::new(side, colors.ground);
        let image = Image::new(
            Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            c.data.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        handle.0 = Some(images.add(image));
        canvas.0 = Some(c);
        state.texture_px = side;
    }
    let (Some(canvas), Some(handle)) = (canvas.0.as_mut(), handle.0.as_ref()) else {
        return;
    };
    state.since = 0.0;
    state.immediate = false;

    let viewport = state.viewport;
    let radius_m = viewport.canvas_radius_m(screen);
    let view = MapView {
        radius_m: radius_m as f32,
        up_heading: 0.0,
    };
    let mut roads = Vec::new();
    let reach = radius_m * std::f64::consts::SQRT_2;
    if let Some(t) = &traffic {
        t.roads_near(viewport.centre, reach, &mut roads);
    }
    if let Some(f) = &fixture {
        f.roads_near(viewport.centre, reach, &mut roads);
    }
    let px_per_m = f64::from(canvas.size) / 2.0 / radius_m;
    // Chords of at least six pixels: the large canvas has many arcs, and a six-pixel chord is
    // still within a fifth of a pixel of the arc (this keeps a redraw near the 2 ms budget).
    let min_chord = MIN_CHORD_PX / px_per_m;
    let mut lines: Vec<MapLine> = Vec::with_capacity(roads.len() * 2);
    for road in &roads {
        push_chords(road, viewport.centre, min_chord, &mut lines);
    }
    let drawn_markers: Vec<MapMarker> = markers
        .iter()
        .map(|(p, h, active)| MapMarker {
            pos: Vec2::new(
                (p.x - viewport.centre.x) as f32,
                (p.y - viewport.centre.y) as f32,
            ),
            heading: *h,
            active: *active,
        })
        .collect();
    raster::render_with_unit(
        canvas,
        view,
        &lines,
        &drawn_markers,
        &colors,
        MARKER_UNIT,
        false,
    );
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
    state.markers = drawn_markers.len();
    state.drawn = Some((viewport, markers));
}

/// Registers the resources and systems of the full-size map.
pub struct FullMapPlugin;

impl Plugin for FullMapPlugin {
    fn build(&self, app: &mut App) {
        use crate::app_state::AppState;
        let open = |s: Res<FullMapState>| s.open;
        app.init_resource::<FullMapState>()
            .init_resource::<FullMapCanvas>()
            .init_resource::<FullMapImage>()
            .add_systems(
                OnEnter(AppState::InGame),
                |mut state: ResMut<FullMapState>, mut image: ResMut<FullMapImage>| {
                    let span = state.remembered_span_m;
                    *state = FullMapState {
                        remembered_span_m: span,
                        ..FullMapState::default()
                    };
                    image.0 = None;
                },
            )
            .add_systems(
                Update,
                (
                    toggle,
                    input.run_if(open),
                    follow.run_if(open),
                    redraw.run_if(open),
                    overlay,
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            );
    }
}
