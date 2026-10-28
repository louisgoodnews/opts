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

use crate::environment::CalendarDate;

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

/// Mean length of the synodic month in days (used for the age only).
pub const SYNODIC_MONTH_DAYS: f64 = 29.530_588_853;

/// Julian day of a date and a time of day (seconds, read as UT).
#[must_use]
pub fn julian_day(date: CalendarDate, seconds_of_day: f64) -> f64 {
    date.days_since_epoch() as f64 + 2_440_587.5 + seconds_of_day / 86_400.0
}

fn norm360(x: f64) -> f64 {
    x.rem_euclid(360.0)
}

/// Julian centuries since J2000.
fn centuries(jd: f64) -> f64 {
    (jd - 2_451_545.0) / 36_525.0
}

/// Ecliptic longitude and latitude of the moon in degrees.
fn moon_ecliptic(t: f64) -> (f64, f64) {
    let lp = 218.316_447_7 + 481_267.881_234_21 * t;
    let d = (297.850_192_1 + 445_267.111_403_4 * t).to_radians();
    let m = (357.529_109_2 + 35_999.050_290_9 * t).to_radians();
    let mp = (134.963_396_4 + 477_198.867_505_5 * t).to_radians();
    let f = (93.272_095_0 + 483_202.017_523_3 * t).to_radians();
    let lon = lp
        + 6.288_774 * mp.sin()
        + 1.274_027 * (2.0 * d - mp).sin()
        + 0.658_314 * (2.0 * d).sin()
        + 0.213_618 * (2.0 * mp).sin()
        - 0.185_116 * m.sin()
        - 0.114_332 * (2.0 * f).sin()
        + 0.058_793 * (2.0 * d - 2.0 * mp).sin()
        + 0.057_066 * (2.0 * d - m - mp).sin()
        + 0.053_322 * (2.0 * d + mp).sin()
        + 0.045_758 * (2.0 * d - m).sin()
        - 0.040_923 * (m - mp).sin()
        - 0.034_720 * d.sin()
        - 0.030_383 * (m + mp).sin();
    let lat = 5.128_122 * f.sin()
        + 0.280_602 * (mp + f).sin()
        + 0.277_693 * (mp - f).sin()
        + 0.173_237 * (2.0 * d - f).sin()
        + 0.055_413 * (2.0 * d - mp + f).sin()
        + 0.046_271 * (2.0 * d - mp - f).sin()
        + 0.032_573 * (2.0 * d + f).sin();
    (norm360(lon), lat)
}

/// Ecliptic longitude of the sun in degrees.
fn sun_longitude(t: f64) -> f64 {
    let l0 = 280.466_46 + 36_000.769_83 * t;
    let ms = (357.529_11 + 35_999.050_29 * t).to_radians();
    norm360(l0 + 1.914_602 * ms.sin() + 0.019_993 * (2.0 * ms).sin())
}

/// Longitude of the moon minus longitude of the sun, `0..360` degrees (0 new, 90 first quarter,
/// 180 full, 270 last quarter). Rises by about 12.2° a day.
#[must_use]
pub fn lunar_elongation_deg(jd: f64) -> f64 {
    let t = centuries(jd);
    norm360(moon_ecliptic(t).0 - sun_longitude(t))
}

/// Right ascension and declination (degrees) of an ecliptic position.
fn equatorial(lon_deg: f64, lat_deg: f64, t: f64) -> (f64, f64) {
    let eps = (23.439_291 - 0.013_004_2 * t).to_radians();
    let (lon, lat) = (lon_deg.to_radians(), lat_deg.to_radians());
    let ra = (lon.sin() * eps.cos() - lat.tan() * eps.sin()).atan2(lon.cos());
    let dec = (lat.sin() * eps.cos() + lat.cos() * eps.sin() * lon.sin()).clamp(-1.0, 1.0);
    (norm360(ra.to_degrees()), dec.asin().to_degrees())
}

/// Elevation and azimuth (degrees, azimuth clockwise from north) for an hour angle, declination
/// and latitude, with the formulas of [`crate::environment::sun_position`].
fn horizontal(hour_angle_deg: f64, dec_deg: f64, lat_deg: f64) -> (f64, f64) {
    let (h, dec, lat) = (
        hour_angle_deg.to_radians(),
        dec_deg.to_radians(),
        lat_deg.to_radians(),
    );
    let sin_el = lat.sin() * dec.sin() + lat.cos() * dec.cos() * h.cos();
    let az = h.sin().atan2(h.cos() * lat.sin() - dec.tan() * lat.cos()) + std::f64::consts::PI;
    (
        sin_el.clamp(-1.0, 1.0).asin().to_degrees(),
        norm360(az.to_degrees()),
    )
}

/// Unit vector (east, north, up) of an elevation/azimuth.
fn unit(el_deg: f64, az_deg: f64) -> [f64; 3] {
    let (el, az) = (el_deg.to_radians(), az_deg.to_radians());
    [el.cos() * az.sin(), el.cos() * az.cos(), el.sin()]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The moon for a date, a time of day (seconds, read as UT and as local solar time) and a
/// latitude.
#[must_use]
pub fn moon_state(date: CalendarDate, seconds_of_day: f64, latitude_deg: f32) -> MoonState {
    let jd = julian_day(date, seconds_of_day);
    let t = centuries(jd);
    let (moon_lon, moon_lat) = moon_ecliptic(t);
    let sun_lon = sun_longitude(t);
    let (ra_moon, dec_moon) = equatorial(moon_lon, moon_lat, t);
    let (ra_sun, dec_sun) = equatorial(sun_lon, 0.0, t);
    let lat = f64::from(latitude_deg);
    let solar_ha = 15.0 * (seconds_of_day / 3600.0 - 12.0);
    let (el, az) = horizontal(solar_ha + ra_sun - ra_moon, dec_moon, lat);
    let (sun_el, sun_az) = horizontal(solar_ha, dec_sun, lat);

    let dl = (moon_lon - sun_lon).to_radians();
    let cos_psi = (moon_lat.to_radians().cos() * dl.cos()).clamp(-1.0, 1.0);
    let elongation = cos_psi.acos().to_degrees();
    let illuminated = (1.0 - cos_psi) / 2.0;
    let waxing = norm360(moon_lon - sun_lon) < 180.0;

    // Bright limb in the screen frame of an observer looking at the moon with the horizon level.
    let (m, s) = (unit(el, az), unit(sun_el, sun_az));
    let up = [0.0, 0.0, 1.0];
    let u_raw = [
        up[0] - m[0] * m[2],
        up[1] - m[1] * m[2],
        up[2] - m[2] * m[2],
    ];
    let u_len = dot(u_raw, u_raw).sqrt();
    let limb = if u_len < 1e-6 {
        0.0
    } else {
        let u = u_raw.map(|x| x / u_len);
        let right = cross(m, u);
        let s_perp = [
            s[0] - m[0] * dot(s, m),
            s[1] - m[1] * dot(s, m),
            s[2] - m[2] * dot(s, m),
        ];
        dot(s_perp, u).atan2(dot(s_perp, right)).to_degrees()
    };

    let cycle = norm360(moon_lon - sun_lon);
    MoonState {
        elevation_deg: el as f32,
        azimuth_deg: az as f32,
        elongation_deg: elongation as f32,
        illuminated: illuminated as f32,
        phase_angle_deg: (180.0 - elongation) as f32,
        bright_limb_deg: limb as f32,
        waxing,
        age_days: (cycle / 360.0 * SYNODIC_MONTH_DAYS) as f32,
    }
}

/// Allen's lunar phase function: the moon's brightness relative to the full moon at a phase
/// angle (1 at 0°, 0.23 at 60°, 0.09 at 90°, 0.026 at 120°).
#[must_use]
pub fn allen_phase_function(phase_angle_deg: f32) -> f32 {
    let a = phase_angle_deg.clamp(0.0, 180.0);
    10f32.powf(-0.4 * (0.026 * a + 4e-9 * a.powi(4)))
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Strength of the moon from its own height: nothing below the horizon, fading in to
/// [`MOON_FADE_ABOVE_DEG`], dimmer when low (extinction).
#[must_use]
pub fn moon_height_factor(moon_elevation_deg: f32) -> f32 {
    smoothstep(MOON_FADE_BELOW_DEG, MOON_FADE_ABOVE_DEG, moon_elevation_deg)
        * (0.4 + 0.6 * moon_elevation_deg.to_radians().sin()).max(0.0)
}

/// How much of the night is left: 1 with the sun below [`MOON_SUN_FULL_DEG`], 0 above
/// [`MOON_SUN_GONE_DEG`] (the moon adds no light in daylight).
#[must_use]
pub fn moon_night_factor(sun_elevation_deg: f32) -> f32 {
    1.0 - smoothstep(MOON_SUN_FULL_DEG, MOON_SUN_GONE_DEG, sun_elevation_deg)
}

/// Illuminance of the moonlight in lx:
/// `900 × Allen(α)^γ × h(el) × w(sun) × weather`, and exactly 0 at the new moon.
///
/// `weather_moon` is [`crate::environment::LightAttenuation::moon`].
#[must_use]
pub fn moon_illuminance(moon: &MoonState, sun_elevation_deg: f32, weather_moon: f32) -> f32 {
    let phase = allen_phase_function(moon.phase_angle_deg).powf(MOON_PHASE_GAMMA);
    let new_gate = smoothstep(0.0, MOON_NEW_FRACTION, moon.illuminated);
    MOON_PEAK_LX
        * phase
        * new_gate
        * moon_height_factor(moon.elevation_deg)
        * moon_night_factor(sun_elevation_deg)
        * weather_moon.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests;
