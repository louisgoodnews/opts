//! Proleptic Gregorian calendar date, solar position, and weather light attenuation.

mod date;
mod light;

pub use date::{days_in_month, is_leap_year, CalendarDate};
pub use light::{light_attenuation, LightAttenuation, WeatherSky};

use super::*;

/// Latitude of the built-in sun formula. OMSI maps carry no geographic position, so the sun path
/// assumes central Europe (where OMSI's maps lie) and treats clock time as local solar time.
pub const LATITUDE_DEG: f32 = 50.0;

/// Sun elevation below which it is night: AI and player vehicle lights, the instruments' night
/// palette, `NightlightA` (3° above the horizon).
pub const NIGHT_SUN_ALT_DEG: f32 = 3.0;

/// Seconds of one day.
pub const DAY_S: f64 = 86_400.0;

/// Sun elevation and azimuth in degrees (azimuth clockwise from north) for a local solar hour,
/// day of the year and latitude (declination approximation, good to about a degree).
///
/// The one sun formula of OPTS: the sky, the scenery clock and the session environment use it.
#[must_use]
pub fn sun_position(hour: f32, day_of_year: f32, latitude_deg: f32) -> (f32, f32) {
    let lat = latitude_deg.to_radians();
    let decl =
        (-23.44f32).to_radians() * (std::f32::consts::TAU * (day_of_year + 10.0) / 365.0).cos();
    let h = (15.0 * (hour - 12.0)).to_radians();
    let sin_el = lat.sin() * decl.sin() + lat.cos() * decl.cos() * h.cos();
    let az = h.sin().atan2(h.cos() * lat.sin() - decl.tan() * lat.cos()) + std::f32::consts::PI;
    (
        sin_el.clamp(-1.0, 1.0).asin().to_degrees(),
        az.to_degrees().rem_euclid(360.0),
    )
}
