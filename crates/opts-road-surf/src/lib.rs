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

mod map;
mod strip;

pub use map::SurfMap;
pub use strip::{plain_unevenness, strips_of, unevenness_at, TexStrip};

/// Height of the brightest red value (m).
pub const SURF_AMPLITUDE_M: f32 = 0.02;
/// Length of the tyre's contact patch along the road (m): the height map is averaged over it, as
/// a tyre does not follow ridges shorter than its footprint.
pub const CONTACT_PATCH_M: f32 = 0.12;

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

#[cfg(test)]
mod tests;
