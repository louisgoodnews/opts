//! The covered-section rule (M-0115, option 1 of the decision brief): a spline whose graphical
//! profile is a ceiling is a covered road. The zone is the spline's ground box, padded by the
//! ceiling's half-width. Pure logic, no world access (ISS-092: moved to `opts-spatial-zones-core`).
//!
//! * A ceiling is a run of consecutive profile points at least [`MIN_CEILING_HEIGHT_M`] high that
//!   spans at least [`MIN_CEILING_WIDTH_M`] across (a U-shaped tunnel profile has its roof run).
//! * Overhead-line profiles (`Meterspur_normal_OL_*` and similar) pass the height test with catenary
//!   wires at 4.5 to 5.8 m; they are excluded by name, see [`is_overhead_wire`].
//! * A covered spline shorter than [`MIN_COVERED_LENGTH_M`] gives no zone.

use glam::Vec2;

use crate::model::{ZoneBox, ZoneId, ZoneKind};

/// Lowest point of a ceiling profile (m). Provisional default, not measured.
pub const MIN_CEILING_HEIGHT_M: f32 = 3.5;
/// Narrowest span of a ceiling profile (m). Provisional default, not measured.
pub const MIN_CEILING_WIDTH_M: f32 = 2.0;
/// Shortest covered stretch that yields a zone (m). Provisional default, not measured.
pub const MIN_COVERED_LENGTH_M: f32 = 15.0;

/// One placed spline of a tile, as the detector needs it. The profiles are the graphical profiles
/// of its `.sli`, each a list of `(x across, z height)` points, so the engine crate needs no OMSI type.
#[derive(Debug, Clone, Copy)]
pub struct SplineCandidate<'a> {
    /// The `.sli` profile path, as the map names it.
    pub sli_file: &'a str,
    /// The graphical profiles: `x` across the spline (m), `y` height (m).
    pub profiles: &'a [Vec<Vec2>],
    /// Ground-plane start of the spline in the sampler's plane (x east, y = Bevy z = -north).
    pub start: Vec2,
    /// Ground-plane end of the spline.
    pub end: Vec2,
    /// Arc length of the spline (m).
    pub length_m: f32,
}

/// Whether the profile file is an overhead-line profile: catenary wires, not a roof.
#[must_use]
pub fn is_overhead_wire(sli_file: &str) -> bool {
    let name = sli_file
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(sli_file)
        .to_ascii_lowercase();
    name.starts_with("meterspur_normal_ol")
        || name.starts_with("ol_")
        || name.contains("_ol_")
        || name.contains("oberleitung")
}

/// Half-width (m) of one run of consecutive high points, when the run spans enough.
fn run_half_width(xs: &[f32]) -> Option<f32> {
    let (left, right) = xs
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(l, r), &x| {
            (l.min(x), r.max(x))
        });
    (xs.len() >= 2 && right - left >= MIN_CEILING_WIDTH_M).then(|| left.abs().max(right.abs()))
}

/// Half-width (m) of the widest ceiling of one graphical profile: the runs of consecutive points at
/// least [`MIN_CEILING_HEIGHT_M`] high, each spanning [`MIN_CEILING_WIDTH_M`]. A U-shaped profile
/// (walls at ground level, roof above) is a ceiling through its roof run only.
fn profile_ceiling_half_width(points: &[Vec2]) -> Option<f32> {
    let mut best = None;
    let mut run: Vec<f32> = Vec::new();
    for p in points {
        if p.y >= MIN_CEILING_HEIGHT_M {
            run.push(p.x);
        } else {
            best = best
                .into_iter()
                .chain(run_half_width(&run))
                .reduce(f32::max);
            run.clear();
        }
    }
    best.into_iter()
        .chain(run_half_width(&run))
        .reduce(f32::max)
}

/// Half-width (m) of the ceiling profiles of a spline, or `None` when it has no ceiling.
#[must_use]
pub fn ceiling_half_width(profiles: &[Vec<Vec2>]) -> Option<f32> {
    profiles
        .iter()
        .filter_map(|points| profile_ceiling_half_width(points))
        .reduce(f32::max)
}

/// The zone of one covered spline, or `None` when it is an overhead-line profile, has no ceiling
/// or is shorter than [`MIN_COVERED_LENGTH_M`].
#[must_use]
pub fn covered_zone(id: ZoneId, candidate: &SplineCandidate) -> Option<ZoneBox> {
    if is_overhead_wire(candidate.sli_file) || candidate.length_m < MIN_COVERED_LENGTH_M {
        return None;
    }
    let half = ceiling_half_width(candidate.profiles)?;
    let pad = Vec2::splat(half);
    Some(ZoneBox {
        id,
        kind: ZoneKind::Tunnel,
        min: candidate.start.min(candidate.end) - pad,
        max: candidate.start.max(candidate.end) + pad,
    })
}

/// The zones of a tile's splines: one per covered spline, numbered in order.
#[must_use]
pub fn covered_zones(splines: &[SplineCandidate]) -> Vec<ZoneBox> {
    splines
        .iter()
        .filter_map(|c| covered_zone(ZoneId(0), c))
        .enumerate()
        .map(|(i, z)| ZoneBox {
            id: ZoneId(u32::try_from(i).unwrap_or(u32::MAX)),
            ..z
        })
        .collect()
}
