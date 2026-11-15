//! Data records of a scenery object: the tree, bounding box, attachments, lights, paths and
//! texture and scripting hooks.

use super::*;

/// Billboard tree helper (`[tree]`): texture plus default height and width-ratio ranges.
///
/// Placed instances override the defaults through their per-instance string variables
/// (texture, height in metres, width/height ratio).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoTree {
    /// Billboard texture file.
    pub texture: String,
    /// Minimum height in metres.
    pub min_height: f32,
    /// Maximum height in metres.
    pub max_height: f32,
    /// Minimum width/height ratio.
    pub min_width_ratio: f32,
    /// Maximum width/height ratio.
    pub max_width_ratio: f32,
}

/// Bounding box of a scenery object.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoBoundingBox {
    /// Half extents in meters along local X, Y, and Z axes.
    pub half_extents: Vec3,
    /// Center offset in meters along local X, Y, and Z axes.
    pub center: Vec3,
}

/// An attachment point (`[new_attachment]`): where objects placed with `[attachObj]` sit on this
/// one, numbered in file order (`docs/src/formats/text/sco.md`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ScoAttachment {
    /// `attach_trans`: position in the object's frame (OMSI axes: x right, y forward, z up).
    pub translation: Vec3,
    /// `attach_rot_x`, `attach_rot_y`, `attach_rot_z` in degrees (z: about the up axis).
    pub rotation_deg: Vec3,
}

/// Map illumination light source (`[maplight]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoMapLight {
    /// Light source position in object-local meters.
    pub position: Vec3,
    /// Light color RGB (0.0..1.0).
    pub color: Vec3,
    /// Radius at full intensity in meters.
    pub radius: f32,
}

/// Spline helper junction arm (`[splinehelper]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoSplineHelper {
    /// Spline profile filename (`.sli`).
    pub profile_file: String,
    /// Position in object-local space.
    pub position: Vec3,
    /// Rotation in degrees.
    pub rotation_deg: f32,
    /// Additional numeric parameters.
    pub extra: Vec<f32>,
}

/// Traffic path lane segment for AI road traffic, pedestrians, or rail (`[path]`, `[path_2]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoPath {
    /// Start position in object-local coordinates.
    pub start: Vec3,
    /// Initial heading in degrees.
    pub heading_deg: f32,
    /// Curve radius in meters (large negative values indicate straight segment).
    pub radius_m: f32,
    /// Arc length in meters.
    pub length_m: f32,
    /// Starting gradient/slope.
    pub slope_start: f32,
    /// Ending gradient/slope.
    pub slope_end: f32,
    /// Path traffic type: 0 = road vehicles, 1 = pedestrians, 2 = rail.
    pub path_type: u32,
    /// Lane width in meters.
    pub width_m: f32,
    /// Allowed travel direction: 0 = forward only, 1 = reverse only, 2 = bidirectional.
    pub direction: u32,
    /// Extra flag or subtype.
    pub extra: Option<u32>,
    /// Index of traffic light controlling this path (if governed by `[use_traffic_light]`).
    pub traffic_light_index: Option<usize>,
    /// Distance in meters before path at which vehicles evaluate traffic light (`[approachdist]`).
    pub approach_distance_m: Option<f32>,
    /// The position of the rail points (`[switchdir]`) in which this path is passable; `None`
    /// for a path of every position (and for objects without `[switch]`).
    pub switch_position: Option<u32>,
}

/// Text texture configuration (`[texttexture]`, `[texttexture_enh]`).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoTextTexture {
    /// Source string variable index (`0` when the source is named, see `string_var`).
    pub string_index: usize,
    /// Source string variable *name* (`[texttexture_enh]` addresses its string by name,
    /// e.g. `DFI_Spalte_Linie1`).
    pub string_var: Option<String>,
    /// Font name.
    pub font_name: String,
    /// Rendered texture width in pixels.
    pub width: u32,
    /// Rendered texture height in pixels.
    pub height: u32,
    /// Color mode (0 = monochrome, 1 = keep colors).
    pub color_mode: u32,
    /// Text color RGB (0..255).
    pub text_color: [u8; 3],
    /// Alignment (`[texttexture_enh]` line 9): 0 centred, 1 left, 2 right; plain `[texttexture]`
    /// is centred.
    pub orientation: u32,
}

/// Change-The-Color (CTC) repaint picker configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoCtc {
    /// Variable name receiving the selection.
    pub var_name: String,
    /// Relative directory containing livery textures.
    pub texture_dir: String,
    /// Default selection index.
    pub default_index: usize,
}
