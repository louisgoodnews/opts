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

pub mod overview;
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

use self::overview::OverviewRoads;
use self::viewport::{wheel_factor, Viewport};
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

/// What the last redraw showed: the view and the markers (position, heading, active).
type DrawnState = (Viewport, Vec<(DVec2, f32, bool)>);

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
    /// Fit the whole map into the screen in the next frame (the protocol or a key).
    pub fit_request: bool,
    /// The protocol set the view: opening keeps it instead of centring on the vehicle.
    pub preset_view: bool,
    /// The overview generation the last redraw drew.
    drawn_generation: u64,
    /// What the last redraw showed.
    drawn: Option<DrawnState>,
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
            fit_request: false,
            preset_view: false,
            drawn_generation: 0,
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

mod draw;
pub use draw::{redraw, FullMapCanvas, FullMapImage};

mod overlay;
pub use overlay::{
    overlay, FullMapCaption, FullMapRoot, FullMapScaleBar, FullMapScaleText, FullMapSurface,
};

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
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    load: Option<Res<SimulationLoadState>>,
    report: Option<Res<ModalActive>>,
    menu: Option<Res<MenuModalActive>>,
    vehicles: Query<(
        &GlobalTransform,
        Option<&SessionVehicle>,
        Has<PlayerVehicle>,
    )>,
    cameras: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    job: Option<Res<crate::in_game::omsi_world::WorldLoadJob>>,
    mut overview: ResMut<OverviewRoads>,
    mut state: ResMut<FullMapState>,
) {
    let request = state.request.take();
    let toggle_key = opts_bevy::action_keys_of(&action_keys).just_pressed(&keyboard, "full_map");
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
    // The first time the map is opened, a reader starts on the roads of the whole map.
    if let Some(job) = job.filter(|_| !overview.started()) {
        let (root, map_dir, tiles) = job.tile_listing();
        overview.start(root, map_dir, tiles);
    }
}

/// Drag pans, the wheel zooms toward the cursor, Home follows the vehicle again.
pub fn input(
    keyboard: Res<ButtonInput<KeyCode>>,
    overview: Res<OverviewRoads>,
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
    if keyboard.just_pressed(KeyCode::End) || state.fit_request {
        state.fit_request = false;
        if let Some((low, high)) = overview.extent() {
            state.viewport = Viewport::fit(low, high, screen);
            state.follow = false;
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

/// Registers the resources and systems of the full-size map.
pub struct FullMapPlugin;

impl Plugin for FullMapPlugin {
    fn build(&self, app: &mut App) {
        use crate::app_state::AppState;
        let open = |s: Res<FullMapState>| s.open;
        app.init_resource::<FullMapState>()
            .init_resource::<FullMapCanvas>()
            .init_resource::<FullMapImage>()
            .init_resource::<OverviewRoads>()
            .add_systems(
                OnEnter(AppState::InGame),
                |mut state: ResMut<FullMapState>,
                 mut image: ResMut<FullMapImage>,
                 mut overview: ResMut<OverviewRoads>| {
                    *overview = OverviewRoads::default();
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
