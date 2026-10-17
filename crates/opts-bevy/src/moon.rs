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
mod tests {
    use super::*;

    /// Julian day of a UTC date and time.
    fn jd_of(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> f64 {
        julian_day(
            CalendarDate::new(y, mo, d),
            f64::from(h) * 3600.0 + f64::from(mi) * 60.0,
        )
    }

    /// Solves for the instant near `reference` at which the elongation passes `target` degrees.
    fn solve(reference: f64, target: f64) -> f64 {
        let f = |jd: f64| {
            let mut d = lunar_elongation_deg(jd) - target;
            while d > 180.0 {
                d -= 360.0;
            }
            while d < -180.0 {
                d += 360.0;
            }
            d
        };
        let (mut lo, mut hi) = (reference - 1.0, reference + 1.0);
        assert!(f(lo) < 0.0 && f(hi) > 0.0, "bracket");
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if f(mid) < 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }

    #[test]
    fn eleven_principal_phases_are_within_ten_minutes() {
        // (year, month, day, hour, minute UTC, target elongation) from published tables.
        let table = [
            (2000, 1, 6, 18, 14, 0.0),
            (2000, 1, 21, 4, 40, 180.0),
            (2024, 1, 11, 11, 57, 0.0),
            (2024, 1, 18, 3, 53, 90.0),
            (2024, 1, 25, 17, 54, 180.0),
            (2024, 4, 8, 18, 21, 0.0),
            (2024, 6, 22, 1, 8, 180.0),
            (2022, 11, 8, 11, 2, 180.0),
            (2025, 3, 14, 6, 55, 180.0),
            (2025, 9, 21, 19, 54, 0.0),
            (2024, 1, 4, 3, 30, 270.0),
        ];
        for (y, mo, d, h, mi, target) in table {
            let reference = jd_of(y, mo, d, h, mi);
            // A target of 0 sits at the 360 wrap: solve on the signed difference.
            let found = solve(reference, target);
            let minutes = (found - reference) * 1440.0;
            assert!(
                minutes.abs() <= 10.0,
                "{y}-{mo:02}-{d:02} {h:02}:{mi:02}: {minutes:+.1} min"
            );
        }
    }

    #[test]
    fn the_fraction_is_zero_at_new_one_at_full_half_at_quarters() {
        let at = |y, mo, d, h, mi, target| {
            let jd = solve(jd_of(y, mo, d, h, mi), target);
            let date = CalendarDate::from_days_since_epoch((jd - 2_440_587.5).floor() as i64);
            let secs = (jd - 2_440_587.5 - (jd - 2_440_587.5).floor()) * 86_400.0;
            moon_state(date, secs, 50.0)
        };
        // The moon's ecliptic latitude (up to 5°) keeps the new and full fractions a hair off 0 / 1.
        assert!(at(2024, 1, 11, 11, 57, 0.0).illuminated < 3e-3);
        assert!(at(2024, 1, 25, 17, 54, 180.0).illuminated > 0.997);
        let q = at(2024, 1, 18, 3, 53, 90.0);
        assert!((q.illuminated - 0.5).abs() < 1e-3 && q.waxing);
        let q = at(2024, 1, 4, 3, 30, 270.0);
        assert!((q.illuminated - 0.5).abs() < 1e-3 && !q.waxing);
    }

    #[test]
    fn the_fraction_waxes_for_half_a_month_and_repeats() {
        let date = CalendarDate::new(2026, 10, 1);
        let k = |days: f64| {
            let jd = julian_day(date, 0.0) + days;
            let d = CalendarDate::from_days_since_epoch((jd - 2_440_587.5).floor() as i64);
            let s = (jd - 2_440_587.5).rem_euclid(1.0) * 86_400.0;
            moon_state(d, s, 50.0)
        };
        // Day-to-day fractions rise strictly through the waxing half.
        let mut prev: Option<f32> = None;
        let mut waxing_days = 0;
        for i in 0..40 {
            let s = k(f64::from(i));
            if s.waxing {
                if let Some(p) = prev {
                    assert!(s.illuminated > p - 1e-6);
                }
                waxing_days += 1;
                prev = Some(s.illuminated);
            } else {
                prev = None;
            }
        }
        assert!((13..=17).contains(&waxing_days), "{waxing_days}");
        // A synodic month later the phase is almost the same.
        let (a, b) = (k(3.0), k(3.0 + SYNODIC_MONTH_DAYS));
        assert!((a.illuminated - b.illuminated).abs() < 0.05);
    }

    #[test]
    fn the_moon_is_with_the_sun_at_new_and_opposite_at_full() {
        let sun_at = |date: CalendarDate, secs: f64| {
            crate::environment::sun_position(
                (secs / 3600.0) as f32,
                date.day_of_year() as f32,
                50.0,
            )
        };
        for (y, mo, d, h, mi, target, near) in [
            (2024, 1, 11, 11, 57, 0.0, true),
            (2024, 1, 25, 17, 54, 180.0, false),
        ] {
            let jd = solve(jd_of(y, mo, d, h, mi), target);
            let date = CalendarDate::from_days_since_epoch((jd - 2_440_587.5).floor() as i64);
            let secs = (jd - 2_440_587.5).rem_euclid(1.0) * 86_400.0;
            let moon = moon_state(date, secs, 50.0);
            let (sun_el, sun_az) = sun_at(date, secs);
            if near {
                assert!((moon.elevation_deg - sun_el).abs() < 6.0);
                let mut daz = (moon.azimuth_deg - sun_az).abs();
                if daz > 180.0 {
                    daz = 360.0 - daz;
                }
                assert!(daz < 15.0 || sun_el.abs() > 60.0, "{daz}");
            } else {
                assert!((moon.elevation_deg + sun_el).abs() < 8.0, "{sun_el}");
            }
        }
    }

    #[test]
    fn sky_position_sample_of_the_design() {
        let date = CalendarDate::new(2026, 10, 17);
        let night = moon_state(date, 22.0 * 3600.0, 50.0);
        assert!((night.elevation_deg + 4.9).abs() < 1.5, "{night:?}");
        assert!((night.azimuth_deg - 235.0).abs() < 3.0, "{night:?}");
        assert!((night.illuminated - 0.43).abs() < 0.03, "{night:?}");
        assert!(night.waxing);
        let eve = moon_state(date, 18.0 * 3600.0, 50.0);
        assert!((eve.elevation_deg - 13.6).abs() < 1.5, "{eve:?}");
        assert!((eve.azimuth_deg - 186.0).abs() < 3.0, "{eve:?}");
    }

    #[test]
    fn angles_are_in_range_and_the_model_is_deterministic() {
        for i in 0..200 {
            let date = CalendarDate::new(2026, 1, 1).add_days(i * 7);
            let secs = f64::from(i as i32 % 24) * 3600.0;
            let a = moon_state(date, secs, 50.0);
            assert_eq!(a, moon_state(date, secs, 50.0));
            assert!((0.0..360.0).contains(&a.azimuth_deg));
            assert!((-90.0..=90.0).contains(&a.elevation_deg));
            assert!((0.0..=1.0).contains(&a.illuminated));
            assert!((0.0..=180.0).contains(&a.phase_angle_deg));
            assert!((-180.0..=180.0).contains(&a.bright_limb_deg));
            assert!((0.0..30.0).contains(&a.age_days));
        }
    }

    #[test]
    fn the_bright_limb_points_towards_the_sun_in_the_sky() {
        // A waxing evening moon in the south-west is lit on its right (sun below the horizon to
        // the west): the limb angle is near 0 (screen right) to within a quarter turn.
        let s = moon_state(CalendarDate::new(2026, 10, 17), 18.0 * 3600.0, 50.0);
        assert!(s.waxing);
        assert!(s.bright_limb_deg.abs() < 60.0, "{s:?}");
    }

    #[test]
    fn allen_phase_function_matches_the_published_values() {
        assert!((allen_phase_function(0.0) - 1.0).abs() < 1e-6);
        assert!((allen_phase_function(60.0) - 0.23).abs() < 0.03);
        assert!((allen_phase_function(90.0) - 0.09).abs() < 0.02);
        assert!((allen_phase_function(120.0) - 0.026).abs() < 0.01);
    }

    fn state(el: f32, k: f32) -> MoonState {
        MoonState {
            elevation_deg: el,
            azimuth_deg: 180.0,
            elongation_deg: 0.0,
            illuminated: k,
            phase_angle_deg: 180.0 - (1.0 - 2.0 * k).acos().to_degrees(),
            bright_limb_deg: 0.0,
            waxing: true,
            age_days: 0.0,
        }
    }

    #[test]
    fn moonlight_is_zero_below_the_horizon_by_day_and_at_new_moon() {
        assert_eq!(moon_illuminance(&state(-5.0, 1.0), -30.0, 1.0), 0.0);
        assert_eq!(moon_illuminance(&state(40.0, 1.0), 20.0, 1.0), 0.0);
        assert_eq!(moon_illuminance(&state(40.0, 0.0), -30.0, 1.0), 0.0);
        assert_eq!(moon_illuminance(&state(40.0, 1.0), -30.0, 0.0), 0.0);
        let full = moon_illuminance(&state(60.0, 1.0), -30.0, 1.0);
        assert!(full > 0.8 * MOON_PEAK_LX && full <= MOON_PEAK_LX, "{full}");
    }

    #[test]
    fn moonlight_grows_with_phase_and_height() {
        let mut prev = 0.0;
        for k in [0.05, 0.2, 0.4, 0.6, 0.8, 1.0] {
            let e = moon_illuminance(&state(50.0, k), -30.0, 1.0);
            assert!(e > prev, "k {k}: {e} <= {prev}");
            prev = e;
        }
        let mut prev = 0.0;
        for el in [0.0, 3.0, 10.0, 30.0, 60.0, 90.0] {
            let e = moon_illuminance(&state(el, 1.0), -30.0, 1.0);
            assert!(e >= prev, "el {el}");
            prev = e;
        }
        // A half moon gives about 0.3 of the full one (the gamma-compressed Allen function).
        let half = moon_illuminance(&state(50.0, 0.5), -30.0, 1.0);
        let full = moon_illuminance(&state(50.0, 1.0), -30.0, 1.0);
        assert!((half / full - 0.3).abs() < 0.06, "{}", half / full);
    }
}
