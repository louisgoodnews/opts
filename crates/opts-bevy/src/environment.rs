//! The session environment: date, time of day, weather and the values derived from them (sun,
//! night, light attenuation, temperature, day of the week). The weather is a preset id, resolved
//! to [`WeatherConditions`] through the [`WeatherCatalogue`] (`weather.rs`).
//!
//! [`SessionEnvironment`] is the one source of these values for a running session. It is written
//! only through [`EnvironmentChange`] messages (session start, the pause menu's *Environment*
//! page, `sim.set_time`, restoring a save) and by its own clock, which moves with
//! `Time<Virtual>` × [`SessionEnvironment::rate`]. Every jump (a change of date, time or weather)
//! bumps [`SessionEnvironment::revision`], so consumers that cache (the sky texture) know to
//! refresh. Design: `docs/src/architecture/session-environment.md`.
//!
//! A change of the date that the map content is built for (`map_date`, Chrono) is announced with
//! [`MapDateChanged`]; the game's Chrono handler (`omsi_world::apply_map_date_changes`) reports
//! its progress in [`ChronoRebuild`], which the pause menu observes.

use std::fmt;

use bevy::prelude::*;
use opts_core::formats::map::lists::day;
use serde::{Deserialize, Serialize};

use crate::moon::{moon_state, MoonState};
pub use crate::weather::WeatherKind;
use crate::weather::{
    effective_cover, light_scales_for_cover, WeatherCatalogue, WeatherConditions,
};
use opts_core::formats::weather::PrecipKind;

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
/// come from [`light_scales_for_cover`] (the rule of [`light_scales`]: 0.55 and 0.8 at full cover,
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

/// What happens to the map date (the date the map content, Chrono, is built for) when a change
/// sets the date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MapDatePolicy {
    /// The map date stays (a restored clock, a preview).
    #[default]
    Keep,
    /// The map date follows the new date; a different date sends [`MapDateChanged`] (the pause
    /// page's *Apply*, `sim.set_time` with a date).
    Follow,
    /// A new session: the map date is set to the date without a message (the map is loaded for
    /// it).
    Reset,
}

/// The only way to change [`SessionEnvironment`]: fields left `None` stay as they are.
#[derive(Message, Debug, Clone, Default, PartialEq)]
pub struct EnvironmentChange {
    /// New date.
    pub date: Option<CalendarDate>,
    /// New time of day in seconds since midnight (wrapped into one day).
    pub seconds_of_day: Option<f64>,
    /// New weather preset id.
    pub weather: Option<String>,
    /// New clock rate (simulated clock seconds per `Time<Virtual>` second; must be positive).
    pub rate: Option<f64>,
    /// What the date does to the map date.
    pub map_date: MapDatePolicy,
}

impl EnvironmentChange {
    /// The environment of a new session: date, time and weather, rate 1, map date = date.
    #[must_use]
    pub fn reset(date: CalendarDate, seconds_of_day: f64, weather: impl Into<String>) -> Self {
        Self {
            date: Some(date),
            seconds_of_day: Some(seconds_of_day),
            weather: Some(weather.into()),
            rate: Some(1.0),
            map_date: MapDatePolicy::Reset,
        }
    }
}

/// The map date changed (the content of the map may differ: Chrono). Sent by the apply system
/// for a change with [`MapDatePolicy::Follow`] whose date differs from the map date.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapDateChanged {
    /// The date the map was built for.
    pub old: CalendarDate,
    /// The new date.
    pub new: CalendarDate,
}

/// Progress of rebuilding the map for a new map date (Chrono).
///
/// The extension point of the live Chrono switch: a handler reads [`MapDateChanged`], sets
/// `total_tiles` and `pending_tiles` and counts `pending_tiles` down as tiles are rebuilt (the
/// game's `omsi_world::apply_map_date_changes`). The pause page shows the progress and *Resume*
/// waits until `pending_tiles` is zero.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct ChronoRebuild {
    /// Tiles still to rebuild.
    pub pending_tiles: u32,
    /// Tiles of the running rebuild.
    pub total_tiles: u32,
}

impl ChronoRebuild {
    /// Whether a rebuild is running.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.pending_tiles > 0
    }
}

/// Date, time and weather of the running session, with the derived values.
///
/// Read it anywhere; change it only with [`EnvironmentChange`] (its fields are private for that
/// reason).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct SessionEnvironment {
    date: CalendarDate,
    seconds_of_day: f64,
    weather: String,
    rate: f64,
    revision: u64,
    map_date: CalendarDate,
    // Derived.
    conditions: WeatherConditions,
    kind: WeatherKind,
    sun_alt_deg: f32,
    sun_az_deg: f32,
    moon: MoonState,
}

impl Default for SessionEnvironment {
    fn default() -> Self {
        Self::new(CalendarDate::default(), 12.0 * 3600.0, "")
    }
}

impl SessionEnvironment {
    /// An environment at the given date, time and weather (rate 1, revision 0, map date = date).
    #[must_use]
    pub fn new(date: CalendarDate, seconds_of_day: f64, weather: &str) -> Self {
        let mut env = Self {
            date,
            seconds_of_day: 0.0,
            weather: weather.to_owned(),
            rate: 1.0,
            revision: 0,
            map_date: date,
            conditions: WeatherConditions::default(),
            kind: WeatherKind::Clear,
            sun_alt_deg: 0.0,
            sun_az_deg: 0.0,
            moon: moon_state(date, 0.0, LATITUDE_DEG),
        };
        env.set_seconds(seconds_of_day);
        env.derive_sun();
        env.resolve_weather(WeatherCatalogue::builtin());
        env
    }

    /// The current date.
    #[must_use]
    pub fn date(&self) -> CalendarDate {
        self.date
    }

    /// Seconds since midnight, `0..86400`.
    #[must_use]
    pub fn seconds_of_day(&self) -> f64 {
        self.seconds_of_day
    }

    /// Hour of the day as a fraction, `0..24`.
    #[must_use]
    pub fn hour(&self) -> f32 {
        (self.seconds_of_day / 3600.0) as f32
    }

    /// `(hour, minute, second)` of the clock.
    #[must_use]
    pub fn hms(&self) -> (u32, u32, u32) {
        let s = self.seconds_of_day as u32;
        (s / 3600 % 24, s / 60 % 60, s % 60)
    }

    /// `HH:MM:SS`.
    #[must_use]
    pub fn clock_text(&self) -> String {
        let (h, m, s) = self.hms();
        format!("{h:02}:{m:02}:{s:02}")
    }

    /// The weather preset id.
    #[must_use]
    pub fn weather(&self) -> &str {
        &self.weather
    }

    /// What the weather does, classified (fallback light, icons).
    #[must_use]
    pub fn weather_kind(&self) -> WeatherKind {
        self.kind
    }

    /// The resolved weather: fog, wind, clouds, precipitation, wetness.
    #[must_use]
    pub fn conditions(&self) -> &WeatherConditions {
        &self.conditions
    }

    /// Clock seconds per `Time<Virtual>` second.
    #[must_use]
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Counts the jumps (changes of date, time or weather); the clock running does not count.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The date the map content is built for (Chrono).
    #[must_use]
    pub fn map_date(&self) -> CalendarDate {
        self.map_date
    }

    /// ISO day of the week, 1 = Monday … 7 = Sunday.
    #[must_use]
    pub fn weekday(&self) -> u32 {
        self.date.weekday()
    }

    /// The `[set_day_of_week]` bit of today.
    #[must_use]
    pub fn day_bit(&self) -> u8 {
        self.date.day_bit()
    }

    /// Sun elevation in degrees.
    #[must_use]
    pub fn sun_alt_deg(&self) -> f32 {
        self.sun_alt_deg
    }

    /// Sun azimuth in degrees, clockwise from north.
    #[must_use]
    pub fn sun_az_deg(&self) -> f32 {
        self.sun_az_deg
    }

    /// Whether it is night (sun below [`NIGHT_SUN_ALT_DEG`]).
    #[must_use]
    pub fn night(&self) -> bool {
        self.sun_alt_deg < NIGHT_SUN_ALT_DEG
    }

    /// The moon (position, phase, bright limb) for the session's date and time, refreshed with
    /// the sun.
    #[must_use]
    pub fn moon(&self) -> &MoonState {
        &self.moon
    }

    /// What the weather does to every light ([`light_attenuation`]).
    #[must_use]
    pub fn light_attenuation(&self) -> LightAttenuation {
        light_attenuation(&WeatherSky::from_conditions(&self.conditions))
    }

    /// Multiplier for the sun's illuminance under the weather: the `direct` part of
    /// [`Self::light_attenuation`].
    #[must_use]
    pub fn attenuation(&self) -> f32 {
        self.light_attenuation().direct
    }

    /// Multiplier for the ambient light under the weather ([`light_scales`]).
    #[must_use]
    pub fn ambient_attenuation(&self) -> f32 {
        self.light_attenuation().ambient
    }

    /// Outside temperature in °C.
    #[must_use]
    pub fn temperature_c(&self) -> f32 {
        self.conditions.temperature_c
    }

    /// Moves the clock on by `dt_s` clock seconds, rolling the date over at midnight. Not a jump:
    /// the revision stays.
    pub fn advance(&mut self, dt_s: f64) {
        if dt_s <= 0.0 || !dt_s.is_finite() {
            return;
        }
        let total = self.seconds_of_day + dt_s;
        let days = (total / DAY_S).floor();
        if days >= 1.0 {
            self.date = self.date.add_days(days as i64);
        }
        self.set_seconds(total);
        self.derive_sun();
    }

    /// Applies a change with the built-in presets only; see [`Self::apply_with`].
    pub fn apply(&mut self, change: &EnvironmentChange) -> Option<MapDateChanged> {
        self.apply_with(change, WeatherCatalogue::builtin())
    }

    /// Applies a change, resolving a new weather id through `catalogue`; returns the map date
    /// change it causes, if any.
    pub fn apply_with(
        &mut self,
        change: &EnvironmentChange,
        catalogue: &WeatherCatalogue,
    ) -> Option<MapDateChanged> {
        let mut jumped = false;
        let mut announced = None;
        if let Some(date) = change.date {
            let date = CalendarDate::clamped(date.year, date.month, date.day);
            jumped |= date != self.date;
            self.date = date;
            match change.map_date {
                MapDatePolicy::Keep => {}
                MapDatePolicy::Follow => {
                    if date != self.map_date {
                        announced = Some(MapDateChanged {
                            old: self.map_date,
                            new: date,
                        });
                        self.map_date = date;
                    }
                }
                MapDatePolicy::Reset => self.map_date = date,
            }
        }
        if let Some(s) = change.seconds_of_day.filter(|s| s.is_finite()) {
            let before = self.seconds_of_day;
            self.set_seconds(s);
            jumped |= (self.seconds_of_day - before).abs() > f64::EPSILON;
        }
        if let Some(w) = &change.weather {
            jumped |= *w != self.weather;
            w.clone_into(&mut self.weather);
        }
        if let Some(r) = change.rate.filter(|r| r.is_finite() && *r > 0.0) {
            self.rate = r;
        }
        if jumped || change.map_date == MapDatePolicy::Reset {
            self.revision += 1;
        }
        self.derive_sun();
        self.resolve_weather(catalogue);
        announced
    }

    /// Resolves the weather again after the catalogue changed (the install became known). A
    /// different outcome is a jump, so the sky and the effects refresh.
    pub fn refresh_weather(&mut self, catalogue: &WeatherCatalogue) {
        let before = self.conditions.clone();
        self.resolve_weather(catalogue);
        if self.conditions != before {
            self.revision += 1;
        }
    }

    fn set_seconds(&mut self, s: f64) {
        self.seconds_of_day = s.rem_euclid(DAY_S);
        // `rem_euclid` can round up to exactly one day.
        if self.seconds_of_day >= DAY_S {
            self.seconds_of_day = 0.0;
        }
    }

    fn resolve_weather(&mut self, catalogue: &WeatherCatalogue) {
        self.conditions = catalogue.resolve(&self.weather);
        self.kind = self.conditions.kind();
    }

    fn derive_sun(&mut self) {
        let (alt, az) = sun_position(self.hour(), self.date.day_of_year() as f32, LATITUDE_DEG);
        self.sun_alt_deg = alt;
        self.sun_az_deg = az;
        self.moon = moon_state(self.date, self.seconds_of_day, LATITUDE_DEG);
    }
}

/// The environment systems: the clock, then the changes. A host limits them to the session with
/// `configure_sets`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnvironmentSet;

/// Moves the clock with `Time<Virtual>` × rate (stands while the game is paused).
pub fn tick_environment(time: Res<Time<Virtual>>, mut env: ResMut<SessionEnvironment>) {
    let dt = time.delta_secs_f64() * env.rate();
    if dt > 0.0 {
        env.advance(dt);
    }
}

/// Applies the [`EnvironmentChange`] messages in order and announces map date changes.
pub fn apply_environment_changes(
    mut changes: MessageReader<EnvironmentChange>,
    mut env: ResMut<SessionEnvironment>,
    catalogue: Res<WeatherCatalogue>,
    mut map_dates: MessageWriter<MapDateChanged>,
) {
    for change in changes.read() {
        if let Some(changed) = env.apply_with(change, &catalogue) {
            tracing::info!(old = %changed.old, new = %changed.new, "map date changed");
            map_dates.write(changed);
        }
    }
}

/// Resolves the weather again when the preset catalogue changes (an install found after start).
pub fn refresh_weather_on_catalogue_change(
    catalogue: Res<WeatherCatalogue>,
    mut env: ResMut<SessionEnvironment>,
) {
    if catalogue.is_changed() && !catalogue.is_added() {
        env.refresh_weather(&catalogue);
    }
}

/// Registers the environment resource, its messages and systems (once per app).
#[derive(Default)]
pub struct EnvironmentPlugin;

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SessionEnvironment>()
            .init_resource::<WeatherCatalogue>()
            .init_resource::<crate::weather::WeatherEffects>()
            .init_resource::<ChronoRebuild>()
            .add_message::<EnvironmentChange>()
            .add_message::<MapDateChanged>()
            .add_systems(
                Update,
                (
                    tick_environment,
                    refresh_weather_on_catalogue_change,
                    apply_environment_changes,
                )
                    .chain()
                    .in_set(EnvironmentSet),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip_through_day_numbers() {
        for d in [
            CalendarDate::new(1970, 1, 1),
            CalendarDate::new(1994, 12, 31),
            CalendarDate::new(2000, 2, 29),
            CalendarDate::new(1900, 3, 1),
            CalendarDate::new(2026, 10, 17),
        ] {
            assert_eq!(CalendarDate::from_days_since_epoch(d.days_since_epoch()), d);
        }
        assert_eq!(CalendarDate::new(1970, 1, 1).days_since_epoch(), 0);
        assert_eq!(
            CalendarDate::new(1994, 12, 31).add_days(1),
            CalendarDate::new(1995, 1, 1)
        );
        assert_eq!(
            CalendarDate::new(2024, 2, 28).add_days(1),
            CalendarDate::new(2024, 2, 29)
        );
    }

    #[test]
    fn weekdays_and_day_bits() {
        // 2026-10-17 is a Saturday, 1989-10-09 a Monday, 1994-12-31 a Saturday.
        assert_eq!(CalendarDate::new(2026, 10, 17).weekday(), 6);
        assert_eq!(CalendarDate::new(1989, 10, 9).weekday(), 1);
        assert_eq!(CalendarDate::new(2026, 10, 18).day_bit(), day::SUNDAY);
        assert_eq!(CalendarDate::new(1994, 12, 31).day_bit(), day::SATURDAY);
        assert_eq!(CalendarDate::new(2026, 10, 14).day_bit(), day::WEEKDAY);
    }

    #[test]
    fn parsing_and_clamping() {
        assert_eq!(
            CalendarDate::parse("1994-12-31"),
            Some(CalendarDate::new(1994, 12, 31))
        );
        assert_eq!(CalendarDate::parse("1994-02-30"), None);
        assert_eq!(CalendarDate::parse("x"), None);
        assert_eq!(
            CalendarDate::clamped(1993, 2, 29),
            CalendarDate::new(1993, 2, 28)
        );
        assert_eq!(CalendarDate::new(1994, 3, 1).to_string(), "1994-03-01");
    }

    #[test]
    fn the_clock_wraps_and_rolls_the_date_over() {
        let mut env = SessionEnvironment::new(CalendarDate::new(1994, 12, 31), 23.5 * 3600.0, "");
        let rev = env.revision();
        env.advance(3600.0);
        assert_eq!(env.date(), CalendarDate::new(1995, 1, 1));
        assert!((env.seconds_of_day() - 1800.0).abs() < 1e-6);
        assert!((env.hour() - 0.5).abs() < 1e-6);
        assert_eq!(env.revision(), rev, "running is not a jump");
        assert_eq!(env.map_date(), CalendarDate::new(1994, 12, 31));
        assert_eq!(env.clock_text(), "00:30:00");
    }

    #[test]
    fn changes_bump_the_revision_and_follow_the_map_date() {
        let mut env = SessionEnvironment::new(CalendarDate::new(1990, 6, 1), 43_200.0, "Clear");
        let r0 = env.revision();
        assert_eq!(
            env.apply(&EnvironmentChange {
                rate: Some(10.0),
                ..default()
            }),
            None
        );
        assert_eq!(env.revision(), r0, "a rate is not a jump");
        assert_eq!(env.rate(), 10.0);
        env.apply(&EnvironmentChange {
            seconds_of_day: Some(90_000.0),
            weather: Some("Winter Snowfall".into()),
            ..default()
        });
        assert_eq!(env.revision(), r0 + 1);
        assert!((env.seconds_of_day() - 3600.0).abs() < 1e-6);
        assert_eq!(env.weather_kind(), WeatherKind::Snow);
        assert_eq!(env.temperature_c(), -3.0);
        assert!(env.attenuation() < 1.0);
        assert!(env.night());
        // A previewed date keeps the map date; Follow moves it and announces it once.
        env.apply(&EnvironmentChange {
            date: Some(CalendarDate::new(1991, 1, 1)),
            ..default()
        });
        assert_eq!(env.map_date(), CalendarDate::new(1990, 6, 1));
        let ann = env.apply(&EnvironmentChange {
            date: Some(CalendarDate::new(1992, 2, 2)),
            map_date: MapDatePolicy::Follow,
            ..default()
        });
        assert_eq!(
            ann,
            Some(MapDateChanged {
                old: CalendarDate::new(1990, 6, 1),
                new: CalendarDate::new(1992, 2, 2)
            })
        );
        assert_eq!(
            env.apply(&EnvironmentChange {
                date: Some(CalendarDate::new(1992, 2, 2)),
                map_date: MapDatePolicy::Follow,
                ..default()
            }),
            None
        );
        let reset = EnvironmentChange::reset(CalendarDate::new(2000, 1, 1), 0.0, "x");
        assert_eq!(env.apply(&reset), None);
        assert_eq!(env.map_date(), CalendarDate::new(2000, 1, 1));
        assert_eq!(env.rate(), 1.0);
    }

    #[test]
    fn sun_and_night_follow_the_clock() {
        let noon = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 12.0 * 3600.0, "");
        assert!(noon.sun_alt_deg() > 50.0 && !noon.night());
        assert!((noon.sun_az_deg() - 180.0).abs() < 1.0);
        let late = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 23.0 * 3600.0, "");
        assert!(late.night());
    }

    #[test]
    fn traffic_density_reads_the_wrapped_hour_and_the_weekend() {
        use opts_core::formats::map::lists::{DensityProfile, GroupDensity};
        let group = GroupDensity {
            name: "cars".into(),
            factor: 1.0,
            profiles: vec![
                DensityProfile {
                    days: day::WEEKDAY,
                    points: vec![(0.0, 0.1), (8.0, 1.0), (24.0, 0.1)],
                },
                DensityProfile {
                    days: day::SATURDAY | day::SUNDAY,
                    points: vec![(0.0, 0.05), (24.0, 0.05)],
                },
            ],
        };
        // Friday 23:00 + 9 h = Saturday 08:00: the hour wraps (8, not 32) and the weekend curve
        // applies.
        let mut env = SessionEnvironment::new(CalendarDate::new(2026, 10, 16), 23.0 * 3600.0, "");
        assert_eq!(env.day_bit(), day::WEEKDAY);
        env.advance(9.0 * 3600.0);
        assert!((env.hour() - 8.0).abs() < 1e-4);
        assert_eq!(env.day_bit(), day::SATURDAY);
        assert!((group.density(env.day_bit(), env.hour()) - 0.05).abs() < 1e-6);
        let monday = SessionEnvironment::new(CalendarDate::new(2026, 10, 19), 8.0 * 3600.0, "");
        assert!((group.density(monday.day_bit(), monday.hour()) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn weather_kinds_from_preset_words() {
        assert_eq!(
            WeatherKind::of_preset("Clear Summer Day"),
            WeatherKind::Clear
        );
        assert_eq!(
            WeatherKind::of_preset("Overcast Autumn"),
            WeatherKind::Overcast
        );
        assert_eq!(WeatherKind::of_preset("Rainy Afternoon"), WeatherKind::Rain);
        assert_eq!(
            WeatherKind::of_preset("Autumn Morning Fog"),
            WeatherKind::Fog
        );
        assert_eq!(WeatherKind::of_preset("Winter Snowfall"), WeatherKind::Snow);
    }

    #[test]
    fn the_systems_tick_with_virtual_time_and_apply_changes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(EnvironmentPlugin);
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(500),
        ));
        app.world_mut().write_message(EnvironmentChange {
            rate: Some(4.0),
            ..EnvironmentChange::reset(CalendarDate::new(1994, 12, 31), 100.0, "Clear")
        });
        for _ in 0..3 {
            app.update();
        }
        let env = app.world().resource::<SessionEnvironment>();
        assert_eq!(env.rate(), 4.0);
        // The first frames have no delta; the clock moved by 0.5 s × 4 per frame after that.
        assert!(env.seconds_of_day() > 100.0, "{}", env.seconds_of_day());
        app.world_mut().write_message(EnvironmentChange {
            date: Some(CalendarDate::new(1995, 1, 2)),
            map_date: MapDatePolicy::Follow,
            ..default()
        });
        app.update();
        let msgs = app.world().resource::<Messages<MapDateChanged>>();
        let mut cursor = msgs.get_cursor();
        let got: Vec<_> = cursor.read(msgs).copied().collect();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].new, CalendarDate::new(1995, 1, 2));
    }

    #[test]
    fn a_weather_change_resolves_through_the_catalogue_and_is_a_jump() {
        use opts_core::formats::weather::{PresetFile, WeatherFile, WeatherFolder};
        let storm = WeatherFile {
            fog: opts_core::formats::weather::Fog {
                visibility_m: 700.0,
                density: 0.5,
            },
            temp: opts_core::formats::weather::Temperature {
                celsius: 9.0,
                dew_point_c: 9.0,
            },
            ..WeatherFile::default()
        };
        let catalogue = WeatherCatalogue::from_folder(
            "/omsi".into(),
            WeatherFolder {
                presets: vec![PresetFile {
                    id: "Sturm".into(),
                    weather: storm,
                    texts: Default::default(),
                }],
                ..default()
            },
        );
        let mut env = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 43_200.0, "Sturm");
        // Without the install the id is unknown: clear weather.
        assert_eq!(env.conditions().fog_visibility_m, 50_000.0);
        let r0 = env.revision();
        env.refresh_weather(&catalogue);
        assert_eq!(env.conditions().fog_visibility_m, 700.0);
        assert_eq!(env.temperature_c(), 9.0);
        assert_eq!(env.revision(), r0 + 1, "a different outcome is a jump");
        env.refresh_weather(&catalogue);
        assert_eq!(env.revision(), r0 + 1, "the same outcome is not");
        env.apply_with(
            &EnvironmentChange {
                weather: Some("Rainy Afternoon".into()),
                ..default()
            },
            &catalogue,
        );
        assert_eq!(env.conditions().kind(), WeatherKind::Rain);
        assert_eq!(
            env.weather(),
            "Rainy Afternoon",
            "the stored id is kept as given"
        );
    }

    #[test]
    fn the_plugin_resolves_weather_changes_and_catalogue_changes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(EnvironmentPlugin);
        app.world_mut().write_message(EnvironmentChange::reset(
            CalendarDate::new(2026, 6, 29),
            43_200.0,
            "Rainy Afternoon",
        ));
        app.update();
        let env = app.world().resource::<SessionEnvironment>();
        assert_eq!(
            env.conditions().precip,
            opts_core::formats::weather::PrecipKind::Rain
        );
        assert!(env.attenuation() < 0.6);
    }

    fn sky(cover: f32) -> WeatherSky {
        WeatherSky {
            cloud_cover: cover,
            ..WeatherSky::default()
        }
    }

    #[test]
    fn attenuation_is_clear_at_one_and_the_old_values_under_full_cover() {
        let clear = light_attenuation(&sky(0.0));
        assert_eq!(
            (clear.direct, clear.ambient, clear.moon, clear.shafts),
            (1.0, 1.0, 1.0, 1.0)
        );
        let over = light_attenuation(&sky(1.0));
        assert!((over.direct - 0.55).abs() < 1e-6);
        assert!((over.ambient - 0.8).abs() < 1e-6);
        assert!((over.moon - 0.35).abs() < 1e-6);
        assert_eq!(over.shafts, 0.0, "a solid overcast has no beam");
    }

    #[test]
    fn attenuation_is_monotone_in_cloud_cover() {
        let mut prev = light_attenuation(&sky(0.0));
        for i in 1..=20 {
            let a = light_attenuation(&sky(i as f32 / 20.0));
            assert!(a.direct <= prev.direct && a.ambient <= prev.ambient);
            assert!(a.moon <= prev.moon && a.shafts <= prev.shafts);
            assert!(a.moon < a.direct, "cloud blocks the weaker light more");
            prev = a;
        }
    }

    #[test]
    fn the_environment_wrappers_agree_with_the_function() {
        let env =
            SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 43_200.0, "Rainy Afternoon");
        let a = env.light_attenuation();
        assert_eq!(env.attenuation(), a.direct);
        assert_eq!(env.ambient_attenuation(), a.ambient);
        assert_eq!(
            a,
            light_attenuation(&WeatherSky::from_conditions(env.conditions()))
        );
        assert!(a.shafts < 0.2 && a.moon < 0.6);
    }

    #[test]
    fn the_moon_follows_the_clock() {
        let mut env = SessionEnvironment::new(CalendarDate::new(2026, 10, 17), 18.0 * 3600.0, "");
        let before = *env.moon();
        env.advance(4.0 * 3600.0);
        assert_ne!(before.elevation_deg, env.moon().elevation_deg);
        assert!((before.illuminated - env.moon().illuminated).abs() < 0.02);
    }
}
