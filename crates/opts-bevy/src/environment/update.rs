//! Advancing and changing the session environment.

use super::*;

impl SessionEnvironment {
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

    pub(super) fn set_seconds(&mut self, s: f64) {
        self.seconds_of_day = s.rem_euclid(DAY_S);
        // `rem_euclid` can round up to exactly one day.
        if self.seconds_of_day >= DAY_S {
            self.seconds_of_day = 0.0;
        }
    }

    pub(super) fn resolve_weather(&mut self, catalogue: &WeatherCatalogue) {
        self.conditions = catalogue.resolve(&self.weather);
        self.kind = self.conditions.kind();
    }

    pub(super) fn derive_sun(&mut self) {
        let (alt, az) = sun_position(self.hour(), self.date.day_of_year() as f32, LATITUDE_DEG);
        self.sun_alt_deg = alt;
        self.sun_az_deg = az;
        self.moon = moon_state(self.date, self.seconds_of_day, LATITUDE_DEG);
    }
}
