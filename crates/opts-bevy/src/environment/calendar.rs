//! Proleptic Gregorian calendar date, solar position, and weather light attenuation.

use super::*;

/// Latitude of the built-in sun formula. OMSI maps carry no geographic position, so the sun path
/// assumes central Europe (where OMSI's maps lie) and treats clock time as local solar time.
pub const LATITUDE_DEG: f32 = 50.0;

/// Sun elevation below which it is night: AI and player vehicle lights, the instruments' night
/// palette, `NightlightA` (3° above the horizon).
pub const NIGHT_SUN_ALT_DEG: f32 = 3.0;

/// Seconds of one day.
pub const DAY_S: f64 = 86_400.0;

/// A calendar date (proleptic Gregorian).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CalendarDate {
    /// Year.
    pub year: i32,
    /// Month, 1–12.
    pub month: u32,
    /// Day of the month, 1–31.
    pub day: u32,
}

impl Default for CalendarDate {
    fn default() -> Self {
        Self::new(2026, 6, 29)
    }
}

impl fmt::Display for CalendarDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl CalendarDate {
    /// A date from its parts, as given (see [`Self::clamped`]).
    #[must_use]
    pub const fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// A valid date: the month brought into 1–12 and the day into the month (31 February
    /// becomes the last day of February).
    #[must_use]
    pub fn clamped(year: i32, month: u32, day: u32) -> Self {
        let month = month.clamp(1, 12);
        Self {
            year,
            month,
            day: day.clamp(1, days_in_month(year, month)),
        }
    }

    /// Whether the parts form a real date.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        (1..=12).contains(&self.month)
            && (1..=days_in_month(self.year, self.month)).contains(&self.day)
    }

    /// Parses `YYYY-MM-DD`; `None` for anything else or an impossible date.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.trim().splitn(3, '-');
        let year = parts.next()?.parse().ok()?;
        let month = parts.next()?.parse().ok()?;
        let day = parts.next()?.parse().ok()?;
        let d = Self::new(year, month, day);
        d.is_valid().then_some(d)
    }

    /// Day of the year, 1–366.
    #[must_use]
    pub fn day_of_year(&self) -> u32 {
        const START: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let m = (self.month.clamp(1, 12) - 1) as usize;
        START[m] + self.day + u32::from(is_leap_year(self.year) && m >= 2)
    }

    /// Days since 1970-01-01 (negative before).
    #[must_use]
    pub fn days_since_epoch(&self) -> i64 {
        // Howard Hinnant's `days_from_civil`.
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` after 1970-01-01.
    #[must_use]
    pub fn from_days_since_epoch(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        Self { year, month, day }
    }

    /// The date `days` later (earlier when negative).
    #[must_use]
    pub fn add_days(&self, days: i64) -> Self {
        Self::from_days_since_epoch(self.days_since_epoch() + days)
    }

    /// ISO day of the week: 1 = Monday … 7 = Sunday.
    #[must_use]
    pub fn weekday(&self) -> u32 {
        // 1970-01-01 was a Thursday (4).
        ((self.days_since_epoch() + 3).rem_euclid(7) + 1) as u32
    }

    /// The `[set_day_of_week]` bit of the day (Monday–Friday, Saturday or Sunday; public
    /// holidays are not known ❓).
    #[must_use]
    pub fn day_bit(&self) -> u8 {
        match self.weekday() {
            6 => day::SATURDAY,
            7 => day::SUNDAY,
            _ => day::WEEKDAY,
        }
    }
}

/// Whether `year` is a leap year.
#[must_use]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days of a month (0 for a month outside 1–12).
#[must_use]
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

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

/// The sky as far as lighting is concerned: the one input of [`light_attenuation`].
///
/// The weather side fills it ([`WeatherSky::from_conditions`]); the lighting code only reads
/// [`LightAttenuation`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherSky {
    /// Share of the sky that blocks the direct light, `0..=1` (clouds, precipitation, fog).
    pub cloud_cover: f32,
    /// Meteorological visibility in metres.
    pub visibility_m: f32,
    /// What falls.
    pub precip: PrecipKind,
    /// Ground wetness `0..=1` (reserved: no rule reads it yet).
    pub wet: f32,
}

impl Default for WeatherSky {
    fn default() -> Self {
        Self {
            cloud_cover: 0.0,
            visibility_m: crate::weather::FOG_MAX_VISIBILITY_M,
            precip: PrecipKind::None,
            wet: 0.0,
        }
    }
}

impl WeatherSky {
    /// The sky of resolved weather conditions.
    #[must_use]
    pub fn from_conditions(c: &WeatherConditions) -> Self {
        Self {
            cloud_cover: effective_cover(c),
            visibility_m: c.fog_visibility_m,
            precip: c.precip,
            wet: c.ground_wet.into_iter().fold(0.0, f32::max).clamp(0.0, 1.0),
        }
    }
}

/// What the sky does to each light, `0..=1` (1 = clear).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightAttenuation {
    /// The direct sunlight.
    pub direct: f32,
    /// The ambient light.
    pub ambient: f32,
    /// The moonlight (cloud blocks the weaker light more).
    pub moon: f32,
    /// Light shafts: they need gaps in the cloud, so a solid overcast has none.
    pub shafts: f32,
}

/// **The one attenuation function** (design `lighting-system.md` 5.6). `direct` and `ambient`
/// come from [`light_scales_for_cover`] (the rule of [`crate::weather::light_scales`]: 0.55 and 0.8 at full cover,
/// the values that were in use before); `moon = 1 − 0.65·cover`; `shafts = direct² × (1 −
/// cover)`, which is 0 under a solid cover.
#[must_use]
pub fn light_attenuation(w: &WeatherSky) -> LightAttenuation {
    let cover = w.cloud_cover.clamp(0.0, 1.0);
    let scales = light_scales_for_cover(cover);
    LightAttenuation {
        direct: scales.sun,
        ambient: scales.ambient,
        moon: 1.0 - 0.65 * cover,
        shafts: scales.sun * scales.sun * (1.0 - cover),
    }
}
