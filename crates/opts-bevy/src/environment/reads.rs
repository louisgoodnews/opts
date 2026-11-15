//! The read side of the session environment: the date, the clock and the weather.

use super::*;

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
}
