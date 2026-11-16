//! The lateral strips of a road profile that carry a height map, and the unevenness under a tyre.

use std::sync::Arc;

use opts_core::formats::scenery::SplineProfile;

use super::{SurfMap, CONTACT_PATCH_M};

/// Points the contact patch is sampled at.
const PATCH_POINTS: usize = 5;
/// A lateral strip of a road profile whose texture has a height map: between `x0` and `x1`
/// across the road (m, right positive), at heights `z0`/`z1`, with the texture's `u` and its
/// `v` per metre along the road at both edges.
#[derive(Debug, Clone)]
pub struct TexStrip {
    /// Left edge (m).
    pub x0: f32,
    /// Right edge (m).
    pub x1: f32,
    /// Height at the left edge (m).
    pub z0: f32,
    /// Height at the right edge (m).
    pub z1: f32,
    /// `u` at the left edge.
    pub u0: f32,
    /// `u` at the right edge.
    pub u1: f32,
    /// `v` per metre along the road at the left edge.
    pub vs0: f32,
    /// `v` per metre along the road at the right edge.
    pub vs1: f32,
    /// The height map.
    pub map: Arc<SurfMap>,
    /// A synthetic map is laid in metres, not texture coordinates: the length it repeats over
    /// across and along the road (m).
    pub period_m: Option<f32>,
}

impl TexStrip {
    /// The strip's share `t` (0 at `x0`, 1 at `x1`) of lateral offset `x`, when `x` lies on it.
    fn share(&self, x: f32) -> Option<f32> {
        let w = self.x1 - self.x0;
        (w > 1e-4 && x >= self.x0 - 0.02 && x <= self.x1 + 0.02)
            .then(|| ((x - self.x0) / w).clamp(0.0, 1.0))
    }

    /// The unevenness under a tyre at lateral offset `x` and `along_m` metres along the road
    /// (averaged over the contact patch), or `None` off the strip.
    #[must_use]
    pub fn unevenness(&self, x: f32, along_m: f32) -> Option<f32> {
        let t = self.share(x)?;
        let (u, vs) = match self.period_m {
            Some(p) => (x / p, 1.0 / p),
            None => (
                self.u0 + (self.u1 - self.u0) * t,
                self.vs0 + (self.vs1 - self.vs0) * t,
            ),
        };
        let sum: f32 = (0..PATCH_POINTS)
            .map(|k| {
                let d = (k as f32 / (PATCH_POINTS - 1) as f32 - 0.5) * CONTACT_PATCH_M;
                self.map.height_at(u, (along_m + d) * vs)
            })
            .sum();
        Some(sum / PATCH_POINTS as f32)
    }

    /// Height of the strip at lateral offset `x` (m).
    fn z_at(&self, x: f32) -> f32 {
        let t = self.share(x).unwrap_or(0.0);
        self.z0 + (self.z1 - self.z0) * t
    }
}

/// The unevenness of the top strip at lateral offset `x` whose surface lies within 5 cm of
/// `surface_z` (the drivable surface there; a kerb face or a lower layer does not count).
#[must_use]
pub fn unevenness_at(strips: &[TexStrip], x: f32, along_m: f32, surface_z: f32) -> f32 {
    strips
        .iter()
        .filter(|s| s.share(x).is_some() && (s.z_at(x) - surface_z).abs() < 0.05)
        .max_by(|a, b| a.z_at(x).total_cmp(&b.z_at(x)))
        .and_then(|s| s.unevenness(x, along_m))
        .unwrap_or(0.0)
}

/// The unevenness of plain asphalt at a place (m), for the ground that carries no texture in the
/// ground index (surface plates, terrain).
#[must_use]
pub fn plain_unevenness(x: f32, y: f32) -> f32 {
    static PLAIN: std::sync::OnceLock<(SurfMap, f32)> = std::sync::OnceLock::new();
    let (map, period) = PLAIN.get_or_init(|| SurfMap::synthetic(0));
    map.height_at(x / period, y / period)
}

/// The strips of `profile` whose texture has a height map; `surf_of(texture index)` gives the
/// map of a texture of the profile and, for a synthetic one, the length it repeats over.
#[must_use]
pub fn strips_of(
    profile: &SplineProfile,
    mut surf_of: impl FnMut(usize) -> Option<(Arc<SurfMap>, Option<f32>)>,
) -> Vec<TexStrip> {
    let mut out = Vec::new();
    for g in &profile.graphical_profiles {
        let Some((map, period_m)) = surf_of(g.texture_index) else {
            continue;
        };
        for w in g.points.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            if b.x - a.x < 1e-3 {
                continue;
            }
            out.push(TexStrip {
                x0: a.x,
                x1: b.x,
                z0: a.z,
                z1: b.z,
                u0: a.u,
                u1: b.u,
                vs0: a.v_slope,
                vs1: b.v_slope,
                map: map.clone(),
                period_m,
            });
        }
    }
    out
}
