//! The session clock and environment values scenery scripts read (`Time`, the date, `SunAlt`,
//! `NightlightA`, …), plus the per-object variable references the runtime resolves once.

use bevy::prelude::*;

/// Sun elevation below which night lighting is on (the threshold the AI vehicles use for their
/// lights).
pub const NIGHT_SUN_ALT_DEG: f32 = 3.0;

/// Latitude for the built-in sun formula (central Germany, like the sky).
const LATITUDE_DEG: f32 = 50.0;

/// Clock and environment of the scenery runtime.
///
/// The game copies the session clock into it every frame while `follow_session` is set. A host
/// that drives time itself (the scenery benchmark) clears `follow_session`, sets the fields and
/// moves time with [`super::advance_scenery`].
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct SceneryClock {
    /// Seconds since midnight of the session's start day (not wrapped: light programs keep
    /// running across midnight like the AI traffic's clock).
    pub seconds_of_day: f64,
    /// Calendar year.
    pub year: i32,
    /// Month, 1–12.
    pub month: u32,
    /// Day of the month, 1–31.
    pub day: u32,
    /// Sun elevation in degrees.
    pub sun_alt_deg: f32,
    /// Night lighting is on (`NightlightA`, variable-less light maps).
    pub night: bool,
    /// Outside temperature in °C (`Weather_Temperature`).
    pub temperature_c: f32,
    /// The game overwrites the clock from the session every frame.
    pub follow_session: bool,
}

impl Default for SceneryClock {
    fn default() -> Self {
        let mut c = Self::at(2026, 6, 29, 12.0 * 3600.0);
        c.follow_session = true;
        c
    }
}

impl SceneryClock {
    /// A host-driven clock at the given date and time; sun and night follow from them.
    #[must_use]
    pub fn at(year: i32, month: u32, day: u32, seconds_of_day: f64) -> Self {
        let mut c = Self {
            seconds_of_day,
            year,
            month: month.clamp(1, 12),
            day: day.clamp(1, 31),
            sun_alt_deg: 0.0,
            night: false,
            temperature_c: 15.0,
            follow_session: false,
        };
        c.update_sun();
        c
    }

    /// Seconds since midnight, wrapped to one day (`Time`).
    #[must_use]
    pub fn time_of_day_s(&self) -> f32 {
        self.seconds_of_day.rem_euclid(86_400.0) as f32
    }

    /// Day of the year, 1–366 (`DayOfYear`).
    #[must_use]
    pub fn day_of_year(&self) -> u32 {
        const START: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let leap = (self.year % 4 == 0 && self.year % 100 != 0) || self.year % 400 == 0;
        let m = (self.month.clamp(1, 12) - 1) as usize;
        START[m] + self.day + u32::from(leap && m >= 2)
    }

    /// `Envir_Brightness`: the ambient brightness `0..=1` that vehicle scripts and `[alphascale]`
    /// glass read. The stock scripts switch to night mode below 0.35 and back above 0.75; OPTS
    /// maps the sun's elevation from civil-twilight end (−6°, 0) to 15° (1) with a smooth step,
    /// so 0.35 falls at about 2° and 0.75 at about 8°. The mapping is a guess ❓
    /// (`docs/src/formats/text/vehicles/predefined-variables.md`).
    #[must_use]
    pub fn envir_brightness(&self) -> f32 {
        let t = ((self.sun_alt_deg + 6.0) / 21.0).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Moves time on by `dt_s` and recomputes the sun and the night switch.
    pub fn advance(&mut self, dt_s: f32) {
        self.seconds_of_day += f64::from(dt_s);
        self.update_sun();
    }

    /// Recomputes `sun_alt_deg` and `night` from the time and date.
    pub fn update_sun(&mut self) {
        self.sun_alt_deg = solar_altitude_deg(
            self.time_of_day_s() / 3600.0,
            self.day_of_year() as f32,
            LATITUDE_DEG,
        );
        self.night = self.sun_alt_deg < NIGHT_SUN_ALT_DEG;
    }
}

/// Sun elevation in degrees for a local solar hour, day of year and latitude (declination
/// approximation, good to about a degree).
#[must_use]
pub fn solar_altitude_deg(hour: f32, day_of_year: f32, latitude_deg: f32) -> f32 {
    let lat = latitude_deg.to_radians();
    let decl =
        (-23.44f32).to_radians() * (std::f32::consts::TAU * (day_of_year + 10.0) / 365.0).cos();
    let h = (15.0 * (hour - 12.0)).to_radians();
    let sin_el = lat.sin() * decl.sin() + lat.cos() * decl.cos() * h.cos();
    sin_el.clamp(-1.0, 1.0).asin().to_degrees()
}

/// A host value that objects without a script read by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostVar {
    /// `NightlightA`.
    Night,
    /// `TrafficLightPhase` of the object's light program.
    Phase,
    /// `Time` (seconds since midnight).
    Time,
    /// Anything else: reads 0.
    Zero,
}

impl HostVar {
    /// The host value behind a variable name (case-insensitive).
    #[must_use]
    pub fn of(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "nightlighta" => Self::Night,
            "trafficlightphase" => Self::Phase,
            "time" => Self::Time,
            _ => Self::Zero,
        }
    }
}

/// Where a rule reads its variable: the object's script engine (by fixed index) or the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarRef {
    /// Float variable index in the object's engine.
    Engine(u32),
    /// Host value (objects without a script).
    Host(HostVar),
    /// Always 1 (a light without a variable).
    One,
}

#[cfg(test)]
mod tests {
    #[test]
    fn brightness_follows_the_sun() {
        let b = |alt: f32| {
            let mut c = super::SceneryClock::at(2026, 6, 29, 43_200.0);
            c.sun_alt_deg = alt;
            c.envir_brightness()
        };
        assert_eq!(b(-20.0), 0.0);
        assert_eq!(b(40.0), 1.0);
        assert!(b(0.0) > 0.0 && b(0.0) < 0.35, "{}", b(0.0));
        assert!(b(9.0) > 0.75 && b(9.0) < 0.85, "{}", b(9.0));
    }

    use super::*;

    #[test]
    fn day_of_year_counts_leap_days() {
        assert_eq!(SceneryClock::at(2026, 1, 1, 0.0).day_of_year(), 1);
        assert_eq!(SceneryClock::at(2026, 6, 29, 0.0).day_of_year(), 180);
        assert_eq!(SceneryClock::at(2024, 3, 1, 0.0).day_of_year(), 61);
    }

    #[test]
    fn noon_is_day_and_late_evening_is_night() {
        let noon = SceneryClock::at(2026, 6, 29, 12.0 * 3600.0);
        assert!(noon.sun_alt_deg > 50.0 && !noon.night);
        let late = SceneryClock::at(2026, 6, 29, 23.0 * 3600.0);
        assert!(late.night);
        let mut c = noon.clone();
        c.advance(11.0 * 3600.0);
        assert!(c.night);
        assert!((c.time_of_day_s() - 23.0 * 3600.0).abs() < 1.0);
    }

    #[test]
    fn host_variables_are_matched_case_insensitively() {
        assert_eq!(HostVar::of("NightLightA"), HostVar::Night);
        assert_eq!(HostVar::of("TrafficLightPhase"), HostVar::Phase);
        assert_eq!(HostVar::of("red"), HostVar::Zero);
    }
}
