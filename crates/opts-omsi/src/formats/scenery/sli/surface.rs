//! Ground surface of a spline profile: the lowest up-facing, near-level segments of its ribbons.

use super::*;

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

impl SplineProfile {
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
}
