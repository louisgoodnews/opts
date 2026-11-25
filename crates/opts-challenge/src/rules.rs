//! The scoring rules: the points each event is worth.
//!
//! The issue gives no values, so every number here is a placeholder. They are named constants
//! until the owner sets them (TODO(owner)).

/// Seconds a served stop may differ from its schedule and still count as on time.
pub const PUNCTUAL_WINDOW_S: f64 = 60.0; // TODO(owner): on-time window
/// The tighter window of the tight-schedule challenge, seconds.
pub const TIGHT_WINDOW_S: f64 = 30.0; // TODO(owner): tight-schedule window
/// Points for a stop served on time.
pub const POINTS_ON_TIME_STOP: i64 = 100; // TODO(owner)
/// Points for a stop served off schedule, before the lateness penalty.
pub const POINTS_OFF_TIME_STOP: i64 = 40; // TODO(owner)
/// Points taken per second a served stop is outside the window.
pub const POINTS_LOST_PER_OFF_S: i64 = 1; // TODO(owner)
/// Points (negative) for a stop that was missed.
pub const PENALTY_MISSED_STOP: i64 = -150; // TODO(owner)
/// Points (negative) for a collision.
pub const PENALTY_COLLISION: i64 = -250; // TODO(owner)
/// Points per passenger that boarded.
pub const POINTS_PER_PASSENGER: i64 = 2; // TODO(owner)
/// Share of the expected stops that must be served on time to pass.
pub const MIN_ON_TIME_RATIO: f64 = 0.8; // TODO(owner)
/// Most collisions a passing run may have.
pub const MAX_COLLISIONS: u32 = 0; // TODO(owner)
/// Most missed stops a passing run may have.
pub const MAX_MISSED_STOPS: u32 = 0; // TODO(owner)

/// The points table of one challenge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoringRules {
    /// Seconds a served stop may differ from its schedule and still count as on time.
    pub punctual_window_s: f64,
    /// Points for a stop served on time.
    pub on_time_points: i64,
    /// Points for a stop served off schedule, before the penalty.
    pub off_time_points: i64,
    /// Points taken per second outside the window.
    pub points_lost_per_off_s: i64,
    /// Points (negative) for a missed stop.
    pub missed_penalty: i64,
    /// Points (negative) for a collision.
    pub collision_penalty: i64,
    /// Points per boarded passenger.
    pub points_per_passenger: i64,
}

impl Default for ScoringRules {
    fn default() -> Self {
        Self {
            punctual_window_s: PUNCTUAL_WINDOW_S,
            on_time_points: POINTS_ON_TIME_STOP,
            off_time_points: POINTS_OFF_TIME_STOP,
            points_lost_per_off_s: POINTS_LOST_PER_OFF_S,
            missed_penalty: PENALTY_MISSED_STOP,
            collision_penalty: PENALTY_COLLISION,
            points_per_passenger: POINTS_PER_PASSENGER,
        }
    }
}
