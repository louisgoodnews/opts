//! Parser and data structures for OMSI Spline Profile (`.sli`) files.
//!
//! A `.sli` file defines a 2D cross-section profile extruded along a 3D curve
//! in the world to produce roads, curbs, rails, bridges, and footpaths.
//! It contains terrain height profiles, graphical mesh ribbon profiles,
//! and traffic path lanes.
//!
//! See specification in `docs/src/formats/text/sli.md`.

use super::error::SceneryParseError;
use crate::formats::text::{decode_bytes, parse_float, scan_blocks};

/// Terrain height deformation profile strip (`[heightprofile]`).
#[derive(Debug, Clone, PartialEq)]
pub struct SplineHeightProfile {
    /// Starting X coordinate (left edge) relative to spline center in meters.
    pub x_start: f32,
    /// Ending X coordinate (right edge) relative to spline center in meters.
    pub x_end: f32,
    /// Starting elevation above spline origin in meters.
    pub z_start: f32,
    /// Ending elevation above spline origin in meters.
    pub z_end: f32,
}

/// Texture definition in a spline file (`[texture]`).
#[derive(Debug, Clone, PartialEq)]
pub struct SplineTexture {
    /// Texture bitmap filename.
    pub file: String,
    /// Optional material slot index.
    pub material_index: Option<usize>,
    /// Optional alpha transparency mode.
    pub alpha_mode: Option<u32>,
}

/// Point on a graphical extrusion profile polyline (`[profilepnt]`).
#[derive(Debug, Clone, PartialEq)]
pub struct SplineProfilePoint {
    /// Lateral offset across profile in meters (positive = right).
    pub x: f32,
    /// Vertical height offset in meters.
    pub z: f32,
    /// Horizontal texture U coordinate.
    pub u: f32,
    /// Texture V gradient per longitudinal meter.
    pub v_slope: f32,
}

/// Graphical profile ribbon extruded along the spline (`[profile]`).
#[derive(Debug, Clone, PartialEq)]
pub struct SplineGraphicalProfile {
    /// Texture index (0-based) referencing `textures`.
    pub texture_index: usize,
    /// Sequential cross-section vertices from left to right.
    pub points: Vec<SplineProfilePoint>,
}

/// AI traffic, pedestrian, or rail path lane in a spline (`[path]`, `[path_2]`).
#[derive(Debug, Clone, PartialEq)]
pub struct SplinePath {
    /// Path traffic type: 0 = road vehicles, 1 = pedestrians, 2 = rail.
    pub path_type: u32,
    /// Lateral offset from centerline in meters (positive = right).
    pub offset_x: f32,
    /// Vertical elevation offset above spline datum in meters.
    pub offset_z: f32,
    /// Evading lane width in meters.
    pub width: f32,
    /// Allowed travel direction: 0 = forward, 1 = reverse, 2 = bidirectional.
    pub direction: u32,
    /// Extra flag from `[path_2]`.
    pub extra: Option<u32>,
}

/// Rail geometry quality / track irregular oscillation parameters (`[rail_enh]`).
#[derive(Debug, Clone, PartialEq)]
pub struct SplineRailEnh {
    /// Length of individual track rails in meters.
    pub rail_length: f32,
    /// Rail joints style (0 = clean, 1 = rough).
    pub joints: u32,
    /// Horizontal reference wavelength in meters.
    pub wavelength: f32,
    /// Maximum horizontal amplitude in meters.
    pub amplitude: f32,
    /// Horizontal exponent power.
    pub power: f32,
    /// Vertical reference wavelength in meters.
    pub wavelength_z: f32,
    /// Maximum vertical amplitude in meters.
    pub amplitude_z: f32,
    /// Vertical exponent power.
    pub power_z: f32,
}

/// Complete representation of an OMSI spline cross-section profile (`.sli`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SplineProfile {
    /// Subdivision unit segment length in meters.
    pub length: Option<f32>,
    /// Half-width of superelevated banking zone in meters.
    pub half_cant_width: Option<f32>,
    /// Scale texture V coordinate by segment length rather than profile slope.
    pub scale_tex_by_length: bool,
    /// Render pass designation (`"4"`, `"presurface"`, etc.).
    pub render_type: Option<String>,
    /// Whether this spline is only visible in the map editor.
    pub only_editor: bool,

    /// Terrain height adjustment strips.
    pub height_profiles: Vec<SplineHeightProfile>,
    /// Preloaded texture bitmaps.
    pub textures: Vec<SplineTexture>,
    /// Graphical extruded ribbon profiles.
    pub graphical_profiles: Vec<SplineGraphicalProfile>,
    /// Traffic lanes and pedestrian paths.
    pub paths: Vec<SplinePath>,
    /// Enhanced rail physics / track geometry parameters.
    pub rail_enh: Option<SplineRailEnh>,
}

mod lanes;
mod parse;
mod surface;

pub use lanes::{LaneKind, BIKE_LANE_MAX_WIDTH_M};
pub use surface::{SurfaceSegment, SURFACE_MAX_HEIGHT_M, SURFACE_MAX_SLOPE};

#[cfg(test)]
mod tests;
