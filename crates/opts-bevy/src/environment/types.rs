//! The environment records: the map date policy, the change and the session environment.

use super::*;

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
    pub(super) date: CalendarDate,
    pub(super) seconds_of_day: f64,
    pub(super) weather: String,
    pub(super) rate: f64,
    pub(super) revision: u64,
    pub(super) map_date: CalendarDate,
    // Derived.
    pub(super) conditions: WeatherConditions,
    pub(super) kind: WeatherKind,
    pub(super) sun_alt_deg: f32,
    pub(super) sun_az_deg: f32,
    pub(super) moon: MoonState,
}

impl Default for SessionEnvironment {
    fn default() -> Self {
        Self::new(CalendarDate::default(), 12.0 * 3600.0, "")
    }
}
