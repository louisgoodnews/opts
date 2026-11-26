//! The run facts a challenge reads. The host maps its own events to these.

/// One fact about a run, in the order it happened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChallengeEvent {
    /// A scheduled stop was served: the vehicle stood at the stop point.
    StopServed {
        /// Station id, as the timetable names it.
        station: u32,
        /// Scheduled time of the stop, seconds of the day.
        scheduled_s: f64,
        /// Actual time the vehicle stopped, seconds of the day.
        actual_s: f64,
    },
    /// A scheduled stop was passed without stopping, so the stop was dropped.
    StopMissed {
        /// Station id, as the timetable names it.
        station: u32,
    },
    /// The vehicle collided with something.
    Collision,
    /// Passengers got on the vehicle.
    PassengersBoarded {
        /// Number of passengers that boarded in this step.
        count: u32,
    },
}
