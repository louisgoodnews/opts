//! The shared sky cube map for glass reflections (`docs/src/architecture/reflections-research.md`
//! section 7 step B, `lighting-system.md` section 9).
//!
//! * The game writes [`SkyCubeInput`] (horizon and zenith colours of the session's sky, the sun,
//!   the weather); this module owns **one** cube image and, while
//!   [`LightQuality::glass_reflections`] is on, a [`GeneratedEnvironmentMapLight`] on the
//!   [`MainCamera`] that points at it. Bevy filters the cube into diffuse and specular maps every
//!   frame (that is the 1.2 ms of the budget; the source is read, nothing is cached), so the
//!   regeneration of the *source* below is cheap CPU work, not GPU work.
//! * [`should_regenerate`] is the policy: at most once per [`REGEN_MIN_INTERVAL_S`] real seconds,
//!   and only when the environment revision changed, the sun moved by more than
//!   [`REGEN_SUN_DELTA_DEG`], or the weather moved by more than [`WEATHER_EPS`]. The first cube is
//!   built at once.
//! * The intensity follows [`exposure`] every frame (no regeneration): daylight times the
//!   weather's ambient attenuation, so night does not glow.
//! * With `glass_reflections` off the components are removed and the image freed: the output is
//!   today's.
//!
//! Mirror cameras do not get the light (a second filter pass costs about 1 ms per camera).
//!
//! The parts live in `sky_cube/`: the tunables, the input, the policy, the texels and the sync.

mod input;
mod policy;
mod sync;
mod texels;
mod tuning;

pub use input::{RegenSnapshot, SkyCubeInput};
pub use policy::{exposure, should_regenerate};
pub use sync::{SkyCubeApplied, SkyCubeParams, SkyCubePlugin, SkyCubeState, sync_sky_cube};
pub use texels::{cube_data, face_direction, sky_radiance};
pub use tuning::{
    BASE_INTENSITY, CUBE_SIZE, GROUND_SHARE, NIGHT_EXPOSURE, REGEN_MIN_INTERVAL_S,
    REGEN_SUN_DELTA_DEG, SUN_SPOT_POWER, SUN_SPOT_RADIANCE, WEATHER_EPS,
};

#[cfg(test)]
mod tests;
