//! Tuning constants of the traffic model: conflicts, signals, axles, lane changes and speed.

/// Conflict zones further ahead than this (m) are not considered.
pub(crate) const CONFLICT_LOOKAHEAD_M: f64 = 60.0;
/// Vehicles stop this far (m) before a conflict zone they have to wait for.
pub(crate) const CONFLICT_STOP_MARGIN_M: f64 = 1.5;
/// Speed (m/s) assumed for arrival-time estimates of slow or standing vehicles.
pub(crate) const MIN_ARRIVAL_SPEED: f64 = 2.0;
/// A vehicle without right of way waits for others that reach the zone within this time (s).
pub(crate) const GAP_ACCEPTANCE_S: f64 = 6.0;
/// A vehicle that has stood this long (s) at a conflict zone goes anyway …
pub(crate) const DEADLOCK_AFTER_S: f64 = 12.0;
/// A vehicle standing this long (s) with no red light ahead is stuck and is despawned.
pub const STUCK_AFTER_S: f64 = 60.0;
/// A released vehicle still gives way to one inside the zone that moves faster than this (m/s).
pub(crate) const RELEASE_RESPECT_SPEED: f64 = 0.5;
/// … and ignores conflicts for this long (s).
pub(crate) const DEADLOCK_IGNORE_S: f64 = 5.0;
/// How far along its planned lanes an agent looks for leaders, signals and zones (m).
pub(crate) const LOOKAHEAD_M: f64 = 100.0;
/// The plan covers at least this much road ahead (m).
pub(crate) const PLAN_HORIZON_M: f64 = 150.0;
/// Vehicles stop this far (m) before the start of a lane whose light is red.
pub(crate) const SIGNAL_STOP_MARGIN_M: f64 = 1.0;
/// Deceleration (m/s²) a driver accepts to stop for a yellow light.
pub(crate) const YELLOW_COMFORT_DECEL: f64 = 3.0;
/// Light-checking distance (m) when the program does not name one (`[approachdist]` `0`).
pub(crate) const DEFAULT_APPROACH_M: f64 = 60.0;

/// Slack (m) on the front axle's motion before the rear axle is placed afresh.
pub(crate) const AXLE_RESET_M: f64 = 0.3;
/// Distance (m) of the point on the path the front axle pursues.
pub(crate) const FRONT_PURSUIT_M: f64 = 1.0;
/// Largest travel (m) of one sub-step of the axle update.
pub(crate) const AXLE_SUBSTEP_M: f64 = 0.2;
/// Largest sideways speed (m/s) of a lane change's glide, for the axle jump check.
pub(crate) const MAX_GLIDE_SPEED: f64 = 3.0;

/// Distance (m) beyond the comfortable braking distance in which lower lane speeds are seen.
pub(crate) const SPEED_LOOK_EXTRA_M: f64 = 15.0;
/// A strategic change must be made this far (m) before the end of its lane …
pub(crate) const STRATEGIC_LAST_M: f64 = 15.0;
/// … where the vehicle otherwise stops this far (m) before the end and waits …
pub(crate) const STRATEGIC_STOP_M: f64 = 1.0;
/// … for at most this long (s) before it re-plans from its own lane.
pub(crate) const STRATEGIC_TIMEOUT_S: f64 = 10.0;
/// Lanes shorter than this (m) offer no strategic changes.
pub(crate) const STRATEGIC_MIN_LANE_M: f64 = 30.0;

/// Vehicles reach a lower lane speed this far (m) before the lane's start.
pub(crate) const SPEED_MARGIN_M: f64 = 2.0;
/// Time constant (s) of shedding excess speed on the current lane.
pub(crate) const SPEED_TAU_S: f64 = 1.0;

/// Indicators are requested when the next lane turns by more than this (radians) …
pub(crate) const BLINK_TURN: f64 = 20.0 * std::f64::consts::PI / 180.0;
/// … and the lane end is closer than this (metres).
pub(crate) const BLINK_DISTANCE: f64 = 30.0;
