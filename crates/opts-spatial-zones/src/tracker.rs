//! Hysteresis for zone membership. A transition needs a run of consecutive samples on the other
//! side of the boundary that covers a minimum distance, so a bridge canopy, a gap in the roof or
//! a single noisy ray does not start or end a zone.

/// The membership change reported by [`ZoneTracker::update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Entered,
    Left,
}

/// Per-subject membership with enter and leave distances (metres travelled under the new state).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneTracker {
    inside: bool,
    run_m: f32,
    enter_m: f32,
    leave_m: f32,
}

impl ZoneTracker {
    /// A tracker outside the zone. `enter_m` is the covered distance needed to enter (15 m in
    /// M-0115), `leave_m` the uncovered distance needed to leave.
    #[must_use]
    pub fn new(enter_m: f32, leave_m: f32) -> Self {
        Self {
            inside: false,
            run_m: 0.0,
            enter_m,
            leave_m,
        }
    }

    /// Whether the subject is currently inside.
    #[must_use]
    pub fn inside(&self) -> bool {
        self.inside
    }

    /// Feeds one sample: `covered` is the coverage at the subject, `advance_m` the distance
    /// travelled since the previous sample (negative values count as zero). Returns the
    /// transition when this sample completes one.
    pub fn update(&mut self, covered: bool, advance_m: f32) -> Option<Transition> {
        if covered == self.inside {
            self.run_m = 0.0;
            return None;
        }
        self.run_m += advance_m.max(0.0);
        let needed = if covered { self.enter_m } else { self.leave_m };
        if self.run_m < needed {
            return None;
        }
        self.inside = covered;
        self.run_m = 0.0;
        Some(if covered {
            Transition::Entered
        } else {
            Transition::Left
        })
    }
}

/// Decides when a 10 Hz sample is due, from the frame delta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleGate {
    period_s: f32,
    elapsed_s: f32,
}

impl SampleGate {
    /// The sample rate of M-0115: at most 10 samples per second.
    pub const RATE_HZ: f32 = 10.0;

    /// A gate that is due on its first call.
    #[must_use]
    pub fn new() -> Self {
        Self {
            period_s: 1.0 / Self::RATE_HZ,
            elapsed_s: f32::INFINITY,
        }
    }

    /// Adds `dt_s` seconds. Returns `true` once per period; a long frame yields one sample, not a
    /// burst, so the backlog is dropped.
    pub fn tick(&mut self, dt_s: f32) -> bool {
        self.elapsed_s += dt_s.max(0.0);
        if self.elapsed_s < self.period_s {
            return false;
        }
        self.elapsed_s = 0.0;
        true
    }
}

impl Default for SampleGate {
    fn default() -> Self {
        Self::new()
    }
}
