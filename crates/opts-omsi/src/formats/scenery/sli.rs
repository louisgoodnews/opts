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

/// What a path lane is for, beyond OMSI's four `[path]` types (`docs/src/simulation/lane-kinds.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LaneKind {
    /// Lane for cars, buses and trucks.
    Road,
    /// Lane for cyclists: a road-type path that is too narrow for a motor vehicle, or any
    /// pedestrian/road path of a profile whose file name says bike.
    Bike,
    /// Footpath.
    Pedestrian,
    /// Track (trams, trains).
    Rail,
    /// Aircraft path.
    Aircraft,
}

/// Widest road-type path (m) still read as a bike lane. The corpus has 59 road paths of width
/// 0.1 (e.g. `Invis_Human_Fahrrad.sli`, whose two ±0.85 m lanes are the bike lanes of a
/// footpath), 45 at 0.5; the 1,118 at 1.0 are bus-only guideways (`ÖPNV_BOT.sli`) and stay
/// road lanes ❓ (inferred, not documented by OMSI).
pub const BIKE_LANE_MAX_WIDTH_M: f32 = 0.6;

/// File-name fragments that mark a profile as a bike facility.
const BIKE_NAME_HINTS: [&str; 4] = ["fahrrad", "bike", "radweg", "cycle"];

impl SplinePath {
    /// Kind of lane; `profile_name` is the `.sli` file name (any case).
    #[must_use]
    pub fn kind(&self, profile_name: &str) -> LaneKind {
        let name = profile_name.to_lowercase();
        let named_bike = BIKE_NAME_HINTS.iter().any(|h| name.contains(h));
        match self.path_type {
            0 if self.width <= BIKE_LANE_MAX_WIDTH_M || (named_bike && self.width < 1.5) => {
                LaneKind::Bike
            }
            0 => LaneKind::Road,
            1 => LaneKind::Pedestrian,
            2 => LaneKind::Rail,
            _ => LaneKind::Aircraft,
        }
    }
}

/// Steepest profile segment (`|dz/dx|`) that still counts as ground surface.
pub const SURFACE_MAX_SLOPE: f32 = 0.25;
/// Highest a profile segment may lie above the datum and still count as ground surface (m).
pub const SURFACE_MAX_HEIGHT_M: f32 = 1.0;

/// One straight piece of a profile's ground surface ([`SplineProfile::ground_surface`]):
/// lateral offsets `x0 < x1` and heights relative to the datum (m).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceSegment {
    /// Left end (m).
    pub x0: f32,
    /// Height at the left end (m).
    pub z0: f32,
    /// Right end (m).
    pub x1: f32,
    /// Height at the right end (m).
    pub z1: f32,
}

impl SurfaceSegment {
    /// Height at lateral offset `x` (extrapolated outside the segment).
    #[must_use]
    pub fn z_at(&self, x: f32) -> f32 {
        let dx = self.x1 - self.x0;
        if dx.abs() < 1e-9 {
            return self.z0.min(self.z1);
        }
        self.z0 + (self.z1 - self.z0) * (x - self.x0) / dx
    }
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

impl SplineProfile {
    /// Lanes of the profile with their kind; `name` is the `.sli` file name.
    #[must_use]
    pub fn lane_kinds(&self, name: &str) -> Vec<(LaneKind, &SplinePath)> {
        self.paths.iter().map(|p| (p.kind(name), p)).collect()
    }

    /// Distinct lane kinds of the profile, in a fixed order. More than one kind means the spline
    /// is a *composite* (a street with a bike lane and a footpath, a tram street, ...): each kind
    /// is a sub-spline with its own traffic.
    #[must_use]
    pub fn composition(&self, name: &str) -> Vec<LaneKind> {
        let mut out: Vec<LaneKind> = Vec::new();
        for (k, _) in self.lane_kinds(name) {
            if !out.contains(&k) {
                out.push(k);
            }
        }
        out.sort_by_key(|k| *k as u8);
        out
    }

    /// Total width of the widest near-horizontal strip of the profile's cross-sections (m): how
    /// much flat surface a spline lays down. Paved areas without lanes (`10m_Pflaster1_op`,
    /// `10m_Asphalt7_op`, …) have none of the `[path]`s of a road but carry vehicles; kerbs,
    /// walls and fences have almost none.
    #[must_use]
    pub fn flat_width(&self) -> f32 {
        const FLAT_DZ: f32 = 0.15;
        self.graphical_profiles
            .iter()
            .map(|g| {
                g.points
                    .windows(2)
                    .filter(|w| (w[1].z - w[0].z).abs() < FLAT_DZ)
                    .map(|w| (w[1].x - w[0].x).abs())
                    .sum::<f32>()
            })
            .fold(0.0, f32::max)
    }

    /// The ground surface the profile lays down: the lowest up-facing, near-level segment of its
    /// graphical profiles at each lateral offset (`docs/src/architecture/terrain-carving.md` §3).
    ///
    /// A segment counts when it faces up (runs left to right), has `|dz/dx| ≤ 0.25` and lies at
    /// most 1.0 m above the datum; where several cover an offset the lowest wins (rail heads drop
    /// out, ballast stays). Embankment flanks, walls, hedges and platforms are left out by these
    /// rules. `[onlyeditor]` profiles have no surface. The result is sorted by `x0`, without
    /// overlaps. Mirrored splines use it with `x` negated.
    #[must_use]
    pub fn ground_surface(&self) -> Vec<SurfaceSegment> {
        if self.only_editor {
            return Vec::new();
        }
        let candidates: Vec<SurfaceSegment> = self
            .graphical_profiles
            .iter()
            .flat_map(|g| g.points.windows(2))
            .filter_map(|w| {
                let (a, b) = (&w[0], &w[1]);
                let dx = b.x - a.x;
                (dx > 1e-3
                    && ((b.z - a.z) / dx).abs() <= SURFACE_MAX_SLOPE
                    && a.z.max(b.z) <= SURFACE_MAX_HEIGHT_M)
                    .then_some(SurfaceSegment {
                        x0: a.x,
                        z0: a.z,
                        x1: b.x,
                        z1: b.z,
                    })
            })
            .collect();
        if candidates.is_empty() {
            return Vec::new();
        }
        // Lower envelope: split at every end point and crossing, take the lowest per interval.
        let mut cuts: Vec<f32> = candidates.iter().flat_map(|c| [c.x0, c.x1]).collect();
        for (i, a) in candidates.iter().enumerate() {
            for b in &candidates[i + 1..] {
                let (lo, hi) = (a.x0.max(b.x0), a.x1.min(b.x1));
                if hi <= lo {
                    continue;
                }
                let (da, db) = (a.z_at(lo) - b.z_at(lo), a.z_at(hi) - b.z_at(hi));
                if da * db < 0.0 {
                    cuts.push(lo + (hi - lo) * da / (da - db));
                }
            }
        }
        cuts.sort_by(f32::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
        let mut out: Vec<SurfaceSegment> = Vec::new();
        for w in cuts.windows(2) {
            let (x0, x1) = (w[0], w[1]);
            let mid = 0.5 * (x0 + x1);
            let Some(low) = candidates
                .iter()
                .filter(|c| c.x0 <= mid && mid <= c.x1)
                .min_by(|a, b| a.z_at(mid).total_cmp(&b.z_at(mid)))
            else {
                continue;
            };
            let piece = SurfaceSegment {
                x0,
                z0: low.z_at(x0),
                x1,
                z1: low.z_at(x1),
            };
            // Merge with the previous piece when it continues the same line.
            if let Some(prev) = out.last_mut() {
                let slope = |s: &SurfaceSegment| (s.z1 - s.z0) / (s.x1 - s.x0);
                if (prev.x1 - x0).abs() < 1e-5
                    && (prev.z1 - piece.z0).abs() < 1e-5
                    && (slope(prev) - slope(&piece)).abs() < 1e-5
                {
                    prev.x1 = piece.x1;
                    prev.z1 = piece.z1;
                    continue;
                }
            }
            out.push(piece);
        }
        out
    }

    /// Height of the ground surface (relative to the datum) at lateral offset `x`; `None` where
    /// the profile lays down no surface (see [`Self::ground_surface`]).
    #[must_use]
    pub fn surface_at(&self, x: f32) -> Option<f32> {
        self.ground_surface()
            .iter()
            .filter(|s| s.x0 - 1e-6 <= x && x <= s.x1 + 1e-6)
            .map(|s| s.z_at(x))
            .min_by(f32::total_cmp)
    }

    /// Whether the profile carries bike lanes.
    #[must_use]
    pub fn has_bike_lane(&self, name: &str) -> bool {
        self.lane_kinds(name)
            .iter()
            .any(|(k, _)| *k == LaneKind::Bike)
    }
}

mod parse;

#[cfg(test)]
mod tests;
