//! The fine unevenness of road textures (M-0146): a street texture may have a `.surf` bitmap
//! beside it (`koppstein_1.dds.surf`), whose red channel is a height map: mid-grey is level,
//! white about +2 cm, black about −2 cm (community wiki, `docs/src/formats/community-crosscheck.md`).
//! Under the wheels it shakes the body on its springs, which the vehicle scripts read as
//! `A_Trans_Z` and turn into interior rattling (`klappern.osc`).
//!
//! A spline profile's ribbons give each lateral strip of the road its texture and texture
//! coordinates (`u` across, `v` per metre along); [`TexStrip`] keeps them with the texture's
//! height map. A texture without a `.surf` gets a synthetic one from its surface type
//! (`[surface]` of `<texture>.cfg`, [`roughness_of`]): cobblestones and gravel shake, asphalt
//! barely. That part is OPTS's assumption ❓; OMSI's handling of such textures is not documented.

use std::sync::Arc;

use opts_core::formats::scenery::SplineProfile;

/// Height of the brightest red value (m).
pub const SURF_AMPLITUDE_M: f32 = 0.02;
/// Length of the tyre's contact patch along the road (m): the height map is averaged over it, as
/// a tyre does not follow ridges shorter than its footprint.
pub const CONTACT_PATCH_M: f32 = 0.12;
/// Cells of a synthetic height map along each side.
const NOISE_CELLS: usize = 32;

/// The synthetic roughness of a surface type (`[surface]`, `docs/src/formats/text/texture-cfg.md`)
/// for a texture without a `.surf`: the peak height (m) and the size of its bumps (m). OPTS's
/// assumption ❓: smooth asphalt and concrete a few millimetres, setts and pavers about a
/// centimetre, unpaved ground more.
#[must_use]
pub fn roughness_of(surface: u32) -> (f32, f32) {
    match surface {
        1 => (0.003, 0.5),      // concrete
        2 => (0.012, 0.22),     // cobblestone
        3 | 12 => (0.015, 0.6), // dirt, sand
        4 | 11 => (0.02, 0.5),  // grass, mud
        5 => (0.01, 0.3),       // gravel
        13 => (0.002, 0.5),     // metal
        14 => (0.006, 0.25),    // wood
        15 => (0.008, 0.2),     // small cobblestone
        16 => (0.005, 0.25),    // interlocking pavers
        _ => (0.002, 0.6),      // asphalt, snow, slush, ice, water
    }
}
/// Points the contact patch is sampled at.
const PATCH_POINTS: usize = 5;

/// A decoded `.surf` height map.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfMap {
    width: usize,
    height: usize,
    /// Heights in metres, row by row from the top of the image.
    heights: Vec<f32>,
}

impl SurfMap {
    /// Decodes a `.surf` file (a BMP or another image format): the red channel, mid-grey level.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let img = image::load_from_memory(bytes).ok()?.to_rgb8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        if w == 0 || h == 0 {
            return None;
        }
        let heights = img
            .pixels()
            .map(|p| (f32::from(p.0[0]) - 127.5) / 127.5 * SURF_AMPLITUDE_M)
            .collect();
        Some(Self {
            width: w,
            height: h,
            heights,
        })
    }

    /// A synthetic height map of random bumps up to `amplitude_m` (deterministic for `seed`),
    /// sampled in metres when its strip has a [`TexStrip::period_m`].
    #[must_use]
    pub fn noise(seed: u64, amplitude_m: f32) -> Self {
        let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
        let heights = (0..NOISE_CELLS * NOISE_CELLS)
            .map(|_| {
                // xorshift64*
                state ^= state >> 12;
                state ^= state << 25;
                state ^= state >> 27;
                let r =
                    (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / (1u64 << 24) as f32;
                (r * 2.0 - 1.0) * amplitude_m
            })
            .collect();
        Self {
            width: NOISE_CELLS,
            height: NOISE_CELLS,
            heights,
        }
    }

    /// The synthetic map of a surface type ([`roughness_of`]) and the length it repeats over (m).
    #[must_use]
    pub fn synthetic(surface: u32) -> (Self, f32) {
        let (amplitude, bump) = roughness_of(surface);
        (
            Self::noise(u64::from(surface) + 1, amplitude),
            bump * NOISE_CELLS as f32,
        )
    }

    /// A map from heights (tests).
    #[must_use]
    pub fn from_heights(width: usize, height: usize, heights: Vec<f32>) -> Self {
        Self {
            width,
            height,
            heights,
        }
    }

    /// Height at texture coordinates `(u, v)` (repeating), bilinear between the pixel centres.
    #[must_use]
    pub fn height_at(&self, u: f32, v: f32) -> f32 {
        let fx = u.rem_euclid(1.0) * self.width as f32 - 0.5;
        let fy = v.rem_euclid(1.0) * self.height as f32 - 0.5;
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let wrap = |i: f32, n: usize| (i as i64).rem_euclid(n as i64) as usize;
        let at =
            |x: f32, y: f32| self.heights[wrap(y, self.height) * self.width + wrap(x, self.width)];
        let top = at(x0, y0) * (1.0 - tx) + at(x0 + 1.0, y0) * tx;
        let bottom = at(x0, y0 + 1.0) * (1.0 - tx) + at(x0 + 1.0, y0 + 1.0) * tx;
        top * (1.0 - ty) + bottom * ty
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> Arc<SurfMap> {
        // 2 × 2: +2 cm and −2 cm alternating.
        Arc::new(SurfMap::from_heights(2, 2, vec![0.02, -0.02, -0.02, 0.02]))
    }

    #[test]
    fn a_bmp_red_channel_decodes_to_plus_minus_two_centimetres() {
        let mut img = image::RgbImage::new(2, 1);
        img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        img.put_pixel(1, 0, image::Rgb([0, 255, 255]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        img.write_to(&mut bytes, image::ImageFormat::Bmp)
            .expect("bmp");
        let map = SurfMap::decode(bytes.get_ref()).expect("decodes");
        assert!((map.height_at(0.25, 0.5) - 0.02).abs() < 1e-4);
        assert!((map.height_at(0.75, 0.5) + 0.02).abs() < 1e-4);
    }

    #[test]
    fn the_map_repeats_and_interpolates() {
        let m = checker();
        assert!((m.height_at(0.25, 0.25) - 0.02).abs() < 1e-6);
        assert!((m.height_at(1.25, -0.75) - 0.02).abs() < 1e-6);
        // Between two pixels of opposite height: level.
        assert!(m.height_at(0.5, 0.25).abs() < 1e-6);
    }

    #[test]
    fn the_top_strip_at_road_level_gives_the_unevenness() {
        let strip = TexStrip {
            x0: -3.0,
            x1: 3.0,
            z0: 0.0,
            z1: 0.0,
            u0: 0.0,
            u1: 1.0,
            vs0: 0.0,
            vs1: 0.0,
            map: Arc::new(SurfMap::from_heights(1, 1, vec![0.015])),
            period_m: None,
        };
        let kerb = TexStrip {
            x0: 3.0,
            x1: 3.1,
            z0: 0.0,
            z1: 0.15,
            ..strip.clone()
        };
        let strips = vec![strip, kerb];
        assert!((unevenness_at(&strips, 0.0, 10.0, 0.0) - 0.015).abs() < 1e-6);
        // Off the road surface (a pavement 15 cm up, no strip there at that height): level.
        assert_eq!(unevenness_at(&strips, 5.0, 10.0, 0.15), 0.0);
    }

    #[test]
    fn the_contact_patch_smooths_ridges_shorter_than_the_tyre() {
        // Ridges about 1.1 cm apart along the road (v per metre 45 over a 2-pixel map).
        let strip = TexStrip {
            x0: -1.0,
            x1: 1.0,
            z0: 0.0,
            z1: 0.0,
            u0: 0.25,
            u1: 0.25,
            vs0: 45.0,
            vs1: 45.0,
            map: checker(),
            period_m: None,
        };
        let raw = strip.map.height_at(0.25, 0.25).abs();
        let felt = strip
            .unevenness(0.0, 0.25 / 45.0)
            .expect("on the strip")
            .abs();
        assert!(felt < raw * 0.5, "felt {felt} raw {raw}");
    }

    #[test]
    fn surface_types_without_a_surf_get_a_synthetic_roughness() {
        let (cobbles, _) = roughness_of(2);
        let (asphalt, _) = roughness_of(0);
        assert!(cobbles > 4.0 * asphalt);
        let map = SurfMap::noise(7, cobbles);
        assert!(map.heights.iter().all(|h| h.abs() <= cobbles + 1e-6));
        assert!(map.heights.iter().any(|h| h.abs() > cobbles * 0.5));
        assert_eq!(
            map,
            SurfMap::noise(7, cobbles),
            "the same for the same seed"
        );
        // Laid in metres: the same place gives the same height wherever the texture repeats.
        let strip = TexStrip {
            x0: -2.0,
            x1: 2.0,
            z0: 0.0,
            z1: 0.0,
            u0: 0.0,
            u1: 0.0,
            vs0: 0.0,
            vs1: 0.0,
            map: Arc::new(map),
            period_m: Some(8.0),
        };
        let a = strip.unevenness(0.3, 5.0).expect("on the strip");
        let b = strip.unevenness(0.3, 13.0).expect("on the strip");
        assert!((a - b).abs() < 1e-6);
    }
}
