//! Ephemeris: the Julian day, the moon's and the sun's ecliptic longitudes, and the lunar elongation.

use crate::environment::CalendarDate;

/// Mean length of the synodic month in days (used for the age only).
pub const SYNODIC_MONTH_DAYS: f64 = 29.530_588_853;

/// Julian day of a date and a time of day (seconds, read as UT).
#[must_use]
pub fn julian_day(date: CalendarDate, seconds_of_day: f64) -> f64 {
    date.days_since_epoch() as f64 + 2_440_587.5 + seconds_of_day / 86_400.0
}

pub(super) fn norm360(x: f64) -> f64 {
    x.rem_euclid(360.0)
}

/// Julian centuries since J2000.
pub(super) fn centuries(jd: f64) -> f64 {
    (jd - 2_451_545.0) / 36_525.0
}

/// Ecliptic longitude and latitude of the moon in degrees.
pub(super) fn moon_ecliptic(t: f64) -> (f64, f64) {
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
pub(super) fn sun_longitude(t: f64) -> f64 {
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
