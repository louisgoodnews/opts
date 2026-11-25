//! Gain curves of the sound engine: pure functions, tested without a sound card.

/// How a spatial source fades with distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Distance {
    /// Inside this distance the source plays at full level (m).
    pub reference: f32,
    /// Beyond this distance it is silent and costs no voice (m).
    pub max: f32,
    /// Steepness of the fall-off between the two (1 is the inverse distance law).
    pub rolloff: f32,
}

impl Default for Distance {
    fn default() -> Self {
        Self {
            reference: 5.0,
            max: 120.0,
            rolloff: 1.0,
        }
    }
}

impl Distance {
    /// A source heard from `reference` to `max` metres with the inverse distance law.
    #[must_use]
    pub fn new(reference: f32, max: f32) -> Self {
        Self {
            reference: reference.max(0.01),
            max: max.max(reference.max(0.01) + 0.01),
            rolloff: 1.0,
        }
    }

    /// The gain (0 to 1) at `distance` metres: 1 inside the reference distance, then
    /// `reference / (reference + rolloff × (d − reference))`, faded to exactly 0 over the last
    /// 15 % of the range so a source does not pop out at the edge.
    #[must_use]
    pub fn gain(&self, distance: f32) -> f32 {
        let d = distance.max(0.0);
        if d <= self.reference {
            return 1.0;
        }
        if d >= self.max {
            return 0.0;
        }
        let inverse =
            self.reference / (self.reference + self.rolloff.max(0.0) * (d - self.reference));
        let fade_start = self.max - 0.15 * (self.max - self.reference);
        let fade = if d <= fade_start {
            1.0
        } else {
            let x = ((self.max - d) / (self.max - fade_start)).clamp(0.0, 1.0);
            x * x * (3.0 - 2.0 * x)
        };
        (inverse * fade).clamp(0.0, 1.0)
    }
}

/// Decibels to a linear gain (`-6 dB` is about one half).
#[must_use]
pub fn db_to_gain(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Semitones to a playback speed factor (`+12` doubles it).
#[must_use]
pub fn semitones_to_speed(semitones: f32) -> f32 {
    2f32.powf(semitones / 12.0)
}

/// A percentage setting (0 to 100) to a linear gain with a perceptual curve: the square of the
/// fraction, so the middle of the slider is about a quarter of the power rather than half.
#[must_use]
pub fn percent_to_gain(percent: f32) -> f32 {
    let f = (percent / 100.0).clamp(0.0, 1.0);
    f * f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gain_is_one_inside_zero_outside_and_falls_in_between() {
        let d = Distance::new(5.0, 100.0);
        assert_eq!(d.gain(0.0), 1.0);
        assert_eq!(d.gain(5.0), 1.0);
        assert_eq!(d.gain(100.0), 0.0);
        assert_eq!(d.gain(500.0), 0.0);
        let near = d.gain(10.0);
        let far = d.gain(50.0);
        assert!(near < 1.0 && far < near && far > 0.0, "{near} {far}");
        // Inverse distance law between reference and the fade zone: 5 / 10 = 0.5.
        assert!((near - 0.5).abs() < 1e-5);
    }

    #[test]
    fn the_gain_never_rises_with_distance_and_ends_smoothly() {
        let d = Distance::new(3.0, 80.0);
        let mut last = 1.0;
        for i in 0..=800 {
            let g = d.gain(i as f32 * 0.1);
            assert!(
                g <= last + 1e-6,
                "rose at {} m: {g} after {last}",
                i as f32 * 0.1
            );
            last = g;
        }
        // No step at the edge: just inside the range the gain is already tiny.
        assert!(d.gain(79.9) < 0.01);
    }

    #[test]
    fn a_steeper_rolloff_is_quieter() {
        let mut a = Distance::new(5.0, 200.0);
        let mut b = a;
        a.rolloff = 1.0;
        b.rolloff = 2.0;
        assert!(b.gain(40.0) < a.gain(40.0));
    }

    #[test]
    fn unit_conversions() {
        assert!((db_to_gain(-6.0) - 0.501).abs() < 0.01);
        assert_eq!(db_to_gain(0.0), 1.0);
        assert!((semitones_to_speed(12.0) - 2.0).abs() < 1e-6);
        assert!((semitones_to_speed(-12.0) - 0.5).abs() < 1e-6);
        assert_eq!(percent_to_gain(100.0), 1.0);
        assert_eq!(percent_to_gain(0.0), 0.0);
        assert!((percent_to_gain(50.0) - 0.25).abs() < 1e-6);
        assert_eq!(percent_to_gain(250.0), 1.0);
    }
}
