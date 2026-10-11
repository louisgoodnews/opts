//! Traffic-light programs and what they mean for vehicles.
//!
//! A junction object carries light programs: a repeating sequence of `[phase] <state> <seconds>`
//! (`docs/src/formats/text/sco.md`). The state numbers are those of OMSI's own signal-head script
//! (`Scripts/…/ampel1.osc`, "Traffic Light for Cars"): `0–2` red, `3–5` red + yellow, `6–8` green,
//! `9–11` yellow, anything else dark. A path names the light that governs it
//! (`[use_traffic_light]`, the next path only); the vehicle stops at the **start** of that path.
//! Which instant of the cycle is "now" is not documented: the cycle starts at midnight of the
//! session clock ❓ (all lights of the map then agree with each other, which is what matters).

/// Identifies a light program: the junction object's `id_code` and the program index
/// (`[use_traffic_light]`, numbered in declaration order across the object's groups).
pub type SignalKey = (u64, usize);

/// What a driver reads from a light.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    /// Red, or red + yellow: stop.
    Stop,
    /// Green: go.
    Go,
    /// Yellow: stop if that is comfortably possible.
    Yellow,
    /// Dark: the light does not regulate (right of way applies).
    Off,
}

impl Aspect {
    /// The aspect of a phase state number.
    #[must_use]
    pub fn from_state(state: u32) -> Self {
        match state {
            0..=5 => Self::Stop,
            6..=8 => Self::Go,
            9..=11 => Self::Yellow,
            _ => Self::Off,
        }
    }
}

/// A repeating light program.
#[derive(Debug, Clone, PartialEq)]
pub struct SignalProgram {
    phases: Vec<(u32, f64)>,
    cycle: f64,
    /// How far before the stop line vehicles check the light (m); `0` = unspecified.
    pub approach: f64,
}

impl SignalProgram {
    /// Builds a program from `(state, seconds)` phases; `None` when the cycle is not positive.
    #[must_use]
    pub fn new(phases: Vec<(u32, f64)>, approach: f64) -> Option<Self> {
        let phases: Vec<_> = phases.into_iter().filter(|p| p.1 > 0.0).collect();
        let cycle: f64 = phases.iter().map(|p| p.1).sum();
        (cycle > 0.0).then_some(Self {
            phases,
            cycle,
            approach,
        })
    }

    /// Length of one cycle in seconds.
    #[must_use]
    pub fn cycle(&self) -> f64 {
        self.cycle
    }

    /// The phase state at clock time `t` seconds.
    #[must_use]
    pub fn state_at(&self, t: f64) -> u32 {
        let mut x = t.rem_euclid(self.cycle);
        for &(state, dur) in &self.phases {
            if x < dur {
                return state;
            }
            x -= dur;
        }
        self.phases.last().map_or(12, |p| p.0)
    }

    /// The aspect at clock time `t`.
    #[must_use]
    pub fn aspect_at(&self, t: f64) -> Aspect {
        Aspect::from_state(self.state_at(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_ranges_follow_the_signal_head_script() {
        let a = Aspect::from_state;
        assert_eq!([a(0), a(2), a(3), a(5)], [Aspect::Stop; 4]);
        assert_eq!([a(6), a(8)], [Aspect::Go; 2]);
        assert_eq!([a(9), a(11)], [Aspect::Yellow; 2]);
        assert_eq!([a(12), a(40)], [Aspect::Off; 2]);
    }

    #[test]
    fn the_program_repeats_and_wraps() {
        // The Gladbeck program: green 2+30, yellow 5, red 33, red-yellow 2, green 6+4 = 82 s.
        let p = SignalProgram::new(
            vec![
                (6, 2.0),
                (6, 30.0),
                (9, 5.0),
                (0, 33.0),
                (3, 2.0),
                (6, 6.0),
                (6, 4.0),
            ],
            0.0,
        )
        .unwrap();
        assert_eq!(p.cycle(), 82.0);
        assert_eq!(p.aspect_at(0.0), Aspect::Go);
        assert_eq!(p.aspect_at(31.9), Aspect::Go);
        assert_eq!(p.aspect_at(32.5), Aspect::Yellow);
        assert_eq!(p.aspect_at(40.0), Aspect::Stop);
        assert_eq!(p.aspect_at(71.0), Aspect::Stop); // red + yellow
        assert_eq!(p.aspect_at(73.0), Aspect::Go);
        // Green runs through the end of the cycle into the start of the next.
        assert_eq!(p.aspect_at(81.9), Aspect::Go);
        assert_eq!(p.aspect_at(82.0 * 5.0 + 1.0), Aspect::Go);
        assert_eq!(p.aspect_at(-1.0), Aspect::Go);
    }

    #[test]
    fn empty_or_zero_length_programs_are_rejected() {
        assert!(SignalProgram::new(Vec::new(), 0.0).is_none());
        assert!(SignalProgram::new(vec![(6, 0.0)], 0.0).is_none());
    }
}
