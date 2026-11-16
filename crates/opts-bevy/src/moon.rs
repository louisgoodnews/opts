//! The moon: position, phase and the light it gives (pure functions, no Bevy types).
//!
//! Design: `docs/src/architecture/lighting-system.md` section 5.2 and 5.3. The moon's ecliptic
//! longitude and latitude come from the 13 + 7 principal terms of the lunar series (after Meeus,
//! *Astronomical Algorithms*, ch. 47, low-precision form); the sun's longitude from the usual
//! low-precision solar series. The moon is placed **relative to the sun**: its hour angle is the
//! app's local-solar-time hour angle (`15° × (hour − 12)`, the same convention as
//! [`crate::environment::sun_position`]) plus the difference of the two right ascensions, so the
//! phase and the moon's position against the sun are exact and only the app's clock convention
//! enters. The session clock is treated as UT (the map has no longitude).
//!
//! # Accuracy
//!
//! * Time of the 11 principal phases 2000-2025 within 5 minutes (unit test: 10 minutes), i.e. the
//!   elongation is right to about 0.04°; the illuminated fraction is 0 / 1 at new / full moon and
//!   0.5 at the quarters at those instants.
//! * Ecliptic longitude of the moon about 0.3° (truncated series); parallax (up to 0.95°),
//!   eclipses, nutation and the time-zone ambiguity (up to 0.5° of lunar motion per hour) are not
//!   modelled. Fine for a disc whose artistic size is 7.7°.
//! * The mean synodic month would be off by up to 16.6 hours and is not used.

mod ephemeris;
mod light;
mod position;

#[cfg(test)]
mod tests;

pub use ephemeris::{julian_day, lunar_elongation_deg, SYNODIC_MONTH_DAYS};
pub use light::{allen_phase_function, moon_height_factor, moon_illuminance, moon_night_factor};
pub use position::moon_state;

/// Peak illuminance of the full moon in game units (lx). The physical ratio full moon to sun is
/// about 1 : 400 000, which would be invisible; 900 lx is the value the fallback light used for
/// its night.
pub const MOON_PEAK_LX: f32 = 900.0;
/// Exponent applied to Allen's lunar phase function: a perceptual compression, so a half moon
/// gives about 0.3 of full instead of 0.09.
pub const MOON_PHASE_GAMMA: f32 = 0.5;
/// Moon elevation (degrees) below which it gives no light.
pub const MOON_FADE_BELOW_DEG: f32 = -1.0;
/// Moon elevation (degrees) above which there is no low-moon fade-in any more.
pub const MOON_FADE_ABOVE_DEG: f32 = 8.0;
/// Sun elevation (degrees) at and below which the moon is at full strength.
pub const MOON_SUN_FULL_DEG: f32 = -4.0;
/// Sun elevation (degrees) at and above which the moonlight is gone (daylight).
pub const MOON_SUN_GONE_DEG: f32 = 6.0;
/// Illuminated fraction below which the moonlight fades to exactly zero (the new moon).
pub const MOON_NEW_FRACTION: f32 = 0.02;
/// Colour of moonlight (linear-ish sRGB components; blue-shifted perception, an OPTS addition).
pub const MOON_COLOR: [f32; 3] = [0.72, 0.80, 1.00];

/// The state of the moon for a date and time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoonState {
    /// Elevation in degrees, positive above the horizon.
    pub elevation_deg: f32,
    /// Azimuth in degrees clockwise from north.
    pub azimuth_deg: f32,
    /// Angular distance from the sun in degrees (0 new, 180 full).
    pub elongation_deg: f32,
    /// Illuminated fraction of the disc, `0..=1`.
    pub illuminated: f32,
    /// Phase angle in degrees (0 full, 180 new).
    pub phase_angle_deg: f32,
    /// Direction of the sun as seen from the moon in the plane of the disc, in degrees,
    /// counter-clockwise from screen-right for an observer looking at the moon with the horizon
    /// level (the middle of the bright limb points this way).
    pub bright_limb_deg: f32,
    /// The moon is waxing (between new and full).
    pub waxing: bool,
    /// Age in days since the new moon (`0..29.53`, from the elongation: not the mean month).
    pub age_days: f32,
}
