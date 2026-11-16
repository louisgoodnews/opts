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

mod draw;
mod input;
mod open;
mod overlay;
mod params;
mod plugin;
mod state;
pub use draw::{redraw, FullMapCanvas, FullMapImage};
pub use input::input;
pub use open::{follow, toggle};
pub use plugin::FullMapPlugin;
pub use state::{FullMapRequest, FullMapState};

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

pub use self::overview::OverviewRoads;
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

pub use overlay::{
    overlay, FullMapCaption, FullMapRoot, FullMapScaleBar, FullMapScaleText, FullMapSurface,
};
