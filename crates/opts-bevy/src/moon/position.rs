//! The moon's place in the sky (elevation, azimuth, bright limb) and the state of its disc.

use super::ephemeris::{
    centuries, julian_day, moon_ecliptic, norm360, sun_longitude, SYNODIC_MONTH_DAYS,
};
use super::MoonState;
use crate::environment::CalendarDate;

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
