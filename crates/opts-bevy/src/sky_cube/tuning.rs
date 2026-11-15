//! The tunable constants of the sky cube: its size, intensities, regeneration limits and the
//! sky's radiance shape.

/// Edge of the source cube in texels (a power of two; Bevy filters it into its own maps).
pub const CUBE_SIZE: u32 = 64;
/// Intensity (cd/m2) at full daylight and clear sky. The cube's diffuse part also lights every
/// standard material, so this is a compromise between visible glass reflections and the extra
/// ambient light in shadow (the research prototype saw a washed-out image at 3000).
pub const BASE_INTENSITY: f32 = 800.0;
/// Shortest time between two regenerations of the cube (real seconds).
pub const REGEN_MIN_INTERVAL_S: f64 = 2.0;
/// The sun must have moved by more than this (degrees of elevation) to regenerate.
pub const REGEN_SUN_DELTA_DEG: f32 = 2.0;
/// A weather value (attenuations, fog share) must have moved by more than this to regenerate.
pub const WEATHER_EPS: f32 = 0.02;
/// Radiance of the sun's disc at full direct light, relative to the sky colours (which are
/// `0..=1`). The disc is a small, very bright spot in the specular reflection.
pub const SUN_SPOT_RADIANCE: f32 = 20.0;
/// Sharpness of the sun spot (`cos^n`).
pub const SUN_SPOT_POWER: f32 = 400.0;
/// Share of the horizon colour the ground below the horizon reflects.
pub const GROUND_SHARE: f32 = 0.3;
/// Least [`exposure`](super::exposure) at night (a trace of moonlight; the sky texture is dark
/// then, too).
pub const NIGHT_EXPOSURE: f32 = 0.03;
