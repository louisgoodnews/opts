//! Traffic light phases, programs and groups of a scenery object.

/// Traffic light phase program definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoPhase {
    /// Signal phase state code (e.g. 0 = red, 6 = green, 9 = yellow).
    pub state: u32,
    /// Phase duration in seconds.
    pub duration_sec: f32,
}

/// Named traffic light signal program.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoTrafficLightProgram {
    /// Program name (e.g. "Main", "RPB Haupt").
    pub name: String,
    /// Sequential phase timing entries (a repeating cycle).
    pub phases: Vec<ScoPhase>,
    /// `[approachdist]` of the program: how far before the stop line vehicles check the light
    /// (`0` = none; corpus: `0`, `25`, `30`, `50`, `100`). It follows the program's phases in the
    /// file, not a path.
    pub approach_distance_m: Option<f32>,
}

impl ScoTrafficLightProgram {
    /// Length of one cycle in seconds (sum of the phase durations).
    #[must_use]
    pub fn cycle_sec(&self) -> f32 {
        self.phases.iter().map(|p| p.duration_sec).sum()
    }
}

/// Traffic light controller group.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoTrafficLightGroup {
    /// Cycle time in seconds (e.g. 99.99).
    pub cycle_time_sec: f32,
    /// Signal programs belonging to this group.
    pub programs: Vec<ScoTrafficLightProgram>,
}

impl ScoTrafficLightGroup {
    /// The period every program of the group runs on, in seconds: the group's cycle time, or the
    /// longest program when one is longer (so no phase is cut off). All programs of one group
    /// share it; that shared clock is what keeps e.g. pedestrian and car greens apart.
    #[must_use]
    pub fn period_sec(&self) -> f32 {
        self.programs
            .iter()
            .map(|p| {
                p.phases
                    .iter()
                    .map(|ph| ph.duration_sec.max(0.0))
                    .sum::<f32>()
            })
            .fold(self.cycle_time_sec.max(0.0), f32::max)
    }
}
