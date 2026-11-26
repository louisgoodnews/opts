//! The pure drawing of the mini map: roads and markers into an RGBA8 buffer.
//!
//! No Bevy, no world: the inputs are a [`MapView`], road polylines and markers in metres relative
//! to the centre (east, north), and the colours. That makes the drawing deterministic, testable
//! against pixels, and cheap: coverage-based anti-aliased lines, culled to the square, with arcs
//! cut into chords of at least a few pixels. The budget is 1 ms for a frame (checked by a test
//! and reported live by `hud.state`).

use opts_minimap_zoom_canvas::canvas;
mod lines;
mod render;
mod shapes;

use bevy::math::Vec2;

pub use canvas::Canvas;
pub use render::{render, render_course, render_routed, render_with_unit};

/// What the map shows of the world: visible radius and orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapView {
    /// Metres from the centre to the edge of the square.
    pub radius_m: f32,
    /// Compass heading (radians, clockwise from north) that points up on the screen; `0` is
    /// north-up.
    pub up_heading: f32,
}

/// One drawable piece of a road: a straight chord, relative to the centre of the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapLine {
    /// Start (east, north) in metres.
    pub a: Vec2,
    /// End (east, north) in metres.
    pub b: Vec2,
    /// Width in metres.
    pub width_m: f32,
}

/// A vehicle (or the free camera) on the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapMarker {
    /// Position (east, north) in metres relative to the centre.
    pub pos: Vec2,
    /// Compass heading in radians.
    pub heading: f32,
    /// The vehicle the user drives (bigger, filled, with a ring).
    pub active: bool,
}

/// Colours as RGBA8, from the instrument tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapColors {
    /// Opaque ground.
    pub ground: [u8; 4],
    /// Roads.
    pub road: [u8; 4],
    /// The active vehicle.
    pub active: [u8; 4],
    /// The other session vehicles.
    pub other: [u8; 4],
    /// North mark and frame.
    pub north: [u8; 4],
    /// The player's route (its path and stops).
    pub route: [u8; 4],
}

/// The player's followed route: the chords of its path and its stops (east, north in metres
/// relative to the centre; the flag marks the next stop).
#[derive(Debug, Clone, Copy, Default)]
pub struct MapRoute<'a> {
    /// The path, drawn under the markers and over the roads.
    pub lines: &'a [MapLine],
    /// The stops of the run still ahead.
    pub stops: &'a [(Vec2, bool)],
    /// The scheduled vehicles on the road: position and compass heading.
    pub vehicles: &'a [(Vec2, f32)],
}

/// The timetable course of the player (M-0105): the chords of its resolved lanes, in their own
/// stroke colour (an instrument token, distinct from the route's). The default, no lines, draws
/// nothing, so a map without a course is unchanged.
#[derive(Debug, Clone, Copy, Default)]
pub struct MapCourse<'a> {
    /// The course's chords, relative to the centre.
    pub lines: &'a [MapLine],
    /// Stroke colour as RGBA8.
    pub color: [u8; 4],
}
